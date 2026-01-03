# LAIR Specification
## Layered Autonomous Intelligence Runtime

**Version**: 0.1.0-draft  
**Status**: Design Phase  
**Last Updated**: January 2026

---

## Executive Summary

LAIR is a next-generation robotics operating system built from scratch in Rust. It takes inspiration from the best ideas in Copper (deterministic execution), integrates Biscuit (physics-constrained safety) and Bagel (data intelligence) as first-class citizens, and adds everything missing to create a complete robotics platform.

### Why LAIR Exists

| Problem with ROS | LAIR Solution |
|------------------|---------------|
| `source setup.bash` nightmare | Rust modules, zero env config |
| MicroROS is painful | Same API for micro and full runtime |
| Non-deterministic scheduling | Configurable deterministic schedulers |
| Package management is weak | `lair pkg install` like apt/dnf |
| No built-in safety | Biscuit physics guards at runtime level |
| Hard to query logs | Bagel NLP integration from day 1 |
| Not cloud-native | Kubernetes operator, cloud-first |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            LAIR ARCHITECTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         CLOUD LAYER                                  │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │   K8s    │ │  Fleet   │ │  Model   │ │   OTA    │ │  Bagel   │  │   │
│  │  │ Operator │ │ Manager  │ │ Registry │ │ Updates  │ │  Cloud   │  │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      APPLICATION LAYER                               │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐               │   │
│  │  │   Nav    │ │Perception│ │ Planning │ │  User    │               │   │
│  │  │  Stack   │ │  Stack   │ │  Stack   │ │  Tasks   │               │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘               │   │
│  │                     ▼ BISCUIT SAFETY LAYER ▼                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      RUNTIME LAYER                                   │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │Scheduler │ │Transform │ │ Actions  │ │  Params  │ │Lifecycle │  │   │
│  │  │  Engine  │ │   Tree   │ │  Server  │ │  Server  │ │ Manager  │  │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │  Clock   │ │ Logging  │ │Diagnostics│ │  Safety  │ │ Metrics  │  │   │
│  │  │  System  │ │ (Bagel)  │ │          │ │(Biscuit) │ │          │  │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    COMMUNICATION LAYER                               │   │
│  │  ┌─────────────────────────────────────────────────────────────┐   │   │
│  │  │            Message Bus (Zero-Copy, Lock-Free)                │   │   │
│  │  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐    │   │   │
│  │  │  │ Topics │ │Services│ │Actions │ │ Zenoh  │ │  gRPC  │    │   │   │
│  │  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘    │   │   │
│  │  └─────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                 HARDWARE ABSTRACTION LAYER                           │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │ Sensors  │ │Actuators │ │   CAN    │ │ EtherCAT │ │   GPIO   │  │   │
│  │  │  Traits  │ │  Traits  │ │  Bridge  │ │  Bridge  │ │          │  │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      PLATFORM LAYER                                  │   │
│  │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐       │   │
│  │  │  Linux (Full)   │ │   lair-micro    │ │   Simulation    │       │   │
│  │  │  x86/ARM/RISC-V │ │  STM32/ESP32/RP │ │   Bevy/Gazebo   │       │   │
│  │  └─────────────────┘ └─────────────────┘ └─────────────────┘       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Concepts

### 1. Tasks

The fundamental unit of computation in LAIR is a **Task**. Tasks are pure functions that transform inputs to outputs.

```rust
use lair::prelude::*;

#[lair::task]
pub struct SensorFusion;

impl LairTask for SensorFusion {
    type Input = (LaserScan, Imu);
    type Output = FusedOdometry;
    
    fn process(
        &mut self,
        clock: &Clock,
        input: &Self::Input,
        output: &mut Self::Output
    ) -> LairResult<()> {
        let (scan, imu) = input;
        // Fusion logic here
        output.set(fused_data);
        Ok(())
    }
}
```

#### Task Types

| Type | Description | Example |
|------|-------------|---------|
| `LairSource` | Produces data, no input | Sensor drivers |
| `LairTask` | Transforms input to output | Algorithms |
| `LairSink` | Consumes data, no output | Actuator drivers |

#### Task Lifecycle

```
┌───────────────┐
│  Unconfigured │
└───────┬───────┘
        │ configure()
        ▼
┌───────────────┐
│   Inactive    │
└───────┬───────┘
        │ activate()
        ▼
┌───────────────┐     process() called
│    Active     │◄────────────────────
└───────┬───────┘     every cycle
        │ deactivate()
        ▼
┌───────────────┐
│   Inactive    │
└───────┬───────┘
        │ cleanup()
        ▼
┌───────────────┐
│   Finalized   │
└───────────────┘
```

---

### 2. Messages

