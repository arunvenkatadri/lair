use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
cu29 = "0.1"
cu29-helpers = "0.1"
lair-core = "0.1"
lair-msgs = "0.1"
serde = { version = "1.0", features = ["derive"] }
"#;

const BUILD_RS_TEMPLATE: &str = r#"fn main() {
    println!(
        "cargo:rustc-env=LOG_INDEX_DIR={}",
        std::env::var("OUT_DIR").unwrap()
    );
}
"#;

const ROBOT_RON_TEMPLATE: &str = r#"(
    tasks: [
        (
            id: "sensor",
            type: "tasks::Sensor",
        ),
        (
            id: "processor",
            type: "tasks::Processor",
        ),
        (
            id: "actuator",
            type: "tasks::Actuator",
        ),
    ],
    cnx: [
        (
            src: "sensor",
            dst: "processor",
            msg: "f32",
        ),
        (
            src: "processor",
            dst: "actuator",
            msg: "f32",
        ),
    ],
)
"#;

const MAIN_RS_TEMPLATE: &str = r#"use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use lair_core::prelude::*;
use std::path::PathBuf;

pub mod tasks {
    use cu29::prelude::*;
    use lair_core::prelude::*;

    pub struct Sensor {}

    impl Freezable for Sensor {}

    impl LairSource for Sensor {
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

    pub struct Actuator {}

    impl Freezable for Actuator {}

    impl LairSink for Actuator {
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
    let logger_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs/{name}.lair.mcap");

    let ctx =
        basic_copper_setup(&logger_path, SLAB_SIZE, true, None).expect("Failed to setup logger.");
    let mut application = AppBuilder::new()
        .with_context(&ctx)
        .build()
        .expect("Failed to create runtime");

    application
        .start_all_tasks()
        .expect("Failed to start application.");
    application.run().expect("Failed to run application.");
    application
        .stop_all_tasks()
        .expect("Failed to stop application.");
}
"#;

pub fn create_project(name: &str) -> Result<()> {
    let project_dir = Path::new(name);

    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    fs::create_dir_all(project_dir.join("src"))
        .with_context(|| format!("Failed to create {}/src", name))?;

    fs::write(
        project_dir.join("Cargo.toml"),
        CARGO_TOML_TEMPLATE.replace("{name}", name),
    )
    .context("Failed to write Cargo.toml")?;

    fs::write(project_dir.join("build.rs"), BUILD_RS_TEMPLATE).context("Failed to write build.rs")?;

    fs::write(project_dir.join("robot.ron"), ROBOT_RON_TEMPLATE)
        .context("Failed to write robot.ron")?;

    fs::write(
        project_dir.join("src/main.rs"),
        MAIN_RS_TEMPLATE.replace("{name}", name),
    )
    .context("Failed to write src/main.rs")?;

    println!("Created LAIR project '{}'", name);
    println!();
    println!("  cd {}", name);
    println!("  cargo build");
    println!("  cargo run");

    Ok(())
}
