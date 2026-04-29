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
    logger.info("🚀 Launching QUIMERIA-HYPERION Hybrid Server on http://localhost:8000")
    uvicorn.run(app, host="0.0.0.0", port=8000)