All inter-task communication uses strongly-typed messages with standard headers.

```rust
use lair_msgs::prelude::*;

#[derive(Message)]
pub struct LaserScan {
    pub header: Header,           // timestamp, frame_id, seq
    pub angle_min: f32,
    pub angle_max: f32,
    pub angle_increment: f32,
    pub ranges: Vec<f32>,
    pub intensities: Vec<f32>,
}
```

#### Standard Message Types

LAIR ships with ROS-compatible message types:

- **Geometry**: `Point`, `Pose`, `Quaternion`, `Transform`, `Twist`, `Vector3`
- **Sensors**: `LaserScan`, `PointCloud2`, `Image`, `Imu`, `NavSatFix`
- **Navigation**: `Odometry`, `Path`, `OccupancyGrid`, `Costmap`
- **Standard**: `Header`, `Time`, `Duration`, `Empty`

---

### 3. Task Graph (Configuration)

The task graph is defined in RON (Rusty Object Notation):

```ron
// robot.ron
(
    tasks: [
        (
            id: "lidar_driver",
            type: "VelodyneVLP16",
            config: { port: 2368 },
            scheduling: (
                priority: High,
                period_ms: 100,
            ),
        ),
        (
            id: "slam",
            type: "SlamToolbox",
            scheduling: (
                priority: Medium,
                period_ms: 100,
            ),
        ),
        (
            id: "navigator",
            type: "Nav2",
        ),
    ],
    
    connections: [
        (src: "lidar_driver", dst: "slam", msg: "LaserScan"),
        (src: "slam", dst: "navigator", msg: "OccupancyGrid"),
    ],
    
    safety: (
        rules: [
            (name: "velocity_limit", max: 2.0),
            (name: "human_distance", min: 1.5),
        ],
        mode: Enforced,  // or Advisory
    ),
)
```

---

### 4. Scheduling

LAIR supports multiple scheduling algorithms:

```rust
use lair::scheduling::*;

#[lair::runtime(
    config = "robot.ron",
    scheduler = RateMonotonic,  // or EDF, TimeTriggered, Custom
)]
struct MyRobot;
```

#### Built-in Schedulers

| Scheduler | Description | Best For |
|-----------|-------------|----------|
| `RateMonotonic` | Priority by period | Simple real-time |
| `EarliestDeadlineFirst` | Priority by deadline | Mixed criticality |
| `TimeTriggered` | Fixed time slots | Deterministic systems |
| `DAG` | Topological execution | Copper-style |
| `Custom` | User-defined | Special requirements |

---

### 5. Safety System (Biscuit Integration)

Safety rules are enforced at the runtime level, not application level.

```rust
// Safety rules are SQL, validated by DuckDB
#[lair::safety_rule]
pub struct VelocityLimit {
    constraint_sql: "SELECT velocity < {max_velocity} FROM robot_action",
    max_velocity: f64,
}

// Tasks can be annotated with required safety rules
#[lair::task(safety = ["velocity_limit", "torque_limit"])]
pub struct MotorController;
```

#### Safety Enforcement Flow

```
   Application Task
         │
         ▼ (wants to output action)
   ┌─────────────────┐
   │  Safety Guard   │ ◄── DuckDB validates against rules
   └─────────────────┘
         │
         ├── Valid: Pass to communication layer
         │
         └── Invalid: 
             ├── Mode::Enforced → Block + error
             └── Mode::Advisory → Warn + pass
```

---

### 6. Logging (Bagel Integration)

LAIR logs are Bagel-compatible from day 1.

```rust
// Recording
lair record --output session_001.lair

// Playback
lair play session_001.lair

// Query with natural language (via Bagel)
lair log query "What was the max velocity?"
lair log query "Show IMU data when robot was turning left"
```

#### Log Format

