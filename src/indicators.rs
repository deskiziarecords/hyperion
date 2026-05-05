use crate::types::Bar;

pub fn atr(bars: &[Bar], period: usize) -> f64 {
    if bars.len() < period {
        return 0.0;
    }

    let mut tr_sum = 0.0;
    for i in (bars.len() - period)..bars.len() {
        let bar = &bars[i];
        let tr = if i == 0 {
            bar.high - bar.low
        } else {
            let prev_close = bars[i - 1].close;
            let hl = bar.high - bar.low;
            let hpc = (bar.high - prev_close).abs();
            let lpc = (bar.low - prev_close).abs();
            hl.max(hpc).max(lpc)
        };
        tr_sum += tr;
    }

    tr_sum / period as f64
}
