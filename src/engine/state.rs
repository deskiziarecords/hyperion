use crate::types::Candle;
use std::collections::VecDeque;

pub struct MarketState {
    pub window_size: usize,
    pub candles: VecDeque<Candle>,
}

impl MarketState {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            candles: VecDeque::with_capacity(window_size),
        }
    }

    pub fn update(&mut self, candle: Candle) {
        if self.candles.len() >= self.window_size {
            self.candles.pop_front();
        }
        self.candles.push_back(candle);
    }

    pub fn last_candle(&self) -> Option<&Candle> {
        self.candles.back()
    }
}
