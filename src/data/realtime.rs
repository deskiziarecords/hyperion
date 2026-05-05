use chrono::{DateTime, Duration, Utc};

pub struct Tick {
    pub price: f64,
    pub time: DateTime<Utc>,
}

pub struct OandaStream {
    prices: Vec<f64>,
    index: usize,
    start: DateTime<Utc>,
}

impl OandaStream {
    /// Produces synthetic EUR/USD tick data for simulation mode.
    pub fn new() -> Self {
        let mut prices = Vec::with_capacity(500);
        let mut price = 1.085_00;
        for i in 0..500_usize {
            let drift = (i as f64 * 0.137).sin() * 0.000_10;
            let spike = if i % 7 == 0 { 0.000_20 } else { -0.000_05 };
            price += drift + spike;
            price = price.clamp(1.060_00, 1.110_00);
            prices.push(price);
        }
        Self { prices, index: 0, start: Utc::now() }
    }

    pub fn next_tick(&mut self) -> Option<Tick> {
        if self.index >= self.prices.len() {
            return None;
        }
        let tick = Tick {
            price: self.prices[self.index],
            time: self.start + Duration::seconds(self.index as i64),
        };
        self.index += 1;
        Some(tick)
    }
}
