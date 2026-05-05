use async_trait::async_trait;
use crate::types::{Order, RawTick};
use anyhow::Result;

pub mod bitget;

#[async_trait]
pub trait ExchangeClient: Send + Sync {
    async fn stream_ticks(&self, symbol: &str) -> Result<tokio::sync::mpsc::Receiver<RawTick>>;
    async fn submit_order(&self, order: &Order) -> Result<String>;
    async fn get_balance(&self, asset: &str) -> Result<f64>;
}
