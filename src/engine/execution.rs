use crate::types::*;
use crate::continuum::HamiltonianEngine;
use crate::engine::state::MarketState;

pub struct Detector {
    last_pattern: Option<Pattern>,
    prev_range: f64,
    prev_delta: f64,
}

impl Detector {
    pub fn new() -> Self {
        Self {
            last_pattern: None,
            prev_range: 0.0,
            prev_delta: 0.0,
        }
    }

    pub fn update(&mut self, candle: &Candle) -> Option<&'static str> {
        let range = candle.high - candle.low;
        let delta = (candle.close - candle.open).abs();

        let signal = if let Some(last) = self.last_pattern {
            match (last, candle.pattern) {
                (Pattern::U, Pattern::D) => Some("REVERSAL_DOWN"),
                (Pattern::D, Pattern::U) => Some("REVERSAL_UP"),
                _ if range > self.prev_range * 1.5 && self.prev_range > 0.0 => Some("VOLATILITY_EXPANSION"),
                _ if delta > self.prev_delta * 1.5 && self.prev_delta > 0.0 => Some("MOMENTUM_SPIKE"),
                _ => None,
            }
        } else {
            None
        };

        self.last_pattern = Some(candle.pattern);
        self.prev_range = range;
        self.prev_delta = delta;

        signal
    }
}

pub struct ExecutionEngine {
    pub h_engine: HamiltonianEngine,
    pub state: MarketState,
    pub detector: Detector,
}

impl ExecutionEngine {
    pub fn new(window_size: usize) -> Self {
        Self {
            h_engine: HamiltonianEngine::new(),
            state: MarketState::new(window_size),
            detector: Detector::new(),
        }
    }

    pub fn process_candle(&mut self, candle: Candle) -> Option<&'static str> {
        self.state.update(candle.clone());

        // Update detector
        let signal = self.detector.update(&candle);

        // Hamiltonian Context for refined filtering
        let ctx = MarketContext {
            eq_50: 1.1550,
            erl_target: 1.1419,
            irl_fvg: 1.1510
        };
        let frame = PriceFrame {
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            atr: 0.0020,
        };

        self.h_engine.energy = 0.92;
        let veto = self.h_engine.compute_displacement_veto(&frame);
        self.h_engine.update_state(frame.close, &ctx, veto);

        signal
    }
}
