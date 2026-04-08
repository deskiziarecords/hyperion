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

        // Generate some mock EUR/USD ticks
        for i in 0..200 {
            let mut price = 1.1550 + (i as f64 * 0.0001).sin();

            // Introduce an "institutional move" at i=100
            if i >= 100 && i < 106 {
                price += (i - 100) as f64 * 0.0010;
            } else if i >= 106 {
                price += 0.0060;
            }

            ticks.push(Tick {
                time: now + chrono::Duration::seconds(i * 10),
                price,
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
