use chrono::{DateTime, Utc};

pub struct Tick {
    pub time: DateTime<Utc>,
    pub price: f64,
}

pub struct OandaStream {
    ticks: Vec<Tick>,
    index: usize,
}

impl OandaStream {
    pub fn new() -> Self {
        let now = Utc::now();
        let mut ticks = Vec::new();
        let mut current_price = 1.1550;

        // Generate more realistic EUR/USD ticks with noise and spikes
        for i in 0..600 {
            // Base noise
            let noise = ((i as f64 * 0.1).sin() * 0.0002) + ((i as f64 * 0.5).cos() * 0.0001);
            current_price += noise;

            // Occasional trend
            if i > 100 && i < 200 {
                current_price += 0.00005; // Gentle uptrend
            }

            // Sudden Momentum Spike
            if i > 300 && i < 310 {
                current_price += 0.0005;
            }

            // Reversal
            if i > 450 && i < 550 {
                current_price -= 0.00008; // Downtrend
            }

            ticks.push(Tick {
                time: now + chrono::Duration::seconds(i * 10),
                price: current_price,
            });
        }

        Self { ticks, index: 0 }
    }

    pub fn next_tick(&mut self) -> Option<Tick> {
        if self.index < self.ticks.len() {
            let tick = &self.ticks[self.index];
            self.index += 1;
            Some(Tick {
                time: tick.time,
                price: tick.price,
            })
        } else {
            None
        }
    }
}
