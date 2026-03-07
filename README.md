# LAIR

**Layered Autonomous Intelligence Runtime**

<p align="center">
  <em>A production robotics runtime. Rust-native. Safety-first. Deterministic.</em>
</p>

<p align="center">
  <a href="./SPEC.md">Specification</a> &bull;
  <a href="./SIMULATION.md">Simulation</a> &bull;
  <a href="./ROADMAP.md">Roadmap</a> &bull;
  <a href="#quickstart">Quickstart</a> &bull;
  <a href="#why-lair">Why LAIR?</a>
</p>

---

## What is LAIR?

LAIR is a robotics runtime for commercial and industrial deployments. It provides a clean, safety-oriented API on top of a battle-tested execution engine, targeting autonomous vehicles, warehouse robots, agricultural machines, and industrial automation.

**Built on [Copper](https://github.com/copper-project/copper-rs).** LAIR's runtime engine &mdash; the DAG scheduler, zero-copy message bus, deterministic clock, unified logging, and task execution model &mdash; is a maintained fork of the Copper robotics framework created by Gbin and the Copper contributors. Copper did the hard systems work; LAIR builds a production API, safety layer, and toolchain on top. See [Acknowledgments](#acknowledgments) for details.

### Core capabilities

- **Deterministic execution** &mdash; DAG scheduler executes tasks in dependency order with sub-microsecond overhead
- **Zero-copy message bus** &mdash; lock-free, bounded queues between tasks; no serialization on the hot path
- **Unified logging** &mdash; every message, every state snapshot, captured in MCAP format for replay and analysis
- **Compile-time graph validation** &mdash; task wiring errors are caught at `cargo build`, not at 2 AM in the field
- **Rust-native** &mdash; memory safety, thread safety, `no_std` support for embedded targets
- **Mockable clock** &mdash; nanosecond-precision monotonic clock with mock support for deterministic testing

---

## Why LAIR?

| ROS Pain Point | LAIR Solution |
|----------------|---------------|
| `source setup.bash` in every terminal | Rust modules &mdash; zero environment setup |
| Non-deterministic execution | Deterministic DAG scheduler, compile-time graph validation |
| Serialization overhead on message bus | Zero-copy message passing between tasks |
| Hard to replay and analyze logs | Unified MCAP logging with deterministic replay |
| Weak safety guarantees | `SafetyValidator` trait, Biscuit physics guards (optional) |
| No embedded story (MicroROS) | Same task traits for `std` and `no_std` targets |

---

## Quickstart

```bash
# Install LAIR CLI
cargo install lair-cli

# Create a new robot project
lair new my_robot
cd my_robot

# Build and run
lair build
lair run

# Check system requirements
lair doctor
```

---

## Example

A minimal LAIR pipeline: sensor source &rarr; processor &rarr; actuator sink.

```rust
use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use lair_core::prelude::*;
use std::path::PathBuf;

// ── Source: produces sensor readings ──

#[lair_task]
pub struct Sensor {}

impl LairSource for Sensor {
    type Resources<'r> = ();
    type Output<'m> = output_msg!(f32);

    fn new(_config: Option<&LairConfig>, _resources: Self::Resources<'_>) -> LairResult<Self>
    where Self: Sized {
        Ok(Self {})
    }

    fn process(&mut self, _ctx: &LairContext, out: &mut Self::Output<'_>) -> LairResult<()> {
        out.set_payload(1.0_f32);
        Ok(())
    }
}

// ── Task: transforms input to output ──

#[lair_task]
pub struct Processor {}

impl LairTask for Processor {
    type Resources<'r> = ();
    type Input<'m> = input_msg!(f32);
    type Output<'m> = output_msg!(f32);

    fn new(_config: Option<&LairConfig>, _resources: Self::Resources<'_>) -> LairResult<Self>
    where Self: Sized {
        Ok(Self {})
    }

    fn process(
        &mut self, _ctx: &LairContext,
        input: &Self::Input<'_>, output: &mut Self::Output<'_>,
    ) -> LairResult<()> {
        let val = input.payload().unwrap_or(&0.0);
        output.set_payload(val * 2.0);
        Ok(())
    }
}

// ── Sink: consumes commands ──

#[lair_task]
pub struct Actuator {}

impl LairSink for Actuator {
    type Resources<'r> = ();
    type Input<'m> = input_msg!(f32);

    fn new(_config: Option<&LairConfig>, _resources: Self::Resources<'_>) -> LairResult<Self>
    where Self: Sized {
        Ok(Self {})
    }

    fn process(&mut self, _ctx: &LairContext, _input: &Self::Input<'_>) -> LairResult<()> {
        Ok(())
    }
}

// ── Runtime wiring (validated at compile time) ──

#[lair_runtime(config = "robot.ron")]
struct App {}

fn main() {
    let logger_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs/app.lair.mcap");
    let ctx = basic_copper_setup(&logger_path, None, true, None)
        .expect("Failed to setup logger");

    let mut app = AppBuilder::new()
        .with_context(&ctx)
        .build()
        .expect("Failed to create runtime");

    app.start_all_tasks().expect("Failed to start");
    app.run().expect("Failed to run");
    app.stop_all_tasks().expect("Failed to stop");
}
```

The task graph is defined in `robot.ron`:

```ron
(
    tasks: [
        ( id: "sensor",    type: "Sensor" ),
        ( id: "processor", type: "Processor" ),
        ( id: "actuator",  type: "Actuator" ),
    ],
    cnx: [
        ( src: "sensor",    dst: "processor", msg: "f32" ),
        ( src: "processor", dst: "actuator",  msg: "f32" ),
    ],
)
```

See [`examples/simple_robot`](./examples/simple_robot) for the full working version.

---

## Architecture

```
+-----------------------------------------------------------+
|                    Your Robot Application                  |
+-----------------------------------------------------------+
|  +-----------------------------------------------------+  |
|  |              BISCUIT SAFETY LAYER (optional)         |  |
|  |     SafetyValidator trait, physics constraints       |  |
|  +-----------------------------------------------------+  |
+-----------------------------------------------------------+
|                       LAIR API                            |
|  LairSource, LairTask, LairSink, LairContext, LairMsg    |
+-----------------------------------------------------------+
|                  COPPER RUNTIME ENGINE                    |
|  DAG Scheduler | Zero-Copy Bus | MCAP Logging | Clock    |
+-----------------------------------------------------------+
|               HARDWARE / SIMULATION                       |
|  Linux (Full)  |  no_std (Embedded)  |  Isaac Sim Bridge  |
+-----------------------------------------------------------+
```

---

## Project Structure

```
lair/
+-- crates/
|   +-- lair/           # Main prelude crate (re-exports everything)
|   +-- lair-core/      # LAIR API types (LairSource, LairTask, LairSink, ...)
|   +-- lair-derive/    # Proc macros (#[lair_task], #[lair_runtime])
|   +-- lair-msgs/      # Standard message types (geometry, sensors, navigation, vehicle)
|   +-- lair-biscuit/   # Safety validation (SafetyValidator trait)
|   +-- lair-bagel/     # Structured logging and telemetry (MCAP)
|   +-- lair-isaac/     # Simulation bridge interface (SimBridge trait)
|   +-- lair-cli/       # CLI tool (lair new, build, run, doctor)
|   +-- cu29-*/         # Copper runtime engine (forked, internal)
+-- examples/
|   +-- simple_robot/   # Minimal source -> task -> sink example
+-- SPEC.md             # Full specification
+-- SIMULATION.md       # Simulation architecture
+-- ROADMAP.md          # Development roadmap
```

---

## Integrations

LAIR is part of the **Extelligence** ecosystem:

| Project | Description | Status |
|---------|-------------|--------|
| **[Bagel](https://github.com/Extelligence-ai/bagel)** | Chat with your robot data | Logging integration (MCAP) |
| **[Biscuit](https://github.com/Extelligence-ai/biscuit)** | Physics-constrained AI safety | `SafetyValidator` trait defined |
| **[NVIDIA Isaac Sim](https://developer.nvidia.com/isaac/sim)** | Robotics simulation | `SimBridge` trait defined |

---

## Status

LAIR is in **closed beta**. The core runtime works. The LAIR API surface, CLI, message types, and integration stubs are implemented. See the [ROADMAP.md](./ROADMAP.md) for what's next.

| Area | Status |
|------|--------|
| DAG scheduler, zero-copy bus, clock, monitoring | Working (Copper engine) |
| LAIR API (`LairSource`, `LairTask`, `LairSink`, etc.) | Working |
| Message types (geometry, sensors, navigation, vehicle) | Working, tested |
| CLI (`lair new`, `lair build`, `lair run`, `lair doctor`) | Working |
| Proc macros (`#[lair_task]`, `#[lair_runtime]`) | Working |
| Safety validation (`SafetyValidator` trait) | Trait + no-op stub |
| Simulation bridge (`SimBridge` trait) | Trait + mock stub |
| Fleet management, cloud sync | Not started (v0.2) |
| Isaac Sim full integration | Not started (v0.3) |
| Certification tooling | Not started (v0.3) |

---

## Acknowledgments

LAIR's runtime engine is a fork of **[Copper](https://github.com/copper-project/copper-rs)** (cu29), created by **Gbin** and the Copper contributors. Copper provides the foundational systems that make LAIR possible:

- **Deterministic DAG scheduler** &mdash; executes task graphs in dependency order
- **Zero-copy message bus** &mdash; lock-free inter-task communication
- **Unified logging** &mdash; MCAP-based structured logging with deterministic replay
- **`#[copper_runtime]` proc macro** &mdash; compile-time task graph validation and code generation
- **Monotonic clock** &mdash; high-precision, mockable `RobotClock`
- **`Freezable` trait** &mdash; task state snapshot/restore for deterministic replay
- **`no_std` support** &mdash; bare-metal execution on embedded targets
- **Simulation infrastructure** &mdash; simulated clocks, simulated hardware bridges

Copper is licensed under the Apache License 2.0. The original license is preserved in [`LICENSE-COPPER`](./LICENSE-COPPER). We are grateful to the Copper community for building such a solid foundation for real-time robotics in Rust.

---

## License

Apache 2.0 &mdash; See [LICENSE](./LICENSE)

The Copper runtime components are also Apache 2.0 &mdash; See [LICENSE-COPPER](./LICENSE-COPPER)

---

<p align="center">
  <strong>LAIR</strong> &mdash; Production robotics, built on Copper.
</p>
