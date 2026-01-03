# LAIR

**Layered Autonomous Intelligence Runtime**

<p align="center">
  <em>A next-generation robotics operating system. Rust-native. Safety-first. Cloud-ready.</em>
</p>

<p align="center">
  <a href="./SPEC.md">Specification</a> •
  <a href="./ROADMAP.md">Roadmap</a> •
  <a href="#quickstart">Quickstart</a> •
  <a href="#why-lair">Why LAIR?</a>
</p>

---

## What is LAIR?

LAIR is a robotics operating system that eliminates the pain points of ROS while preserving the ecosystem's value. Built from scratch in Rust with:

- **🦀 Rust-Native**: Memory safety, thread safety, zero-cost abstractions
- **🛡️ Safety-First**: Physics constraints enforced at runtime (Biscuit integration)
- **🔍 Data Intelligence**: Query your robot logs with natural language (Bagel integration)
- **☸️ Cloud-Ready**: Kubernetes-native, fleet management built-in
- **⚡ Deterministic**: Configurable scheduling algorithms, zero packet loss
- **🔧 Developer-Friendly**: No more `source setup.bash`, visual editor, modern CLI

---

## Why LAIR?

| ROS Pain Point | LAIR Solution |
|----------------|---------------|
| `source setup.bash` in every terminal | Rust modules - zero environment setup |
| MicroROS is a nightmare | Same API for embedded and full runtime |
| Non-deterministic execution | Pluggable deterministic schedulers |
| Weak package management | `lair pkg install` like apt/dnf |
| No built-in safety mechanisms | Biscuit physics guards at runtime level |
| Hard to analyze log files | Bagel NLP queries from day 1 |
| Not cloud-native | Kubernetes operator, Helm charts |

---

## Quickstart

```bash
# Install LAIR CLI
cargo install lair-cli

# Create a new robot project
lair new my_robot --template differential_drive
cd my_robot

# Build and run
lair build
lair run

# Visualize the task graph
lair graph

# Query your logs with natural language
lair log query "what was the maximum velocity?"
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Your Robot Application               │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────┐   │
│  │               BISCUIT SAFETY LAYER              │   │
│  │    Physics constraints enforced automatically    │   │
│  └─────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────┤
│                     LAIR RUNTIME                        │
│  Scheduler • Transforms • Logging • Parameters          │
├─────────────────────────────────────────────────────────┤
│                  COMMUNICATION LAYER                    │
│  Zero-copy message bus • Topics • Services • Actions    │
├─────────────────────────────────────────────────────────┤
│                HARDWARE ABSTRACTION                     │
│  Sensor Traits • Actuator Traits • Driver Framework     │
├─────────────────────────────────────────────────────────┤
│  Linux (Full)  │  lair-micro (Embedded)  │  Simulation │
└─────────────────────────────────────────────────────────┘
```

---

## Example

```rust
use lair::prelude::*;

#[lair::task(period = "100ms")]
pub struct ObstacleDetector;

impl LairTask for ObstacleDetector {
    type Input = LaserScan;
    type Output = ObstacleList;
    
    fn process(
        &mut self, 
        _clock: &Clock,
        scan: &LaserScan, 
        obstacles: &mut ObstacleList
    ) -> LairResult<()> {
        // Your detection logic here
        let detected = detect_obstacles(scan);
        obstacles.set(detected);
        Ok(())
    }
}

#[lair::runtime(config = "robot.ron")]
struct MyRobot;

fn main() -> LairResult<()> {
    MyRobot::run()
}
```

---

## Integrations

LAIR is part of the **Extelligence** ecosystem:

| Project | Description | Integration |
|---------|-------------|-------------|
| **[Bagel](https://github.com/Extelligence-ai/bagel)** | Chat with your robot data | Native logging format |
| **[Biscuit](https://github.com/Extelligence-ai/biscuit)** | Physics-constrained AI | Runtime safety guards |

---

## Status

🚧 **LAIR is in active development** 🚧

See the [ROADMAP.md](./ROADMAP.md) for the full plan.

Current focus: **v0.1 MVP** - Core runtime, basic CLI, safety integration

---

## Project Structure

```
lair/
├── crates/
│   ├── lair/           # Main runtime library
│   ├── lair-core/      # Core traits and types
│   ├── lair-msgs/      # Standard message types
│   ├── lair-biscuit/   # Safety system integration
│   ├── lair-bagel/     # Logging layer
│   ├── lair-derive/    # Proc macros (#[lair::task], etc.)
│   └── lair-cli/       # Command-line tool
├── examples/
│   └── simple_robot/   # Example robot
├── SPEC.md             # Full specification
├── ROADMAP.md          # Development roadmap
└── README.md           # You are here
```

---

## Contributing

We'd love your help! See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

Key areas where we need contributors:
- Core runtime implementation
- Hardware drivers
- Documentation
- Testing

---

## License

Apache 2.0 - See [LICENSE](./LICENSE)

---

<p align="center">
  <strong>LAIR</strong> — Robotics without the pain.
</p>

