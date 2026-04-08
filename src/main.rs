use hyperion_adco::data::realtime::OandaStream;
use hyperion_adco::data::candle_builder::CandleBuilder;
use hyperion_adco::engine::execution::ExecutionEngine;

fn main() {
    println!("🚀 Starting Hyperion Engine...");

    let mut stream = OandaStream::new();
    let mut builder = CandleBuilder::new();
    let mut engine = ExecutionEngine::new(100); // 100-candle rolling window

    while let Some(tick) = stream.next_tick() {
        if let Some(candle) = builder.update(tick.price, tick.time) {
            println!(
                "[{}] M1 Candle: O={:.5} H={:.5} L={:.5} C={:.5} Pattern={:?}",
                candle.time, candle.open, candle.high, candle.low, candle.close, candle.pattern
            );

            engine.process_candle(candle);
        }
    }

    println!("✅ Hyperion execution cycle complete.");
}
