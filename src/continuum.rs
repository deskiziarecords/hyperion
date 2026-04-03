use crate::types::*;

pub struct HamiltonianEngine {
    pub energy: f64,
    pub state: ContinuumState,
}

impl HamiltonianEngine {
    pub fn new() -> Self {
        Self { energy: 0.0, state: ContinuumState::Consolidation }
    }

    pub fn compute_displacement_veto(&self, frame: &PriceFrame) -> bool {
        let range = frame.high - frame.low;
        if range <= f64::EPSILON { return true; }
        let body = (frame.close - frame.open).abs();
        
        // Veto if NOT a high-conviction institutional move
        !((body / range > 0.7) && (range > 1.5 * frame.atr))
    }

    pub fn update_state(&mut self, price: f64, context: &MarketContext, veto: bool) {
        match self.state {
            ContinuumState::Consolidation if !veto && self.energy > 0.85 => {
                self.state = ContinuumState::Expansion;
                self.energy = 0.0; 
            },
            ContinuumState::Expansion if (price - context.erl_target).abs() < 0.0005 => {
                self.state = ContinuumState::Reversal;
            },
            _ => {
                if (price - context.eq_50).abs() < 0.0002 {
                    self.state = ContinuumState::Consolidation;
                }
            }
        }
    }
}
