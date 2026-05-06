use chrono::Utc;
use crate::types::{Action, Candle, Phase, Signal};
use crate::engine::state::EngineState;

pub struct ExecutionEngine {
    pub state: EngineState,
}

impl ExecutionEngine {
    pub fn new(capacity: usize) -> Self {
        Self { state: EngineState::new(capacity) }
    }

    pub fn process_candle(&mut self, candle: Candle) -> Option<Signal> {
        self.state.push(candle);

        let n = self.state.candles.len();
        if n < 3 {
            return None;
        }

        let prev = &self.state.candles[n - 2];
        let curr = &self.state.candles[n - 1];

        let body = (curr.close - curr.open).abs();
        let range = (curr.high - curr.low).max(1e-9);
        let ratio = body / range;

        if ratio > 0.65 && curr.close > curr.open && curr.close > prev.high {
            return Some(Signal {
                action: Action::Buy,
                size: 1.0,
                phase: Phase::Expansion,
                kill_zone: false,
                timestamp: Utc::now().timestamp_millis() as f64,
                price: Some(curr.close),
            });
        }

        if ratio > 0.65 && curr.close < curr.open && curr.close < prev.low {
            return Some(Signal {
                action: Action::Sell,
                size: 1.0,
                phase: Phase::Distribution,
                kill_zone: false,
                timestamp: Utc::now().timestamp_millis() as f64,
                price: Some(curr.close),
            });
        }

        None
    }
}
