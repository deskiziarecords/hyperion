use async_trait::async_trait;
use crate::types::{Order, RawTick, Action};
use crate::brokers::ExchangeClient;
use anyhow::{Result, anyhow};
use reqwest::Client;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct BitgetClient {
    api_key: String,
    secret: String,
    passphrase: String,
    client: Client,
}

impl BitgetClient {
    pub fn new(api_key: String, secret: String, passphrase: String) -> Self {
        Self {
            api_key,
            secret,
            passphrase,
            client: Client::new(),
        }
    }

    fn generate_signature(&self, timestamp: &str, method: &str, request_path: &str, body: &str) -> String {
        let message = format!("{}{}{}{}", timestamp, method, request_path, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        base64::encode(mac.finalize().into_bytes())
    }
}

#[async_trait]
impl ExchangeClient for BitgetClient {
    async fn stream_ticks(&self, _symbol: &str) -> Result<tokio::sync::mpsc::Receiver<RawTick>> {
        // WebSocket implementation for Bitget would go here
        Err(anyhow!("WebSocket streaming not implemented for Bitget yet"))
    }

    async fn submit_order(&self, order: &Order) -> Result<String> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis().to_string();
        let method = "POST";
        let path = "/api/v2/mix/order/place-order";
        
        let side = match order.action {
            Action::Buy => "buy",
            Action::Sell => "sell",
            Action::Flat => return Err(anyhow!("Cannot submit FLAT order")),
        };

        // Simplified payload
        let body = format!(r#"{{"symbol":"BTCUSDT","marginCoin":"USDT","side":"{}","orderType":"market","size":"{}"}}"#, side, order.size);
        let signature = self.generate_signature(&timestamp, method, path, &body);

        // In a real implementation, we would send the request here:
        // let res = self.client.post(format!("https://api.bitget.com{}", path))
        //     .header("ACCESS-KEY", &self.api_key)
        //     .header("ACCESS-SIGN", signature)
        //     .header("ACCESS-TIMESTAMP", timestamp)
        //     .header("ACCESS-PASSPHRASE", &self.passphrase)
        //     .body(body)
        //     .send().await?;

        Ok(format!("BITGET-SIM-{}", order.id))
    }

    async fn get_balance(&self, _asset: &str) -> Result<f64> {
        Ok(1000.0) // Stub
    }
}
