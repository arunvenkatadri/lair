# LAIR OS

<p align="center">
  <em>Layered AI and Robotics OS. A deterministic robotics runtime for production deployments. Built in Rust on <a href="https://github.com/copper-project/copper-rs">Copper</a>.</em>
</p>

<p align="center">
  <a href="./SPEC.md">Specification</a> &bull;
  <a href="./SIMULATION.md">Simulation</a> &bull;
  <a href="./ROADMAP.md">Roadmap</a> &bull;
  <a href="#quickstart">Quickstart</a>
</p>

---

> **v0.1-beta** &mdash; LAIR is in closed beta. The runtime, API surface, CLI, message types, and the physics-constrained safety validator work. Simulation is architecturally defined but not yet enforced &mdash; the trait exists, the implementation is a stub. See [What's in v0.1-beta](#whats-in-v01-beta) for exactly what ships and what doesn't.

---

LAIR is a robotics runtime for commercial and industrial deployments &mdash; autonomous vehicles, warehouse robots, agricultural machines, industrial automation. It provides a clean API on top of a battle-tested execution engine, with an architecture designed for safety-critical systems from day one.

**Built on [Copper](https://github.com/copper-project/copper-rs).** The runtime engine &mdash; DAG scheduler, zero-copy message bus, deterministic clock, unified logging, task execution model &mdash; is a maintained fork of the Copper robotics framework created by [Gbin](https://github.com/gbin) and the Copper contributors. Copper did the hard systems work; LAIR builds a production API, safety layer, and toolchain on top. See [Acknowledgments](#acknowledgments).

### What you get

- **Deterministic execution** &mdash; DAG scheduler, dependency-ordered, sub-microsecond overhead
- **Zero-copy message bus** &mdash; lock-free bounded queues; no serialization on the hot path
- **Compile-time graph validation** &mdash; wiring errors caught at `cargo build`, not at 2 AM in the field
- **Unified logging** &mdash; every message and state snapshot in MCAP format for replay and analysis
- **Mockable clock** &mdash; nanosecond-precision monotonic clock with mock support for deterministic testing
- **Rust-native** &mdash; memory safety, thread safety; runtime and core traits compile for `no_std` embedded targets

---

## Why LAIR?

| ROS Pain Point | LAIR Solution |
|----------------|---------------|
| `source setup.bash` in every terminal | Rust modules &mdash; zero environment setup |
| Non-deterministic execution | Deterministic DAG scheduler, compile-time graph validation |
| Serialization overhead on message bus | Zero-copy message passing between tasks |
| Hard to replay and analyze logs | Unified MCAP logging with deterministic replay |
| No embedded story (MicroROS) | Runtime and traits compile for `no_std`; no separate embedded API |

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
|  |   PhysicsSafetyValidator: bounds + vehicle dynamics  |  |
|  +-----------------------------------------------------+  |
+-----------------------------------------------------------+
|                       LAIR API                            |
|  LairSource, LairTask, LairSink, LairContext, LairMsg    |
+-----------------------------------------------------------+
|                  COPPER RUNTIME ENGINE                    |
|  DAG Scheduler | Zero-Copy Bus | MCAP Logging | Clock    |
+-----------------------------------------------------------+
|               HARDWARE / SIMULATION                       |
|  Linux / macOS / Windows  |  no_std  |  Isaac Sim Bridge  |
+-----------------------------------------------------------+
```

---

## What's in v0.1-beta

This is an honest accounting. "Working" means you can build against it today. "Stub" means the trait is defined and the architecture is in place, but the implementation is a passthrough.

| Area | Status | Notes |
|------|--------|-------|
| DAG scheduler, zero-copy bus, clock, monitoring | **Working** | Copper engine, battle-tested |
| LAIR API (`LairSource`, `LairTask`, `LairSink`, etc.) | **Working** | Type aliases + trait re-exports over Copper |
| Message types (geometry, sensors, navigation, vehicle) | **Working** | Serialization roundtrip tested |
| CLI (`lair new`, `build`, `run`, `doctor`) | **Working** | Project scaffolding, build/run wrappers, system checks |
| Proc macros (`#[lair_task]`, `#[lair_runtime]`) | **Working** | `lair_task` auto-impls Freezable; `lair_runtime` delegates to Copper |
| Safety validation | **Working** | `PhysicsSafetyValidator` enforces actuator bounds, throttle/brake exclusion, speed-dependent steering (rollover), gear-change and speed limits; `enforce`/`safe_stop` for graceful degradation. Call it from your control sink &mdash; not yet auto-inserted into the DAG. |
| Simulation bridge | **Stub** | `SimBridge` trait defined, `MockSimBridge` is in-memory only |
| Fleet management, cloud sync | Not started | Planned for v0.2 |
| Full Isaac Sim integration | Not started | Planned for v0.3 |
| Certification tooling | Not started | Planned for v0.3 |

The safety architecture is designed into LAIR from the ground up &mdash; every control command flows through a `SafetyValidator` before reaching actuators. As of v0.1-beta that validator is real: `PhysicsSafetyValidator` in [`lair-biscuit`](./crates/lair-biscuit) rejects commands that violate actuator bounds or the vehicle's dynamic envelope (e.g. steering hard enough to roll at speed), and can clamp commands back into the safe envelope or issue a controlled stop for limp-home behavior. What's still coming: automatic insertion of the validator into the task graph so it can't be bypassed by construction.

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

| Project | Description | v0.1-beta Status |
|---------|-------------|------------------|
| **[Bagel](https://github.com/Extelligence-ai/bagel)** | Chat with your robot data | MCAP logging wired |
| **[Biscuit](https://github.com/Extelligence-ai/biscuit)** | Physics-constrained AI safety | `PhysicsSafetyValidator` shipped |
| **[Matcha](https://github.com/Extelligence-ai/matcha)** | Cloud fleet management &amp; monitoring | Coming soon |
| **[NVIDIA Isaac Sim](https://developer.nvidia.com/isaac/sim)** | Robotics simulation | Trait defined, mock impl |

---

## For Contributors

This README is aimed at both **users** (robotics engineers evaluating LAIR) and **contributors** (developers building on or extending it).

If you're contributing:
- The runtime engine lives in `crates/cu29-*` &mdash; this is forked Copper, modify carefully
- The LAIR API layer lives in `crates/lair-*` &mdash; this is where most new work happens
- `cargo check` must pass on the full workspace before submitting changes
- `cargo test --workspace --exclude cu29-base-derive --exclude cu29-clock` runs the stable test suite

---

## Acknowledgments

LAIR's runtime engine is a fork of **[Copper](https://github.com/copper-project/copper-rs)** (cu29), created by **[Gbin](https://github.com/gbin)** and the Copper contributors. Copper provides the foundational systems that make LAIR possible:

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
