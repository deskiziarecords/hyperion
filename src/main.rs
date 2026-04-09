use hyperion_sentinel::data::realtime::OandaStream;
use hyperion_sentinel::data::candle_builder::CandleBuilder;
use hyperion_sentinel::engine::execution::ExecutionEngine;
use hyperion_sentinel::routing::SchurRouter;

fn main() {
    println!("🚀 Starting Hyperion Orchestrator...");

    let mut stream = OandaStream::new();
    let mut builder = CandleBuilder::new();
    let mut engine = ExecutionEngine::new(100);

    while let Some(tick) = stream.next_tick() {
        if let Some(candle) = builder.update(tick.price, tick.time) {
            let pattern = candle.pattern;
            let time = candle.time;

            if let Some(signal) = engine.process_candle(candle) {
                let (dark, lit) = SchurRouter::route(5_000_000.0);
                println!(
                    "[{}] ⚡ SIGNAL: {} | Pattern: {:?} | Route: Dark={:.0}, Lit={:.0}",
                    time, signal, pattern, dark, lit
                );
            } else {
                // Optional: print silent candles for debugging
                // println!("[{}] (no signal) Pattern: {:?}", time, pattern);
            }
        }
    }

    println!("✅ Hyperion execution cycle complete.");
}
