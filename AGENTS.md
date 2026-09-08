# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Sencha is an experimental robotics systems platform exploring a successor to ROS, built in Rust on Copper. Commercial and industrial robots provide the application setting. Researchers and engineers from the ROS and Copper communities are welcome, and reusable experimental results should benefit both communities.

**Status**: Early implementation. Main contains a Copper-derived runtime, Sencha API/tooling, message types, and direct MCAP logging. Safety and external simulation are stubs on main; additional prototype work exists on another branch. See `RESEARCH.md` for the dated source assessment and `RESEARCH_AGENDA.md` for the research direction and early-2027 paper/artifact target.

The architecture, feature, dependency, and simulation lists below include historical plans. Verify them against Cargo manifests and source before treating them as implemented. Distinguish inherited Copper capabilities, Sencha changes, experimental hypotheses, and validated results.

## Build Commands

```bash
# Build the workspace
cargo build

# Build with release optimizations
cargo build --release

# Check the complete workspace against the lockfile
cargo check --workspace --locked

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Run benchmarks
cargo bench
```

## Workspace Architecture

The project uses a Cargo workspace with the following crates. Descriptions here include planned extensions; check the README and source for current behavior:

- **cu29** (in `crates/sencha`) - Main runtime facade; retains the Copper package name
- **sencha-core** - Core traits (`SenchaSource`, `SenchaTask`, `SenchaSink`), types, and error handling
- **sencha-msgs** - Standard message types (Geometry, Sensors, Navigation) compatible with ROS message conventions
- **sencha-biscuit** - Safety system integration (optional, feature-gated) - validates outputs against physics constraints using DuckDB
- **sencha-bagel** - Logging layer (Parquet/DuckDB format) with NLP query support
- **sencha-derive** - Proc macros (`#[sencha_task]`, `#[sencha_runtime]`)
- **sencha-cli** - Command-line tool (`sencha new`, `sencha build`, `sencha run`, etc.)
- **sencha-isaac** - NVIDIA Isaac Sim integration (optional, feature-gated) - simulation, synthetic data, testing

Future crates (not in current workspace): `sencha-micro` (no_std embedded), `sencha-bevy` (Bevy simulation), `sencha-studio` (Web UI), `sencha-k8s` (Kubernetes operator)

## Core Concepts

### Task System
Tasks are the fundamental computation unit. Three trait types:
- `SenchaSource` - Produces data, no input (sensor drivers)
- `SenchaTask` - Transforms input to output (algorithms)
- `SenchaSink` - Consumes data, no output (actuator drivers)

Tasks have a lifecycle: Unconfigured → Inactive → Active → Inactive → Finalized

### Configuration
Task graphs are defined in RON format (`.ron` files). See `SPEC.md` section 3 for full schema.

### Scheduling
Supports multiple algorithms: `RateMonotonic`, `EarliestDeadlineFirst`, `TimeTriggered`, `DAG`, `Custom`

### Feature Flags
All integrations are optional:
- `biscuit` - LLM safety guards (for service robots, NOT for AV)
- `bagel` - Smart logging (Matcha compatible)
- `matcha` - Cloud sync / fleet management
- `ml` - ML inference runtime
- `isaac` - NVIDIA Isaac Sim Python bridge (v0.3+)
- `isaac-grpc` - Isaac Sim gRPC bridge (v0.4+, production)
- `replay` - Record/replay simulation (v0.1+)
- `bevy-sim` - Bevy-based simulation (future)

## Key Dependencies

- **Async**: tokio, async-trait, futures
- **Serialization**: serde, bincode, ron (for config)
- **Data/Safety**: duckdb, arrow, parquet
- **Math**: nalgebra (linear algebra), uom (units of measure)
- **Zero-copy**: zerocopy, crossbeam, parking_lot
- **Macros**: syn, quote, proc-macro2
- **Simulation**: pyo3 (Isaac Sim bridge), tonic (gRPC), zenoh (transport)

## Simulation

Sencha prioritizes **NVIDIA Isaac Sim** for simulation, aligned with target markets (AV, industrial, agriculture) that already use NVIDIA hardware.

### Simulation Modes

1. **Replay** (v0.1+) - Play back recorded sensor data for testing
2. **Isaac Sim** (v0.3+) - Full physics simulation with photorealistic sensors
3. **Bevy** (future) - Open-source alternative for non-NVIDIA users

### Isaac Sim Requirements

- NVIDIA RTX GPU (RTX 2070+)
- 32GB+ RAM
- Ubuntu 22.04 or Windows 11
- NVIDIA Omniverse + Isaac Sim (free download)

See `SIMULATION.md` for detailed architecture and integration design.

## Design Philosophy

1. Industrial applications, open research - use realistic robots to evaluate new architectures and welcome community participation
2. Reliability > Features - 24/7 uptime over cool demos
3. Fleet-First - design for managing 500 robots, not just one
4. Evidence before claims - state assumptions, compare strong baselines, and publish reproducible results; certification remains a future objective
5. Modular - use what you need via feature flags

## Related Documentation

- `RESEARCH_AGENDA.md` - ROS-successor ambition, research program, and community contributions
- `RESEARCH.md` - Dated implementation assessment and Copper comparison
- `SPEC.md` - Full technical specification with API examples
- `ROADMAP.md` - Release phases and feature priorities
- `NOTES.md` - Design decisions and open questions
