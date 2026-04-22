use chrono::Utc;
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

use crate::brokers::bitget::BitgetClient;
use crate::brokers::ExchangeClient;
use crate::types::Action;
use crate::types::Order;

#[pyclass]
pub struct SentinelEngine {
    api_key: String,
    symbol: String,
    shared_state: Arc<RwLock<ExecutionEngine>>,
    is_running: Arc<RwLock<bool>>,
    broker: Option<Arc<dyn ExchangeClient>>,
}

#[pymethods]
impl SentinelEngine {
    #[new]
    #[pyo3(signature = (api_key="".to_string(), symbol="EUR/USD".to_string(), use_bitget=false))]
    fn new(api_key: String, symbol: String, use_bitget: bool) -> Self {
        // Load .env if it exists
        let _ = dotenvy::dotenv();

        let broker: Option<Arc<dyn ExchangeClient>> = if use_bitget {
            let bg_key = std::env::var("BITGET_API_KEY").unwrap_or_default();
            let bg_secret = std::env::var("BITGET_SECRET").unwrap_or_default();
            let bg_pass = std::env::var("BITGET_PASSPHRASE").unwrap_or_default();
            
            if !bg_key.is_empty() {
                println!("🌐 Initializing Bitget Client...");
                Some(Arc::new(BitgetClient::new(bg_key, bg_secret, bg_pass)))
            } else {
                println!("⚠️ Bitget requested but BITGET_API_KEY not found in .env");
                None
            }
        } else {
            None
        };

        Self {
            api_key,
            symbol,
            shared_state: Arc::new(RwLock::new(ExecutionEngine::new(100))),
            is_running: Arc::new(RwLock::new(false)),
            broker,
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
        let broker = self.broker.clone();
        let symbol = self.symbol.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let mut builder = CandleBuilder::new();
                
                println!("🚀 [RUST] Engine Started. Mode: {}", if broker.is_some() { "LIVE/BROKER" } else { "SIMULATION" });

                if let Some(broker_client) = &broker {
                    let mut rx = match broker_client.stream_ticks(&symbol).await {
                        Ok(r) => r,
                        Err(e) => {
                            println!("❌ [RUST] Failed to start broker stream: {}", e);
                            return;
                        }
                    };

                    while *is_running.read() {
                        if let Some(raw_tick) = rx.recv().await {
                            // Convert RawTick to Candle updates
                            // We use Utc::now() for time if not provided or to ensure sync
                            let time = Utc::now();
                            if let Some(candle) = builder.update(raw_tick.price, time) {
                                let mut engine = shared_state.write();
                                engine.process_candle(candle);
                            }
                        }
                    }
                } else {
                    // SIMULATION MODE
                    let mut stream = OandaStream::new();
                    while *is_running.read() {
                        if let Some(tick) = stream.next_tick() {
                            if let Some(candle) = builder.update(tick.price, tick.time) {
                                let mut engine = shared_state.write();
                                engine.process_candle(candle);
                            }
                        } else {
                            stream = OandaStream::new();
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
            });
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
        
        let action = if signal == "BUY" { Action::Buy } else { Action::Sell };
        let order = Order {
            id: format!("ord_{}", Utc::now().timestamp_ms()),
            action,
            size,
            ref_price: 0.0, // Should ideally be current market price
            timestamp: Utc::now().timestamp_millis() as f64,
        };

        if let Some(broker) = &self.broker {
            let broker = Arc::clone(broker);
            // Spawn execution in a background tokio task
            thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match broker.submit_order(&order).await {
                        Ok(id) => println!("✅ [BROKER] Order Submitted: {}", id),
                        Err(e) => println!("❌ [BROKER] Order Failed: {}", e),
                    }
                });
            });
        }

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
