pub struct SchurRouter;

impl SchurRouter {
    /// Splits order size between dark pool and lit venue using golden-ratio allocation.
    /// Returns (dark_pool_size, lit_venue_size).
    pub fn route(size: f64) -> (f64, f64) {
        let dark = size * 0.618;
        let lit = size - dark;
        (dark, lit)
    }
}
