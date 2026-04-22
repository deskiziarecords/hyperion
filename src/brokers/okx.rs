use async_trait::async_trait;
use crate::types::{Order, RawTick};
use crate::brokers::ExchangeClient;
use anyhow::Result;

pub struct OkxClient;
#[async_trait]
impl ExchangeClient for OkxClient {
    async fn stream_ticks(&self, _symbol: &str) -> Result<tokio::sync::mpsc::Receiver<RawTick>> { todo!() }
    async fn submit_order(&self, _order: &Order) -> Result<String> { Ok("OKX-STUB".into()) }
    async fn get_balance(&self, _asset: &str) -> Result<f64> { Ok(0.0) }
}
