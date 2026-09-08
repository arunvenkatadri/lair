# Sencha Specification
## Experimental robotics systems

**Version**: 0.1.0-draft  
**Status**: Historical design draft; not an implementation reference
**Last Updated**: January 2026

---

## Executive Summary

Sencha is an experimental robotics systems platform exploring a successor to ROS, built in Rust on a fork of Copper. Its research program evaluates new execution architectures and shares reusable results with the ROS and Copper communities. See [RESEARCH_AGENDA.md](RESEARCH_AGENDA.md) for the current direction and [README.md](README.md) for implementation status.

The schemas, APIs, comparisons, integrations, and commands below are historical design proposals. They are not claims that these capabilities currently exist; the runtime and public API have diverged from this draft.

### Why Sencha Exists

| Problem with ROS | Sencha Solution |
|------------------|---------------|
| `source setup.bash` nightmare | Rust modules, zero env config |
| MicroROS is painful | Same API for micro and full runtime |
| Non-deterministic scheduling | Configurable deterministic schedulers |
| Package management is weak | `sencha pkg install` like apt/dnf |
| No built-in safety | Biscuit physics guards at runtime level |
| Hard to query logs | Bagel NLP integration from day 1 |
| Not cloud-native | Kubernetes operator, cloud-first |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Sencha ARCHITECTURE                                │
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
│  │  │  Linux (Full)   │ │   sencha-micro    │ │   Simulation    │       │   │
│  │  │  x86/ARM/RISC-V │ │  STM32/ESP32/RP │ │   Bevy/Gazebo   │       │   │
│  │  └─────────────────┘ └─────────────────┘ └─────────────────┘       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Concepts

### 1. Tasks

The fundamental unit of computation in Sencha is a **Task**. Tasks are pure functions that transform inputs to outputs.

```rust
use sencha::prelude::*;

#[sencha::task]
pub struct SensorFusion;

impl SenchaTask for SensorFusion {
    type Input = (LaserScan, Imu);
    type Output = FusedOdometry;
    
    fn process(
        &mut self,
        clock: &Clock,
        input: &Self::Input,
        output: &mut Self::Output
    ) -> SenchaResult<()> {
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
| `SenchaSource` | Produces data, no input | Sensor drivers |
| `SenchaTask` | Transforms input to output | Algorithms |
| `SenchaSink` | Consumes data, no output | Actuator drivers |

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
use sencha_msgs::prelude::*;

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

Sencha ships with ROS-compatible message types:

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

Sencha supports multiple scheduling algorithms:

```rust
use sencha::scheduling::*;

#[sencha::runtime(
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
#[sencha::safety_rule]
pub struct VelocityLimit {
    constraint_sql: "SELECT velocity < {max_velocity} FROM robot_action",
    max_velocity: f64,
}

// Tasks can be annotated with required safety rules
#[sencha::task(safety = ["velocity_limit", "torque_limit"])]
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

Sencha logs are Bagel-compatible from day 1.

```rust
// Recording
sencha record --output session_001.sencha

// Playback
sencha play session_001.sencha

// Query with natural language (via Bagel)
sencha log query "What was the max velocity?"
sencha log query "Show IMU data when robot was turning left"
```

#### Log Format

- **Native**: Parquet + DuckDB (Bagel's format)
- **Export**: MCAP, ROS2 bag, Copper unified log

---

### 7. Transform System

Hierarchical coordinate frame management:

```rust
use sencha::transform::*;

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
#[sencha::task]
pub struct Navigator {
    #[param(default = 0.5)]
    max_velocity: f64,
    
    #[param]
    goal_tolerance: f64,
}

// At runtime
sencha param set /navigator/max_velocity 1.0
sencha param get /navigator/max_velocity
```

---

### 9. Actions

Long-running tasks with feedback:

```rust
use sencha::action::*;

#[sencha::action]
pub struct NavigateToGoal;

impl SenchaAction for NavigateToGoal {
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

### Package Manifest (`sencha.yaml`)

```yaml
package:
  name: my_warehouse_robot
  version: "1.0.0"
  edition: "2026"
  authors: ["team@company.com"]
  license: "MIT"
  
dependencies:
  sencha-nav2: "^2.0"
  sencha-perception: "^1.5"
  sencha-slam:
    git: "https://github.com/sencha-robotics/slam"
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
sencha pkg install nav2
sencha pkg search "lidar driver"
sencha pkg update
sencha pkg remove old-package

# Development
sencha new my_robot --template differential_drive
sencha build [--release] [--target aarch64-unknown-linux-gnu]
sencha run [--config production.ron]
sencha test [--requirements]

# Debugging
sencha graph                    # Show task DAG
sencha topic list              # List active topics
sencha topic echo /scan        # Print messages
sencha param list              # List parameters

# Logging
sencha record -o session.sencha  # Record data
sencha play session.sencha       # Play back
sencha log query "max velocity" # NLP query (Bagel)

# Simulation
sencha sim launch world.sdf
sencha sim spawn robot.urdf
```

---

## Embedded Runtime (sencha-micro)

Same API, no_std compatible:

```rust
#![no_std]
#![no_main]

use sencha_micro::prelude::*;

#[sencha_micro::task(period = "1ms")]
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

#[sencha_micro::entry]
fn main() -> ! {
    MicroExecutor::new()
        .add_task::<MotorController>()
        .run()
}
```

---

## Kubernetes Integration

```yaml
# sencha-deployment.yaml
apiVersion: sencha.io/v1
kind: RobotDeployment
metadata:
  name: warehouse-fleet
spec:
  replicas: 10
  robotConfig: warehouse-robot.ron
  
  containers:
    - name: perception
      image: sencha.io/perception:v2
      resources:
        limits:
          nvidia.com/gpu: 1
          
    - name: navigation
      image: sencha.io/nav2:v1
      
    - name: control
      image: sencha.io/control:v1
      nodeSelector:
        sencha.io/edge: "true"
```

---

## Comparison

| Feature | ROS 2 | Copper | Sencha |
|---------|-------|--------|------|
| Language | C++/Python | Rust | Rust |
| Deterministic | No | Yes | Yes |
| Safety Built-in | No | No | Yes (Biscuit) |
| Data Queries | No | No | Yes (Bagel) |
| Package Manager | colcon/rosdep | cargo | sencha pkg |
| Embedded | MicroROS (pain) | Planned | sencha-micro |
| Cloud Native | Manual | No | Kubernetes |
| Setup Files | source setup.bash | None | None |
| Config Format | XML/YAML | RON | RON/YAML |

---

## Getting Started

```bash
# Install
cargo install sencha-cli

# Create project
sencha new my_robot --template differential_drive
cd my_robot

# Build and run
sencha build
sencha run

# Query your data
sencha log query "what happened during the run?"
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

*Sencha is part of the Extelligence ecosystem, alongside Bagel and Biscuit.*
