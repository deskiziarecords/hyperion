use async_trait::async_trait;
use crate::types::{Order, RawTick};
use anyhow::Result;

#[async_trait]
pub trait ExchangeClient: Send + Sync {
    /// Ingest raw ticks from the exchange (REST or WebSocket)
    async fn stream_ticks(&self, symbol: &str) -> Result<tokio::sync::mpsc::Receiver<RawTick>>;
    
    /// Submit a live trade order
    async fn submit_order(&self, order: &Order) -> Result<String>;
    
    /// Get current account balance for a specific asset
    async fn get_balance(&self, asset: &str) -> Result<f64>;
}

pub mod okx;
pub mod bybit;
pub mod mexc;
pub mod bitget;
