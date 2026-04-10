import jax
import jax.numpy as jnp
import numpy as np
import time
import logging

# --- IMPORT YOUR COMPILED RUST KERNEL ---
# Assumes you ran: maturin develop inside the hyperion folder
try:
    import hyperion_sentinel as sentinel
except ImportError:
    print("CRITICAL: Run 'maturin develop' in /hyperion/ first.")
    exit(1)

# --- IMPORT YOUR JAX BRAIN ---
from logic.adelic_koopman_ipda_synchronizer import AdelicKoopmanSynchronizer
from logic.reverse_period_detector import ReversePeriodDetector

# Configure Logging for the Friday Demo
logging.basicConfig(level=logging.INFO, format='%(asctime)s [%(levelname)s] %(message)s')
logger = logging.getLogger("GMOS_HYBRID")

class GMOSOrchestrator:
    def __init__(self):
        # Initialize the Brain (JAX)
        self.brain = AdelicKoopmanSynchronizer()
        self.detector = ReversePeriodDetector()

        # Warm up the JAX JIT before the market opens
        dummy_data = jnp.zeros((60,))
        _ = self.brain.compute_sync(dummy_data)
        logger.info("JAX Kernels Hot and Ready.")

        # Initialize the Sentinel (Rust)
        # Pass your TwelveData API Key and target symbol
        self.engine = sentinel.SentinelEngine(api_key="YOUR_KEY", symbol="EUR/USD")

        self.is_active = True

    def run_production_loop(self):
        logger.info("Starting GMOS Hybrid Pipeline: Sentinel (Rust) + Brain (JAX)")

        # Start the Rust thread for io_uring ingestion
        self.engine.start()

        try:
            while self.is_active:
                # 1. PULL CLEAN DATA FROM RUST
                # Rust returns a list of Bar objects (OHLCV + OFI)
                bars = self.engine.get_latest_bars(lookback=60)

                if len(bars) < 20:
                    logger.info(f"Warming up... {len(bars)}/20 bars")
                    time.sleep(1)
                    continue

                # 2. CONVERT TO JAX TENSORS
                price_data = jnp.array([b.close for b in bars])
                volume_data = jnp.array([b.volume for b in bars])

                # 3. COMPUTE ADELIC BIAS (THE BRAIN)
                # This uses your XLA-fused Mandra-Gate primitives
                bias, stability, q_t_size = self.brain.compute_sync(price_data)

                # 4. LAMBDA-6 DISPLACEMENT VETO (THE SAFETY)
                current_bar = bars[-1]
                body = abs(current_bar.close - current_bar.open)
                range_total = current_bar.high - current_bar.low

                # Refined threshold for NFP/NY Open volatility
                # VETO if wicks are too large (body-to-range ratio must be > 0.75)
                is_legal = (body / (range_total + 1e-9)) > 0.75

                # 5. EXECUTION DECISION
                if is_legal and stability > 0.85:
                    signal = "BUY" if bias > 0 else "SELL"
                    # Pass the signal BACK to Rust for sub-millisecond execution
                    self.engine.execute_trade(signal, size=q_t_size)
                    logger.info(f"EXECUTION: {signal} | Size: {q_t_size} | Stability: {stability:.4f}")
                else:
                    # Veto active or unstable manifold
                    self.engine.cancel_all_pending()
                    if not is_legal:
                        logger.warning(f"VETO: λ6 Displacement Violation (Body/Range Ratio: {(body/range_total):.2f})")
                    if stability <= 0.85:
                        logger.error(f"CIRCUIT BREAKER: Regime Fracture Detected (Stability: {stability:.4f})")

                # Frequency control (sync with bar close)
                time.sleep(0.5)

        except KeyboardInterrupt:
            logger.info("Shutting down safely...")
        finally:
            self.engine.stop()

if __name__ == "__main__":
    gmos = GMOSOrchestrator()
    gmos.run_production_loop()
