use crate::types::*;

pub struct HamiltonianEngine {
    pub energy: f64,
    pub state: Phase,
}

impl HamiltonianEngine {
    pub fn new() -> Self {
        Self { energy: 0.0, state: Phase::Consolidation }
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
            Phase::Consolidation if !veto && self.energy > 0.85 => {
                self.state = Phase::Expansion;
                self.energy = 0.0; 
            },
            Phase::Expansion if (price - context.erl_target).abs() < 0.0005 => {
                self.state = Phase::Reversal;
            },
            _ => {
                if (price - context.eq_50).abs() < 0.0002 {
                    self.state = Phase::Consolidation;
                }
            }
        }
    }
}
