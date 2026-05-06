use async_trait::async_trait;
use crate::types::{Order, RawTick, Action};
use crate::brokers::ExchangeClient;
use anyhow::{Result, anyhow};
use reqwest::Client;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose};
use std::time::Duration;
use chrono::Utc;

pub struct BitgetClient {
    api_key: String,
    secret: String,
    passphrase: String,
    client: Client,
    base_url: String,
}

impl BitgetClient {
    pub fn new(api_key: String, secret: String, passphrase: String) -> Self {
        Self {
            api_key,
            secret,
            passphrase,
            client: Client::new(),
            base_url: "https://api.bitget.com".to_string(),
        }
    }

    fn generate_signature(&self, timestamp: &str, method: &str, request_path: &str, body: &str) -> String {
        let message = format!("{}{}{}{}", timestamp, method, request_path, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        general_purpose::STANDARD.encode(mac.finalize().into_bytes())
    }
}

#[async_trait]
impl ExchangeClient for BitgetClient {
    async fn stream_ticks(&self, symbol: &str) -> Result<tokio::sync::mpsc::Receiver<RawTick>> {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        let url = "wss://ws.bitget.com/v2/ws/public";
        let symbol = symbol.to_string();

        tokio::spawn(async move {
            use futures_util::{StreamExt, SinkExt};
            use tokio_tungstenite::connect_async;
            use tokio_tungstenite::tungstenite::protocol::Message;

            loop {
                println!("🔄 [BITGET] Connecting to WebSocket: {}...", url);
                match connect_async(url).await {
                    Ok((mut ws_stream, _)) => {
                        println!("✅ [BITGET] WebSocket Connected.");

                        // Subscribe to tickers
                        // For Bitget, pairs are usually symbol+USDT
                        let inst_id = if symbol.contains("/") {
                            symbol.replace("/", "")
                        } else {
                            symbol.clone()
                        };

                        let sub_msg = serde_json::json!({
                            "op": "subscribe",
                            "args": [{
                                "instType": "SPOT",
                                "channel": "tickers",
                                "instId": inst_id
                            }]
                        });

                        if let Err(e) = ws_stream.send(Message::Text(sub_msg.to_string())).await {
                            println!("❌ [BITGET] Subscription Failed: {}", e);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }

                        let mut ping_interval = tokio::time::interval(Duration::from_secs(25));

                        loop {
                            tokio::select! {
                                _ = ping_interval.tick() => {
                                    if let Err(e) = ws_stream.send(Message::Text("ping".to_string())).await {
                                        println!("❌ [BITGET] Ping Failed: {}", e);
                                        break;
                                    }
                                }
                                msg = ws_stream.next() => {
                                    match msg {
                                        Some(Ok(Message::Text(text))) => {
                                            if text == "pong" { continue; }
                                            
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                                if let Some(data_array) = json["data"].as_array() {
                                                    for item in data_array {
                                                        let price = item["lastPr"].as_str().and_then(|p| p.parse::<f64>().ok()).unwrap_or(0.0);
                                                        let ts = item["ts"].as_str().and_then(|t| t.parse::<i64>().ok()).unwrap_or_else(|| Utc::now().timestamp_millis());
                                                        
                                                        let tick = RawTick {
                                                            price,
                                                            volume: 1.0, // Ticker doesn't always give last trade volume
                                                            ts_ms: ts,
                                                        };
                                                        if tx.send(tick).await.is_err() {
                                                            return;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                                            println!("⚠️ [BITGET] WebSocket Disconnected.");
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ [BITGET] Connection Failed: {}. Retrying in 5s...", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(rx)
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

        // Normalize: "BTC/USDT" → "BTCUSDT" for Bitget Mix API
        let inst_id = order.symbol.replace('/', "");
        let body = format!(r#"{{"symbol":"{}","marginCoin":"USDT","side":"{}","orderType":"market","size":"{}"}}"#, inst_id, side, order.size);
        let signature = self.generate_signature(&timestamp, method, path, &body);

        let res = self.client.post(format!("{}{}", self.base_url, path))
            .header("ACCESS-KEY", &self.api_key)
            .header("ACCESS-SIGN", signature)
            .header("ACCESS-TIMESTAMP", &timestamp)
            .header("ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        let status = res.status();
        let text = res.text().await?;
        if status.is_success() {
            Ok(format!("BITGET-SUCCESS-{}", text))
        } else {
            Err(anyhow!("Bitget Order Error: {} - {}", status, text))
        }
    }

    async fn get_balance(&self, asset: &str) -> Result<f64> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis().to_string();
        let method = "GET";
        let path = format!("/api/v2/mix/account/account?symbol={}USDT&marginCoin=USDT", asset);
        
        let signature = self.generate_signature(&timestamp, method, &path, "");

        let res = self.client.get(format!("{}{}", self.base_url, path))
            .header("ACCESS-KEY", &self.api_key)
            .header("ACCESS-SIGN", signature)
            .header("ACCESS-TIMESTAMP", &timestamp)
            .header("ACCESS-PASSPHRASE", &self.passphrase)
            .send()
            .await?;

        if res.status().is_success() {
            let json: serde_json::Value = res.json().await?;
            // Extract balance from Bitget's nested response structure
            let balance = json["data"]["available"].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            Ok(balance)
        } else {
            Err(anyhow!("Bitget Balance Error: {}", res.status()))
        }
    }
}
