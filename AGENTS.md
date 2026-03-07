# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

LAIR (Layered Autonomous Intelligence Runtime) is a Rust-native robotics operating system targeting commercial/industrial deployments (autonomous vehicles, warehouses, agriculture, commercial robotics). It is NOT for research, academia, or hobbyists.

**Status**: Design phase - specifications exist but implementation has not started.

## Build Commands

```bash
# Build the workspace (once crates are implemented)
cargo build

# Build with release optimizations
cargo build --release

# Build with specific features
cargo build --features bagel,matcha          # For AV, Industrial
cargo build --features biscuit,bagel,matcha  # For service robots
cargo build --features isaac                 # With Isaac Sim support
cargo build --no-default-features            # Minimal/embedded

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Run benchmarks
cargo bench
```

## Workspace Architecture

The project uses a Cargo workspace with these planned crates:

- **lair** - Main runtime library (re-exports core functionality)
- **lair-core** - Core traits (`LairSource`, `LairTask`, `LairSink`), types, and error handling
- **lair-msgs** - Standard message types (Geometry, Sensors, Navigation) compatible with ROS message conventions
- **lair-biscuit** - Safety system integration (optional, feature-gated) - validates outputs against physics constraints using DuckDB
- **lair-bagel** - Logging layer (Parquet/DuckDB format) with NLP query support
- **lair-derive** - Proc macros (`#[lair::task]`, `#[lair::runtime]`, `#[lair::action]`)
- **lair-cli** - Command-line tool (`lair new`, `lair build`, `lair run`, etc.)
- **lair-isaac** - NVIDIA Isaac Sim integration (optional, feature-gated) - simulation, synthetic data, testing

Future crates (not in current workspace): `lair-micro` (no_std embedded), `lair-bevy` (Bevy simulation), `lair-studio` (Web UI), `lair-k8s` (Kubernetes operator)

## Core Concepts

### Task System
Tasks are the fundamental computation unit. Three trait types:
- `LairSource` - Produces data, no input (sensor drivers)
- `LairTask` - Transforms input to output (algorithms)
- `LairSink` - Consumes data, no output (actuator drivers)

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

LAIR prioritizes **NVIDIA Isaac Sim** for simulation, aligned with target markets (AV, industrial, agriculture) that already use NVIDIA hardware.

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

1. Commercial/Industrial ONLY - build for production, not research
2. Reliability > Features - 24/7 uptime over cool demos
3. Fleet-First - design for managing 500 robots, not just one
4. Certification-Ready - design for ISO 26262 from day one
5. Modular - use what you need via feature flags

## Related Documentation

- `SPEC.md` - Full technical specification with API examples
- `ROADMAP.md` - Release phases and feature priorities
- `NOTES.md` - Design decisions and open questions
