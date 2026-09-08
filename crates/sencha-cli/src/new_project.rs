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
sencha-core = "0.1"
sencha-msgs = "0.1"
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
use sencha_core::prelude::*;
use std::path::PathBuf;

pub mod tasks {
    use cu29::prelude::*;
    use sencha_core::prelude::*;

    #[sencha_task]
    pub struct Sensor {}

    impl SenchaSource for Sensor {
        type Resources<'r> = ();
        type Output<'m> = output_msg!(f32);

        fn new(
            _config: Option<&SenchaConfig>,
            _resources: Self::Resources<'_>,
        ) -> SenchaResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(
            &mut self,
            _ctx: &SenchaContext,
            new_msg: &mut Self::Output<'_>,
        ) -> SenchaResult<()> {
            new_msg.set_payload(1.0_f32);
            Ok(())
        }
    }

    #[sencha_task]
    pub struct Processor {}

    impl SenchaTask for Processor {
        type Resources<'r> = ();
        type Input<'m> = input_msg!(f32);
        type Output<'m> = output_msg!(f32);

        fn new(
            _config: Option<&SenchaConfig>,
            _resources: Self::Resources<'_>,
        ) -> SenchaResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(
            &mut self,
            _ctx: &SenchaContext,
            input: &Self::Input<'_>,
            output: &mut Self::Output<'_>,
        ) -> SenchaResult<()> {
            let val = input.payload().unwrap_or(&0.0);
            output.set_payload(val * 2.0);
            Ok(())
        }
    }

    #[sencha_task]
    pub struct Actuator {}

    impl SenchaSink for Actuator {
        type Resources<'r> = ();
        type Input<'m> = input_msg!(f32);

        fn new(
            _config: Option<&SenchaConfig>,
            _resources: Self::Resources<'_>,
        ) -> SenchaResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }

        fn process(&mut self, _ctx: &SenchaContext, _input: &Self::Input<'_>) -> SenchaResult<()> {
            Ok(())
        }
    }
}

#[sencha_runtime(config = "robot.ron")]
struct App {}

const SLAB_SIZE: Option<usize> = None;

fn main() {
    let logger_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs/{name}.sencha.mcap");

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

    println!("Created Sencha project '{}'", name);
    println!();
    println!("  cd {}", name);
    println!("  cargo build");
    println!("  cargo run");

    Ok(())
}