- **Native**: Parquet + DuckDB (Bagel's format)
- **Export**: MCAP, ROS2 bag, Copper unified log

---

### 7. Transform System

Hierarchical coordinate frame management:

```rust
use lair::transform::*;

// Broadcast a transform
tf_broadcaster.send(Transform {
    header: Header::now("odom"),
    child_frame_id: "base_link",
    transform: pose,
})?;

// Query a transform
let transform = tf_buffer.lookup_transform(
    "map",
    "camera_link",
    Time::now(),
)?;
```

---

### 8. Parameters

Type-safe parameter system with dynamic reconfiguration:

```rust
#[lair::task]
pub struct Navigator {
    #[param(default = 0.5)]
    max_velocity: f64,
    
    #[param]
    goal_tolerance: f64,
}

// At runtime
lair param set /navigator/max_velocity 1.0
lair param get /navigator/max_velocity
```

---

### 9. Actions

Long-running tasks with feedback:

```rust
use lair::action::*;

#[lair::action]
pub struct NavigateToGoal;

impl LairAction for NavigateToGoal {
    type Goal = Pose;
    type Feedback = NavigationProgress;
    type Result = NavigationResult;
    
    async fn execute(
        &mut self,
        goal: Self::Goal,
        feedback_tx: FeedbackSender<Self::Feedback>,
    ) -> Self::Result {
        loop {
            // Navigation logic
            feedback_tx.send(progress).await?;
            
            if reached_goal {
                return NavigationResult::Success;
            }
        }
    }
}
```

---

## Package System

### Package Manifest (`lair.yaml`)

```yaml
package:
  name: my_warehouse_robot
  version: "1.0.0"
  edition: "2026"
  authors: ["team@company.com"]
  license: "MIT"
  
dependencies:
  lair-nav2: "^2.0"
  lair-perception: "^1.5"
  lair-slam:
    git: "https://github.com/lair-robotics/slam"
    tag: "v3.0"

provides:
  tasks:
    - warehouse_navigator
    - pallet_detector
  messages:
    - PalletPose

hardware:
  sensors:
    - type: lidar_3d
      model: velodyne_vlp16
  compute:
    gpu: required

safety:
  rules:
    - velocity_limit: 1.5
    - human_distance: 2.0
  mode: enforced

scheduling:
  perception:
    priority: high
    period: 33ms
```

### CLI Commands

```bash
# Package management
lair pkg install nav2
lair pkg search "lidar driver"
lair pkg update
lair pkg remove old-package

# Development
lair new my_robot --template differential_drive
lair build [--release] [--target aarch64-unknown-linux-gnu]
lair run [--config production.ron]
lair test [--requirements]

# Debugging
lair graph                    # Show task DAG
lair topic list              # List active topics
lair topic echo /scan        # Print messages
lair param list              # List parameters

# Logging
lair record -o session.lair  # Record data
lair play session.lair       # Play back
lair log query "max velocity" # NLP query (Bagel)

# Simulation
lair sim launch world.sdf
lair sim spawn robot.urdf
```

---

## Embedded Runtime (lair-micro)

Same API, no_std compatible:

```rust
#![no_std]
#![no_main]

use lair_micro::prelude::*;

#[lair_micro::task(period = "1ms")]
struct MotorController {
    pwm: PwmChannel,
}

impl MicroTask for MotorController {
    type Input = VelocityCommand;
    type Output = EncoderReading;
    
    fn process(&mut self, input: &Self::Input, output: &mut Self::Output) {
        self.pwm.set_duty(input.to_pwm());
        output.ticks = self.encoder.read();
    }
}

#[lair_micro::entry]
fn main() -> ! {
    MicroExecutor::new()
        .add_task::<MotorController>()
        .run()
}
```

---

## Kubernetes Integration

```yaml
# lair-deployment.yaml
apiVersion: lair.io/v1
kind: RobotDeployment
metadata:
  name: warehouse-fleet
spec:
  replicas: 10
  robotConfig: warehouse-robot.ron
  
  containers:
    - name: perception
      image: lair.io/perception:v2
      resources:
        limits:
          nvidia.com/gpu: 1
          
    - name: navigation
      image: lair.io/nav2:v1
      
    - name: control
      image: lair.io/control:v1
      nodeSelector:
        lair.io/edge: "true"
```

---

## Comparison

| Feature | ROS 2 | Copper | LAIR |
|---------|-------|--------|------|
| Language | C++/Python | Rust | Rust |
| Deterministic | No | Yes | Yes |
| Safety Built-in | No | No | Yes (Biscuit) |
| Data Queries | No | No | Yes (Bagel) |
| Package Manager | colcon/rosdep | cargo | lair pkg |
| Embedded | MicroROS (pain) | Planned | lair-micro |
| Cloud Native | Manual | No | Kubernetes |
| Setup Files | source setup.bash | None | None |
| Config Format | XML/YAML | RON | RON/YAML |

---

## Getting Started

```bash
# Install
cargo install lair-cli

# Create project
lair new my_robot --template differential_drive
cd my_robot

# Build and run
lair build
lair run

# Query your data
lair log query "what happened during the run?"
```

---

## Open Design Questions

1. **Transport Layer**: Zenoh vs custom vs Iceoryx2?
2. **Simulation**: Bevy-native vs Gazebo bridge priority?
3. **Message IDL**: Define in Rust vs external schema?
4. **Strict Mode**: How strict? Full AutoSAR compliance?
5. **Python Bindings**: PyO3 for ML ecosystem compat?

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) and [ROADMAP.md](./ROADMAP.md).

---

*LAIR is part of the Extelligence ecosystem, alongside Bagel and Biscuit.*

