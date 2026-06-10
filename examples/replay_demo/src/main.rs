//! Replay a recorded MCAP stream through the graph as a *virtual sensor*.
//!
//! `lair_bagel::replay::ReplaySource<f32>` is wired into the graph in place of a
//! real sensor driver. On startup we record a small fixture log; the source then
//! reads it back and re-emits the readings one per cycle, which the `printer` sink
//! reports. This is the foundation for deterministic, replay-driven testing.

use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;

pub mod tasks {
    use cu29::prelude::*;
    use lair_core::prelude::{LairConfig, LairContext, LairResult, LairSink};

    /// A sink that reports each replayed reading.
    pub struct Printer {}

    impl Freezable for Printer {}

    impl LairSink for Printer {
        type Resources<'r> = ();
        type Input<'m> = input_msg!(f32);

        fn new(_config: Option<&LairConfig>, _res: Self::Resources<'_>) -> LairResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(&mut self, _ctx: &LairContext, input: &Self::Input<'_>) -> LairResult<()> {
            if let Some(reading) = input.payload() {
                println!("printer <- replayed reading: {reading}");
            } else {
                println!("printer <- (recording exhausted)");
            }
            Ok(())
        }
    }
}

#[lair_runtime(config = "replay.ron")]
struct App {}

fn main() {
    // Resolve all relative paths against this crate, regardless of where `cargo
    // run` was invoked, so the fixture we write and the one the ReplaySource reads
    // line up.
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).expect("set cwd");

    // Record a fixture stream the virtual sensor will replay.
    let readings: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
    lair_bagel::write_payloads(
        std::path::Path::new("replay_fixture.mcap"),
        "/sensor",
        "f32",
        &readings,
    )
    .expect("record fixture");
    println!("recorded fixture: {readings:?}");

    let logger_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs/replay_demo.lair.mcap");
    let ctx = basic_copper_setup(&logger_path, None, true, None).expect("Failed to setup logger.");

    let mut app = AppBuilder::new()
        .with_context(&ctx)
        .build()
        .expect("Failed to create runtime");

    app.start_all_tasks().expect("Failed to start tasks.");
    // Run one more iteration than we recorded to show exhaustion.
    for _ in 0..(readings.len() + 1) {
        app.run_one_iteration().expect("Failed to run iteration.");
    }
    app.stop_all_tasks().expect("Failed to stop tasks.");
}
