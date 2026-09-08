# Sencha — Experimental Robotics Systems

<p align="center">
  <em>Exploring a successor to ROS. Built in Rust on <a href="https://github.com/copper-project/copper-rs">Copper</a>.</em>
</p>

<p align="center">
  <a href="./SPEC.md">Specification</a> &bull;
  <a href="./SIMULATION.md">Simulation</a> &bull;
  <a href="./ROADMAP.md">Roadmap</a> &bull;
  <a href="./RESEARCH.md">Research assessment</a> &bull;
  <a href="./RESEARCH_AGENDA.md">Research agenda</a> &bull;
  <a href="#quickstart">Quickstart</a>
</p>

---

> **Early development — v0.1-beta.** Sencha contains a Copper-derived runtime, API wrappers, CLI, and message types. Safety validation and external simulation are stubs on the main branch. The research contributions and migration path described below are objectives to investigate. See [What's in v0.1-beta](#whats-in-v01-beta) for implementation status.

---

**Sencha is an experimental robotics systems platform exploring a successor to ROS.** Built on Copper, it develops and evaluates new execution architectures, with reusable results for the ROS and Copper communities.

Formerly LAIR, Sencha is named after the tea its founders drink every morning. See the [rename notes](./MIGRATING_TO_SENCHA.md) for package, API, and CLI changes.

**Built on [Copper](https://github.com/copper-project/copper-rs).** The DAG scheduler, message-passing machinery, clock, unified logging infrastructure, and task execution model come from the Copper robotics framework created by [Gbin](https://github.com/gbin) and the Copper contributors. Sencha maintains a fork for experimentation, adds its own API/tooling and direct MCAP backend, and explores further architectural changes. See [Acknowledgments](#acknowledgments).

Our long-term ambition is to replace ROS as the foundation of a robot application. Our research obligation is to demonstrate which architectural changes improve robot behavior, predictability, or reliability, and under what assumptions. Industrial robots provide the application setting; researchers and engineers from both communities are welcome to participate.

### Research direction

We are investigating how end-to-end execution requirements—such as observation freshness, timing, and fault response—could become part of the robot program and be checked when components are composed. The first contribution will be selected through comparison with Copper and existing ROS-based approaches; a compiled graph or a Rust implementation alone is not a novelty claim.

The target for **early 2027** is a paper and reproducible artifact demonstrating one consequential architectural contribution on a complete robot application. Evaluation should include strong baselines, physical outcomes, limitations, and an example of incremental migration from ROS. This is a research target, not a stable-release or paper-acceptance commitment.

Useful results should travel: we intend to share methods, benchmarks, negative results, and implementation improvements that Copper and ROS contributors can adopt independently. Upstream adoption is a successful outcome. Read the [research agenda](./RESEARCH_AGENDA.md) and [source-based comparison with Copper](./RESEARCH.md).

### What you get

- **Graph-based execution** &mdash; Copper's dependency-ordered task execution
- **In-process message passing** &mdash; Copper's message and payload infrastructure
- **Compile-time graph validation** &mdash; wiring errors caught at `cargo build`, not at 2 AM in the field
- **MCAP recording** &mdash; a direct backend for unified log sections; payload decoding and complete state restoration are separate requirements
- **Mockable clock** &mdash; nanosecond-precision monotonic clock with mock support for deterministic testing
- **Rust-native** &mdash; Copper includes embedded `no_std` support; Sencha's full package/feature combinations need separate validation

Performance and replay guarantees must be evaluated for the actual Sencha configuration. In particular, the direct MCAP backend changes the logging path, and stateful tasks require explicit snapshot/restore implementations.

---

## Why Sencha?

### Coming from ROS

| Area to evaluate | Sencha's starting point |
|------------------|-----------------------|
| Application model | Rust tasks composed through a RON graph |
| Execution and wiring | Copper's dependency ordering and generated runtime |
| Data movement | Copper's in-process message/payload model |
| Recording | Unified log infrastructure with a direct MCAP backend |
| Migration | Planned incremental migration of bounded application paths, preserving useful ROS components where possible |

ROS has existing work on deterministic execution and timing analysis. Our experiments must compare against those approaches as well as ordinary deployments; see the [research agenda and related work](./RESEARCH_AGENDA.md). ROS compatibility and a complete migration workflow are research objectives, not implemented capabilities of this release.

### Coming from Copper

| Area | Sencha Addition | Status |
|------------|---------------|--------|
| API vocabulary | Sencha aliases/re-exports plus `#[sencha_task]` and `#[sencha_runtime]` wrappers | Implemented |
| Project tooling | Sencha-specific `new` / `build` / `run` / `doctor` commands | Implemented; public scaffolding workflow needs validation |
| Message bundle | `sencha-msgs`: geometry, sensors, navigation, vehicle | Implemented |
| Log storage | Direct MCAP backend and Sencha logging helper | Implemented; performance needs measurement |
| Actuator validation | `SafetyValidator` interface | Stub on main; prototype work on a separate branch |
| External simulation | `SimBridge` interface | Mock implementation |
| Execution research | Composition and enforcement of system-level requirements | Proposed; contribution not yet demonstrated |

Copper has its own prelude, project templates, simulation support, and safety monitoring. Sencha's additions above do not imply those capabilities are absent upstream. We want experiments here to be useful to Copper contributors as well as Sencha users.

---

## Quickstart

Use the source checkout for this early build. Install a current stable Rust toolchain and your platform's native compiler/linker, then:

```bash
git clone https://github.com/arunvenkatadri/sencha.git
cd sencha
cargo check --workspace --locked
cargo run --locked -p sencha-cli -- --help
cargo run --locked -p simple-robot
```

The example runs a synthetic sensor → processor → placeholder sink graph. It does not drive hardware. Stop it with Ctrl+C; its log is written under `examples/simple_robot/logs/`.

The CLI includes project scaffolding, but its generated registry dependencies still need to be aligned with this fork before it is a supported standalone setup path. Use [`examples/simple_robot`](./examples/simple_robot) as the source-based starting point.

## Example

A Sencha transform task uses Copper's message and lifecycle interfaces under Sencha names:

```rust
use cu29::prelude::*;

pub struct Processor;

// This task has no persistent state to snapshot.
impl Freezable for Processor {}

impl SenchaTask for Processor {
    type Resources<'r> = ();
    type Input<'m> = input_msg!(f32);
    type Output<'m> = output_msg!(f32);

    fn new(_config: Option<&SenchaConfig>, _resources: Self::Resources<'_>) -> SenchaResult<Self> {
        Ok(Self)
    }

    fn process(
        &mut self,
        _ctx: &SenchaContext,
        input: &Self::Input<'_>,
        output: &mut Self::Output<'_>,
    ) -> SenchaResult<()> {
        output.set_payload(input.payload().copied().unwrap_or(0.0) * 2.0);
        Ok(())
    }
}
```

See the [complete example](./examples/simple_robot/src/main.rs) and [RON graph](./examples/simple_robot/robot.ron) for runtime wiring. Stateful tasks must implement snapshot/restore; the empty `Freezable` default does not save their fields.

---

## Architecture

```
+--------------------------------------------------------------+
|                    Your Robot Application                    |
+--------------------------------------------------------------+
|       Biscuit interface (stub; no enforced validation)       |
+--------------------------------------------------------------+
|                          Sencha API                          |
+--------------------------------------------------------------+
|    SenchaSource | SenchaTask | SenchaSink | SenchaContext    |
+--------------------------------------------------------------+
|                    Copper Runtime Engine                     |
+--------------------------------------------------------------+
|      DAG Scheduler | Message Passing | Logging | Clock       |
+--------------------------------------------------------------+
|                    Hardware / Simulation                     |
+--------------------------------------------------------------+
|     Copper platform support | External simulation: mock      |
+--------------------------------------------------------------+
```

---

## What's in v0.1-beta

This table describes the main-branch implementation. Implemented code is not a claim of production readiness or a validated research result.

| Area | Status | Notes |
|------|--------|-------|
| DAG scheduler, zero-copy bus, clock, monitoring | **Implemented** | Inherited Copper engine; Sencha-specific performance needs measurement |
| Sencha API (`SenchaSource`, `SenchaTask`, `SenchaSink`, etc.) | **Implemented** | Type aliases + trait re-exports over Copper |
| Message types (geometry, sensors, navigation, vehicle) | **Implemented** | Serialization roundtrip tests included |
| CLI (`sencha new`, `build`, `run`, `doctor`) | **Implemented** | Cargo wrappers and system checks; scaffolding dependency setup needs work |
| Proc macros (`#[sencha_task]`, `#[sencha_runtime]`) | **Implemented** | `sencha_task` auto-impls Freezable; `sencha_runtime` delegates to Copper |
| Safety validation | **Stub** | `SafetyValidator` trait defined, `NoOpSafetyValidator` passes everything |
| Simulation bridge | **Stub** | `SimBridge` trait defined, `MockSimBridge` is in-memory only |
| Fleet management, cloud sync | Not started | Future work |
| Full Isaac Sim integration | Not started | Future work |
| Certification tooling | Not started | Future work |

The main branch defines a safety validation interface but does not enforce routing through it before actuation. A separate prototype branch contains additional safety and replay work; the [research assessment](./RESEARCH.md) records its scope and limitations.

---

## Project Structure

```
sencha/
+-- crates/
|   +-- sencha/           # Runtime facade (Cargo package: cu29)
|   +-- sencha-core/      # Sencha API types (SenchaSource, SenchaTask, SenchaSink, ...)
|   +-- sencha-derive/    # Proc macros (#[sencha_task], #[sencha_runtime])
|   +-- sencha-msgs/      # Standard message types (geometry, sensors, navigation, vehicle)
|   +-- sencha-biscuit/   # Safety validation (SafetyValidator trait)
|   +-- sencha-bagel/     # Structured logging and telemetry (MCAP)
|   +-- sencha-isaac/     # Simulation bridge interface (SimBridge trait)
|   +-- sencha-cli/       # CLI tool (sencha new, build, run, doctor)
|   +-- cu29-*/         # Copper runtime engine (forked, internal)
+-- examples/
|   +-- simple_robot/   # Minimal source -> task -> sink example
+-- SPEC.md             # Full specification
+-- SIMULATION.md       # Simulation architecture
+-- ROADMAP.md          # Development roadmap
```

---

## Integrations

Sencha is part of the **Extelligence** ecosystem:

| Project | Description | v0.1-beta Status |
|---------|-------------|------------------|
| **[Bagel](https://github.com/Extelligence-ai/bagel)** | Chat with your robot data | Direct MCAP recording; no query integration in this crate |
| **[Biscuit](https://github.com/Extelligence-ai/biscuit)** | Physics-constrained AI safety | Coming soon |
| **[Matcha](https://github.com/Extelligence-ai/matcha)** | Cloud fleet management &amp; monitoring | Coming soon |
| **[NVIDIA Isaac Sim](https://developer.nvidia.com/isaac/sim)** | Robotics simulation | Trait defined, mock impl |

---

## For Contributors

This README is aimed at both **users** (robotics engineers evaluating Sencha) and **contributors** (developers building on or extending it).

We welcome ROS and Copper contributors, robotics researchers, and engineers working on industrial systems. Useful contributions include reproducible failure cases, baseline implementations, robot experiments, architecture reviews, and improvements suitable for upstream adoption.

If you're contributing:
- The runtime engine lives in `crates/cu29-*` &mdash; this is forked Copper, modify carefully
- The Sencha API layer lives in `crates/sencha-*` &mdash; this is where most new work happens
- `cargo check` must pass on the full workspace before submitting changes
- `cargo test --workspace --exclude cu29-base-derive --exclude cu29-clock` runs the stable test suite

---

## Acknowledgments

Sencha's runtime engine is a fork of **[Copper](https://github.com/copper-project/copper-rs)** (cu29), created by **[Gbin](https://github.com/gbin)** and the Copper contributors. Copper provides the foundational systems that make Sencha possible:

- **Deterministic DAG scheduler** &mdash; executes task graphs in dependency order
- **Zero-copy message bus** &mdash; lock-free inter-task communication
- **Unified logging infrastructure** &mdash; structured logs, messages, and task snapshots; Sencha adds a direct MCAP backend
- **`#[copper_runtime]` proc macro** &mdash; compile-time task graph validation and code generation
- **Monotonic clock** &mdash; high-precision, mockable `RobotClock`
- **`Freezable` trait** &mdash; task state snapshot/restore for deterministic replay
- **`no_std` support** &mdash; bare-metal execution on embedded targets
- **Simulation infrastructure** &mdash; simulated clocks, simulated hardware bridges

Copper is licensed under the Apache License 2.0. The original license is preserved in [`LICENSE-COPPER`](./LICENSE-COPPER). We are grateful to the Copper community for building such a solid foundation for real-time robotics in Rust.

---

## License

Apache 2.0, as declared in the workspace manifest. The Apache license text and Copper notices are preserved in [LICENSE-COPPER](./LICENSE-COPPER).

---

<p align="center">
  <strong>Sencha</strong> &mdash; Robotics systems research, built on Copper.
</p>
