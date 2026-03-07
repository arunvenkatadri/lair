use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;

pub mod tasks {
    use cu29::prelude::*;

    // Import LAIR API names
    use lair_core::prelude::{LairConfig, LairContext, LairResult, LairSink, LairSource, LairTask};

    pub struct SensorSource {}

    impl Freezable for SensorSource {}

    impl LairSource for SensorSource {
        type Resources<'r> = ();
        type Output<'m> = output_msg!(f32);

        fn new(
            _config: Option<&LairConfig>,
            _resources: Self::Resources<'_>,
        ) -> LairResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(
            &mut self,
            _ctx: &LairContext,
            new_msg: &mut Self::Output<'_>,
        ) -> LairResult<()> {
            new_msg.set_payload(1.0_f32);
            Ok(())
        }
    }

    pub struct Processor {}

    impl Freezable for Processor {}

    impl LairTask for Processor {
        type Resources<'r> = ();
        type Input<'m> = input_msg!(f32);
        type Output<'m> = output_msg!(f32);

        fn new(
            _config: Option<&LairConfig>,
            _resources: Self::Resources<'_>,
        ) -> LairResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(
            &mut self,
            _ctx: &LairContext,
            input: &Self::Input<'_>,
            output: &mut Self::Output<'_>,
        ) -> LairResult<()> {
            let val = input.payload().unwrap_or(&0.0);
            output.set_payload(val * 2.0);
            Ok(())
        }
    }

    pub struct ActuatorSink {}

    impl Freezable for ActuatorSink {}

    impl LairSink for ActuatorSink {
        type Resources<'r> = ();
        type Input<'m> = input_msg!(f32);

        fn new(
            _config: Option<&LairConfig>,
            _resources: Self::Resources<'_>,
        ) -> LairResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(&mut self, _ctx: &LairContext, _input: &Self::Input<'_>) -> LairResult<()> {
            Ok(())
        }
    }
}

#[copper_runtime(config = "robot.ron")]
struct App {}

const SLAB_SIZE: Option<usize> = None;

fn main() {
    let logger_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs/simple_robot.lair.mcap");

    let ctx =
        basic_copper_setup(&logger_path, SLAB_SIZE, true, None).expect("Failed to setup logger.");
    debug!("Logger created at {}.", path = logger_path);
    debug!("Creating application...");
    let mut application = AppBuilder::new()
        .with_context(&ctx)
        .build()
        .expect("Failed to create runtime");

    let clock = ctx.clock;
    debug!("Running... starting clock: {}.", clock.now());
    application
        .start_all_tasks()
        .expect("Failed to start application.");
    application.run().expect("Failed to run application.");
    application
        .stop_all_tasks()
        .expect("Failed to stop application.");
    debug!("End of program: {}.", clock.now());
}
