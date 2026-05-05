use chrono::{DateTime, Duration, Utc};
use crate::types::{Candle, Pattern};

pub struct CandleBuilder {
    open: Option<f64>,
    high: f64,
    low: f64,
    last_price: f64,
    bar_start: Option<DateTime<Utc>>,
    bar_duration_ms: i64,
}

impl CandleBuilder {
    pub fn new() -> Self {
        Self {
            open: None,
            high: f64::MIN,
            low: f64::MAX,
            last_price: 0.0,
            bar_start: None,
            bar_duration_ms: 60_000,
        }
    }

    pub fn update(&mut self, price: f64, time: DateTime<Utc>) -> Option<Candle> {
        if self.bar_start.is_none() {
            self.bar_start = Some(time);
            self.open = Some(price);
            self.high = price;
            self.low = price;
            self.last_price = price;
            return None;
        }

        let elapsed = time.signed_duration_since(self.bar_start.unwrap());
        if elapsed >= Duration::milliseconds(self.bar_duration_ms) {
            let candle = Candle {
                time: self.bar_start.unwrap(),
                open: self.open.unwrap_or(price),
                high: self.high,
                low: self.low,
                close: self.last_price,
                volume: 0.0,
                pattern: self.classify_pattern(),
            };
            self.bar_start = Some(time);
            self.open = Some(price);
            self.high = price;
            self.low = price;
            self.last_price = price;
            return Some(candle);
        }

        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.last_price = price;
        None
    }

    fn classify_pattern(&self) -> Pattern {
        let open = self.open.unwrap_or(self.last_price);
        let body = (self.last_price - open).abs();
        let range = (self.high - self.low).max(1e-9);
        let ratio = body / range;

        let name = if ratio < 0.10 {
            "DOJI"
        } else if self.last_price > open && ratio > 0.60 {
            "BULLISH_MARUBOZU"
        } else if self.last_price < open && ratio > 0.60 {
            "BEARISH_MARUBOZU"
        } else if self.last_price > open {
            "BULLISH"
        } else {
            "BEARISH"
        };

        Pattern { name: name.to_string(), strength: ratio }
    }
}
