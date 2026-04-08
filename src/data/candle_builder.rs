use chrono::{DateTime, Utc};
use crate::types::{Candle, Pattern};

pub struct CandleBuilder {
    current_minute: Option<i64>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

impl CandleBuilder {
    pub fn new() -> Self {
        Self {
            current_minute: None,
            open: 0.0,
            high: f64::MIN,
            low: f64::MAX,
            close: 0.0,
        }
    }

    pub fn update(&mut self, price: f64, time: DateTime<Utc>) -> Option<Candle> {
        let minute = time.timestamp() / 60;

        if let Some(current) = self.current_minute {
            if minute != current {
                let candle = Candle {
                    time,
                    open: self.open,
                    high: self.high,
                    low: self.low,
                    close: self.close,
                    volume: 0.0,
                    pattern: classify_pattern(self.open, self.close),
                };

                self.open = price;
                self.high = price;
                self.low = price;
                self.close = price;
                self.current_minute = Some(minute);

                return Some(candle);
            }
        } else {
            self.current_minute = Some(minute);
            self.open = price;
            self.high = price;
            self.low = price;
        }

        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;

        None
    }
}

fn classify_pattern(open: f64, close: f64) -> Pattern {
    if close > open {
        Pattern::U
    } else if close < open {
        Pattern::D
    } else {
        Pattern::I
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_candle_building() {
        let mut builder = CandleBuilder::new();
        let t1 = Utc.timestamp_opt(60, 0).unwrap(); // Start of minute 1
        let t2 = Utc.timestamp_opt(120, 0).unwrap(); // Start of minute 2

        // First tick
        assert!(builder.update(1.10, t1).is_none());
        // Second tick same minute
        assert!(builder.update(1.15, t1 + chrono::Duration::seconds(30)).is_none());

        // Tick in new minute triggers candle completion
        let candle = builder.update(1.12, t2).expect("Should return a candle");
        assert_eq!(candle.open, 1.10);
        assert_eq!(candle.high, 1.15);
        assert_eq!(candle.low, 1.10);
        assert_eq!(candle.close, 1.15);
    }
}
