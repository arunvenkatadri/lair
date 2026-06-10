//! A LAIR pipeline that demonstrates safety as a *graph node*.
//!
//! ```text
//! planner ──(ControlCommand)──> safety_guard ──(ControlCommand)──> actuator
//! ```
//!
//! The `planner` deliberately emits an unsafe command (throttle out of range,
//! throttle and brake at once, steering past the wheel lock). Because
//! `lair_biscuit::SafetyGuardTask` sits on the wire between the planner and the
//! actuator, the actuator only ever observes a command clamped into the safe
//! envelope — the planner cannot reach the actuator unchecked.

use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;

pub mod tasks {
    use cu29::prelude::*;
    use lair_core::prelude::{LairConfig, LairContext, LairResult, LairSink, LairSource};
    use lair_msgs::{ControlCommand, Gear};

    /// A stand-in planner that emits a deliberately unsafe command.
    pub struct Planner {}

    impl Freezable for Planner {}

    impl LairSource for Planner {
        type Resources<'r> = ();
        type Output<'m> = output_msg!(ControlCommand);

        fn new(_config: Option<&LairConfig>, _res: Self::Resources<'_>) -> LairResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(&mut self, _ctx: &LairContext, out: &mut Self::Output<'_>) -> LairResult<()> {
            // Unsafe on purpose: throttle > 1, throttle+brake conflict, steering past lock.
            out.set_payload(ControlCommand {
                throttle: 2.0,
                brake: 0.5,
                steering: 5.0,
                gear: Gear::Drive,
            });
            Ok(())
        }
    }

    /// An actuator that reports what it actually received.
    pub struct Actuator {}

    impl Freezable for Actuator {}

    impl LairSink for Actuator {
        type Resources<'r> = ();
        type Input<'m> = input_msg!(ControlCommand);

        fn new(_config: Option<&LairConfig>, _res: Self::Resources<'_>) -> LairResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(&mut self, _ctx: &LairContext, input: &Self::Input<'_>) -> LairResult<()> {
            if let Some(cmd) = input.payload() {
                println!(
                    "actuator <- throttle={:.2} brake={:.2} steering={:.3} gear={:?}",
                    cmd.throttle, cmd.brake, cmd.steering, cmd.gear
                );
            }
            Ok(())
        }
    }
}

#[lair_runtime(config = "vehicle.ron")]
struct App {}

fn main() {
    let logger_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs/safe_vehicle.lair.mcap");
    let ctx = basic_copper_setup(&logger_path, None, true, None).expect("Failed to setup logger.");

    let mut app = AppBuilder::new()
        .with_context(&ctx)
        .build()
        .expect("Failed to create runtime");

    app.start_all_tasks().expect("Failed to start tasks.");

    println!("planner emits: throttle=2.00 brake=0.50 steering=5.000 gear=Drive (UNSAFE)");
    // Bounded run so the example terminates cleanly.
    for _ in 0..3 {
        app.run_one_iteration().expect("Failed to run iteration.");
    }

    app.stop_all_tasks().expect("Failed to stop tasks.");
}
