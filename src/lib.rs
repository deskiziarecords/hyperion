use chrono::Utc;
pub mod types;
pub mod continuum;
pub mod routing;
pub mod brokers;
pub mod config;
pub mod state;
pub mod phase;
pub mod indicators;
pub mod kill_zone;
pub mod urol;
pub mod ipda;
pub mod aecabi;

pub mod data {
    pub mod candle_builder;
    pub mod realtime;
}

pub mod engine {
    pub mod execution;
    pub mod state;
}

use pyo3::prelude::*;
use crate::types::{Candle, Bar, Signal};
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
    redis_url: String,
    adelic_state: Option<crate::state::SharedState>,
}

#[pymethods]
impl SentinelEngine {
    #[new]
    #[pyo3(signature = (api_key="".to_string(), symbol="EUR/USD".to_string(), use_bitget=false, redis_url="redis://127.0.0.1/".to_string()))]
    fn new(api_key: String, symbol: String, use_bitget: bool, redis_url: String) -> Self {
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
            redis_url,
            adelic_state: None,
        }
    }

    fn start(&mut self) {
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

    fn start_adelic(&mut self) {
        let mut running = self.is_running.write();
        if *running {
            return;
        }
        *running = true;

        let redis_url = self.redis_url.clone();
        let adelic_state = Arc::new(tokio::sync::RwLock::new(crate::state::GlobalState::default()));
        self.adelic_state = Some(adelic_state.clone());

        let is_running = Arc::clone(&self.is_running);

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                println!("🌐 [QUIMERIA] Starting Adelic Pipeline on {}...", redis_url);

                let client = match redis::Client::open(redis_url) {
                    Ok(c) => c,
                    Err(e) => {
                        println!("❌ [QUIMERIA] Redis connection failed: {}", e);
                        return;
                    }
                };

                let con = match client.get_multiplexed_tokio_connection().await {
                    Ok(c) => c,
                    Err(e) => {
                        println!("❌ [QUIMERIA] Multiplexed connection failed: {}", e);
                        return;
                    }
                };

                // Spawn UROL Ingestion
                let urol_con = con.clone();
                let urol_state = adelic_state.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::urol::run_ingestion(urol_con, urol_state).await {
                        println!("❌ [UROL] Ingestion error: {}", e);
                    }
                });

                // Spawn UROL Watchdog
                let watchdog_con = con.clone();
                let watchdog_state = adelic_state.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::urol::run_watchdog(watchdog_con, watchdog_state).await {
                        println!("❌ [WATCHDOG] error: {}", e);
                    }
                });

                // Spawn IPDA Core
                let ipda_con = con.clone();
                let ipda_state = adelic_state.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::ipda::run(ipda_con, ipda_state).await {
                        println!("❌ [IPDA] error: {}", e);
                    }
                });

                // Spawn AECABI Gateway
                let aecabi_con = con.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::aecabi::run(aecabi_con).await {
                        println!("❌ [AECABI] error: {}", e);
                    }
                });

                while *is_running.read() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                println!("🛑 [QUIMERIA] Adelic Pipeline Stopped.");
            });
        });
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
            id: format!("ord_{}", Utc::now().timestamp_millis()),
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
fn quimeria_hyperion(m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
    m.add_class::<SentinelEngine>()?;
    m.add_class::<Candle>()?;
    m.add_class::<types::Pattern>()?;
    m.add_class::<Bar>()?;
    m.add_class::<Signal>()?;
    m.add_class::<Order>()?;
    m.add_class::<types::Action>()?;
    m.add_class::<types::Phase>()?;
    Ok(())
}
