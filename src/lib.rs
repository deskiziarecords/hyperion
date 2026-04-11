pub mod types;
pub mod continuum;
pub mod routing;
pub mod brokers;

pub mod data {
    pub mod candle_builder;
    pub mod realtime;
}

pub mod engine {
    pub mod execution;
    pub mod state;
}

use pyo3::prelude::*;
use crate::types::Candle;
use crate::data::realtime::OandaStream;
use crate::data::candle_builder::CandleBuilder;
use crate::engine::execution::ExecutionEngine;
use std::sync::Arc;
use parking_lot::RwLock;
use std::thread;
use std::time::Duration;

#[pyclass]
pub struct SentinelEngine {
    api_key: String,
    symbol: String,
    shared_state: Arc<RwLock<ExecutionEngine>>,
    is_running: Arc<RwLock<bool>>,
}

#[pymethods]
impl SentinelEngine {
    #[new]
    fn new(api_key: String, symbol: String) -> Self {
        Self {
            api_key,
            symbol,
            shared_state: Arc::new(RwLock::new(ExecutionEngine::new(100))),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    fn start(&self) {
        let mut running = self.is_running.write();
        if *running {
            return;
        }
        *running = true;

        let is_running = Arc::clone(&self.is_running);
        let shared_state = Arc::clone(&self.shared_state);

        thread::spawn(move || {
            let mut stream = OandaStream::new();
            let mut builder = CandleBuilder::new();

            while *is_running.read() {
                if let Some(tick) = stream.next_tick() {
                    if let Some(candle) = builder.update(tick.price, tick.time) {
                        let mut engine = shared_state.write();
                        engine.process_candle(candle);
                    }
                } else {
                    // Reset stream for simulation if it ends
                    stream = OandaStream::new();
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    fn stop(&self) {
        let mut running = self.is_running.write();
        *running = false;
    }

    fn get_latest_bars(&self, lookback: usize) -> Vec<Candle> {
        let engine = self.shared_state.read();
        let skip = engine.state.candles.len().saturating_sub(lookback);
        engine.state.candles.iter().skip(skip).cloned().collect()
    }

    fn execute_trade(&self, signal: String, size: f64) {
        let (dark, lit) = crate::routing::SchurRouter::route(size);
        println!("🚀 [RUST EXECUTION] {} | Total Size: {:.0}", signal, size);
        println!("   ↳ Routing: Dark Pool (61.8%)={:.0}, Lit Venue={:.0}", dark, lit);
    }

    fn cancel_all_pending(&self) {
        println!("🛑 [RUST] Cancelling all pending orders");
    }
}

#[pymodule]
fn hyperion_sentinel(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<SentinelEngine>()?;
    m.add_class::<Candle>()?;
    m.add_class::<types::Pattern>()?;
    Ok(())
}
