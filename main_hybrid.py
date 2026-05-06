import jax
import jax.numpy as jnp
import numpy as np
import time
import logging
import asyncio
import json
import websockets
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
import uvicorn
from datetime import datetime
import MetaTrader5 as mt5

# --- IMPORT RUST SENTINEL ---
try:
    import quimeria_hyperion as sentinel
except ImportError:
    print("CRITICAL: Run 'maturin develop' first.")
    exit(1)

# --- IMPORT JAX BRAIN ---
from logic.adelic_koopman_ipda_synchronizer import AdelicKoopmanSynchronizer

# Configure Logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s [%(levelname)s] %(message)s')
logger = logging.getLogger("QUIMERIA_HYBRID")

# FastAPI App
app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

class ConnectionManager:
    def __init__(self):
        self.active_connections: list[WebSocket] = []

    async def connect(self, websocket: WebSocket):
        await websocket.accept()
        self.active_connections.append(websocket)

    def disconnect(self, websocket: WebSocket):
        self.active_connections.remove(websocket)

    async def broadcast(self, message: dict):
        for connection in self.active_connections:
            try:
                await connection.send_json(message)
            except Exception:
                pass

manager = ConnectionManager()

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await manager.connect(websocket)
    try:
        while True:
            await websocket.receive_text()
    except WebSocketDisconnect:
        manager.disconnect(websocket)

class GMOSOrchestrator:
    def __init__(self):
        self.brain = AdelicKoopmanSynchronizer()
        
        # Warm up JAX
        dummy_data = jnp.zeros((60,))
        _ = self.brain.compute_sync(dummy_data)
        logger.info("JAX Kernels Hot and Ready.")

        # Initialize MT5
        if not mt5.initialize():
            logger.error(f"MT5 initialization failed: {mt5.last_error()}")
        else:
            mt5.symbol_select("EURUSD", True)
            logger.info("MT5 Initialized Successfully for Paper Trading.")

        # Initialize the QUIMERIA Sentinel (Rust)
        logger.info("Initializing QUIMERIA-HYPERION Sentinel...")
        try:
            self.engine = sentinel.SentinelEngine("DEMO_KEY", "BTC/USDT", False, "redis://127.0.0.1/")
        except Exception as e:
            logger.error(f"Failed to init Sentinel: {e}")
            self.engine = sentinel.SentinelEngine()

        self.price_history = []
        self.is_active = True

    async def run_bitget_ws(self):
        url = "wss://ws.bitget.com/v2/ws/public"
        symbol = "BTCUSDT"
        
        while self.is_active:
            try:
                logger.info(f"Connecting to Bitget WebSocket: {url}")
                async with websockets.connect(url) as ws:
                    sub_msg = {
                        "op": "subscribe",
                        "args": [{
                            "instType": "SPOT",
                            "channel": "ticker",
                            "instId": symbol
                        }]
                    }
                    await ws.send(json.dumps(sub_msg))
                    
                    while self.is_active:
                        msg_text = await ws.recv()
                        if msg_text == "pong": continue
                        
                        data = json.loads(msg_text)
                        if "data" in data and len(data["data"]) > 0:
                            ticker = data["data"][0]
                            price = float(ticker["lastPr"])
                            
                            # Keep track of history for JAX
                            self.price_history.append(price)
                            if len(self.price_history) > 100:
                                self.price_history.pop(0)

                            if len(self.price_history) >= 20:
                                # Run Brain
                                price_tensor = jnp.array(self.price_history[-60:])
                                bias, stability, q_t_size = self.brain.compute_sync(price_tensor)
                                
                                signal = "FLAT"
                                if stability > 0.85:
                                    signal = "BUY" if bias > 0 else "SELL"
                                    # We can tell Rust to execute
                                    try:
                                        self.engine.execute_trade(signal, float(q_t_size))
                                    except:
                                        pass
                                    
                                    # Execute on MT5
                                    try:
                                        action = mt5.ORDER_TYPE_BUY if signal == "BUY" else mt5.ORDER_TYPE_SELL
                                        tick = mt5.symbol_info_tick("EURUSD")
                                        if tick:
                                            price = tick.ask if action == mt5.ORDER_TYPE_BUY else tick.bid
                                            request = {
                                                "action": mt5.TRADE_ACTION_DEAL,
                                                "symbol": "EURUSD",
                                                "volume": 0.01,
                                                "type": action,
                                                "price": price,
                                                "deviation": 20,
                                                "magic": 234000,
                                                "comment": "Hyperion Paper",
                                                "type_time": mt5.ORDER_TIME_GTC,
                                                "type_filling": mt5.ORDER_FILLING_IOC,
                                            }
                                            result = mt5.order_send(request)
                                            if result and result.retcode != mt5.TRADE_RETCODE_DONE:
                                                logger.warning(f"MT5 Order Failed: {result.retcode} - {result.comment}")
                                            elif result:
                                                logger.info(f"MT5 Order Placed: {signal} on EURUSD")
                                        else:
                                            logger.warning("MT5 Tick not found for EURUSD")
                                    except Exception as e:
                                        logger.error(f"MT5 Execution Error: {e}")
                                
                                # Broadcast to dashboard
                                payload = {
                                    "timestamp": datetime.fromtimestamp(int(ticker["ts"])/1000).isoformat(),
                                    "price": price,
                                    "open": price, # Approximate for ticker
                                    "high": float(ticker["high24h"]),
                                    "low": float(ticker["low24h"]),
                                    "bias": float(bias),
                                    "stability": float(stability),
                                    "signal": signal,
                                    "is_legal": True,
                                    "adelic_active": True
                                }
                                await manager.broadcast(payload)

                        # Ping to keep alive
                        if int(time.time()) % 20 == 0:
                            await ws.send("ping")

            except Exception as e:
                logger.error(f"Bitget WS Error: {e}. Retrying in 5s...")
                await asyncio.sleep(5)

    async def run_production_loop(self):
        # We start the Rust engine and the Adelic pipeline
        try:
            logger.info("🚀 [QUIMERIA] Starting Adelic Pipeline and Engine...")
            self.engine.start()
            self.engine.start_adelic()
        except Exception as e:
            logger.error(f"Failed to start Adelic pipeline: {e}")
        
        # Core Bitget streamer runs in its own task
        await self.run_bitget_ws()

orchestrator = GMOSOrchestrator()

@app.on_event("startup")
async def startup_event():
    asyncio.create_task(orchestrator.run_production_loop())

if __name__ == "__main__":
    logger.info("🚀 Launching Hyperion Hybrid Server on http://localhost:8001")
    uvicorn.run(app, host="0.0.0.0", port=8001)
