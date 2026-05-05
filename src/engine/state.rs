use crate::types::Candle;

pub struct EngineState {
    pub candles: Vec<Candle>,
    capacity: usize,
}

impl EngineState {
    pub fn new(capacity: usize) -> Self {
        Self {
            candles: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, candle: Candle) {
        self.candles.push(candle);
        if self.candles.len() > self.capacity {
            self.candles.remove(0);
        }
    }
}
