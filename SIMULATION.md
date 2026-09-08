# LAIR Simulation Architecture

**Version**: 0.1.0-draft
**Status**: Historical design draft; external simulation is a mock on main
**Last Updated**: February 2026

---

## Executive Summary

The integration designs and dates below are proposals. The [research agenda](RESEARCH_AGENDA.md) prioritizes the simulator or robot platform needed to evaluate the first architectural contribution; full Isaac Sim support is not a prerequisite for the early-2027 paper/artifact target. See the [README](README.md) for current implementation status.

LAIR's simulation strategy prioritizes **NVIDIA Isaac Sim** as the primary simulation platform, with record/replay as the initial testing mechanism and Bevy as an open-source fallback. This aligns with LAIR's target markets (AV, industrial, agriculture) which already use NVIDIA hardware.

### Strategy Overview

```
Phase 1 (v0.1-v0.2)          Phase 2 (v0.3)              Phase 3 (Future)
──────────────────          ───────────────             ────────────────
Record/Replay               Isaac Sim Integration        Bevy Alternative
"Good enough for CI"        "Production simulation"      "Open source option"

Q2 2026                     Q4 2026                      2027
```

---

## Why NVIDIA Isaac Sim?

### Market Alignment

LAIR's target customers already use NVIDIA:

| Market | NVIDIA Hardware | Why Isaac Sim Matters |
|--------|----------------|----------------------|
| **Autonomous Vehicles** | NVIDIA Drive AGX | Photorealistic sensor sim for perception validation |
| **Industrial/Warehouse** | Jetson AGX Orin | Fleet testing without physical robots |
| **Agriculture** | Jetson for autonomy | Simulate field conditions, weather, lighting |
| **Commercial Robotics** | Jetson/GPU compute | Rapid prototyping, edge case testing |

### Technical Advantages

- **PhysX 5**: GPU-accelerated, industry-standard physics
- **RTX Rendering**: Photorealistic camera/lidar simulation
- **Synthetic Data**: Generate training data for perception models
- **Hardware-in-Loop**: Test real code against simulated sensors
- **USD Ecosystem**: Interoperable with Blender, Maya, Unreal
- **Proven at Scale**: Used by major AV companies, industrial robotics

---

## Architecture

### High-Level Overview

```
┌────────────────────────────────────────────────────────────────┐
│                      LAIR Application                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Your Robot Tasks (LairSource, LairTask, LairSink)      │  │
│  │  - ObstacleDetector                                      │  │
│  │  - PathPlanner                                           │  │
│  │  - MotorController                                       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│                           ▼                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              LAIR Runtime (Pure Rust)                    │  │
│  │  Scheduler • Transforms • Logging • Safety • Parameters  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│         ┌─────────────────┼─────────────────┐                  │
│         ▼                 ▼                 ▼                  │
│  ┌────────────┐    ┌────────────┐    ┌────────────┐          │
│  │   Linux    │    │  lair-micro│    │ lair-isaac │          │
│  │  (Real HW) │    │ (Embedded) │    │   (Sim)    │          │
│  └────────────┘    └────────────┘    └────────────┘          │
│                                              │                  │
└──────────────────────────────────────────────┼──────────────────┘
                                               │
                                               ▼
                    ┌────────────────────────────────────────┐
                    │      NVIDIA Isaac Sim (Omniverse)      │
                    │  ┌──────────────────────────────────┐  │
                    │  │  PhysX 5 Physics Engine          │  │
                    │  │  • GPU-accelerated dynamics      │  │
                    │  │  • Contact resolution            │  │
                    │  │  • Joint constraints             │  │
                    │  └──────────────────────────────────┘  │
                    │  ┌──────────────────────────────────┐  │
                    │  │  RTX Sensor Simulation           │  │
                    │  │  • Raytrace lidar                │  │
                    │  │  • Render cameras (RGB/depth)    │  │
                    │  │  • IMU from physics state        │  │
                    │  └──────────────────────────────────┘  │
                    │  ┌──────────────────────────────────┐  │
                    │  │  USD Scene (Omniverse)           │  │
                    │  │  • Warehouse.usd                 │  │
                    │  │  • Robot.urdf → USD              │  │
                    │  │  • Lighting, materials           │  │
                    │  └──────────────────────────────────┘  │
                    └────────────────────────────────────────┘
```

---

## Phase 1: Record/Replay (v0.1-v0.2)

**Goal**: Enable automated testing without full simulation.

### Design

```rust
// robot.ron - Replay mode
(
    runtime: (
        mode: Replay,
        data_source: "test_runs/nominal_case.lair",
    ),
    tasks: [
        // Same task definitions as production
        (id: "lidar_driver", type: "VelodyneVLP16"),
        (id: "slam", type: "SlamToolbox"),
    ],
)
```

### Implementation

```rust
// crates/lair-core/src/replay.rs
use lair_bagel::LairLog;

pub struct ReplayRuntime {
    log: LairLog,
    playback_speed: f64,  // 1.0 = real-time, 0.0 = as fast as possible
}

impl ReplayRuntime {
    pub fn new(path: &str) -> LairResult<Self> {
        let log = LairLog::open(path)?;
        Ok(Self { log, playback_speed: 1.0 })
    }

    pub fn next_frame(&mut self) -> Option<RecordedFrame> {
        // Read from Parquet log
        self.log.read_next_frame()
    }
}

// Replay sensor "drivers" just read from log
#[lair::task]
pub struct ReplayLidarSource {
    replay: Arc<Mutex<ReplayRuntime>>,
}

impl LairSource for ReplayLidarSource {
    type Output = LaserScan;

    fn generate(&mut self, clock: &Clock, output: &mut LaserScan) {
        let frame = self.replay.lock().next_frame().unwrap();
        *output = frame.get_topic::<LaserScan>("/scan");
    }
}
```

### Use Cases
- **CI/CD Testing**: Run regression tests on recorded data
- **Algorithm Development**: Iterate on perception/planning without robot
- **Reproducible Debugging**: Replay exact sensor sequence that caused a bug

### Limitations
- Can't test actuator outputs (no physics feedback)
- Can't test novel scenarios
- Limited to recorded data diversity

---

## Phase 2: Isaac Sim Integration (v0.3)

**Goal**: Full physics simulation with photorealistic sensors.

### Integration Architecture

Three integration options evaluated:

#### **Option 1: Python Bridge (Recommended for v0.3)**

**Pros**: Easiest to implement, Isaac Sim has mature Python API
**Cons**: FFI overhead, not pure Rust

```rust
// crates/lair-isaac/src/lib.rs
use pyo3::prelude::*;
use pyo3::types::PyModule;

pub struct IsaacSimBridge {
    py: Python<'static>,
    sim_module: Py<PyModule>,
    stage: Py<PyAny>,  // USD stage
}

impl IsaacSimBridge {
    pub fn new(world_path: &str) -> LairResult<Self> {
        Python::with_gil(|py| {
            // Import Isaac Sim Python modules
            let sim = PyModule::import(py, "isaacsim")?;
            let stage = sim.call_method1("load_stage", (world_path,))?;

            Ok(Self {
                py,
                sim_module: sim.into(),
                stage: stage.into(),
            })
        })
    }

    pub fn step(&self, dt: f64) -> LairResult<()> {
        Python::with_gil(|py| {
            self.sim_module
                .as_ref(py)
                .call_method1("step", (dt,))?;
            Ok(())
        })
    }

    pub fn read_lidar(&self, prim_path: &str) -> LairResult<Vec<f32>> {
        Python::with_gil(|py| {
            let data = self.sim_module
                .as_ref(py)
                .call_method1("get_lidar_data", (prim_path,))?;

            // Convert numpy array to Rust Vec
            let ranges: Vec<f32> = data.extract()?;
            Ok(ranges)
        })
    }
}
```

#### **Option 2: gRPC Bridge (Recommended for Production)**

**Pros**: Language-agnostic, clean separation, scalable
**Cons**: More implementation work, latency

```rust
// Define protobuf schema
// lair-isaac/proto/isaac_bridge.proto
syntax = "proto3";

service IsaacSimBridge {
    rpc StepSimulation(StepRequest) returns (StepResponse);
    rpc ReadSensor(SensorRequest) returns (SensorData);
    rpc SetActuator(ActuatorCommand) returns (ActuatorResponse);
}

message StepRequest {
    double dt = 1;
}

message SensorRequest {
    string prim_path = 1;
    SensorType type = 2;
}

message SensorData {
    oneof data {
        LaserScanData lidar = 1;
        ImageData camera = 2;
        ImuData imu = 3;
    }
}

// Rust client
use tonic::transport::Channel;
use isaac_bridge_proto::isaac_sim_bridge_client::IsaacSimBridgeClient;

pub struct GrpcIsaacBridge {
    client: IsaacSimBridgeClient<Channel>,
}

impl GrpcIsaacBridge {
    pub async fn connect(addr: &str) -> LairResult<Self> {
        let client = IsaacSimBridgeClient::connect(addr).await?;
        Ok(Self { client })
    }

    pub async fn step(&mut self, dt: f64) -> LairResult<()> {
        let request = StepRequest { dt };
        self.client.step_simulation(request).await?;
        Ok(())
    }
}
```

Python server runs in Isaac Sim:
```python
# isaac_sim_server.py
import grpc
from concurrent import futures
import isaacsim
from isaac_bridge_pb2 import *
from isaac_bridge_pb2_grpc import *

class IsaacSimBridgeServicer(IsaacSimBridgeServicer):
    def __init__(self):
        self.stage = isaacsim.load_stage("warehouse.usd")

    def StepSimulation(self, request, context):
        isaacsim.step(request.dt)
        return StepResponse(success=True)

    def ReadSensor(self, request, context):
        if request.type == LIDAR:
            data = isaacsim.get_lidar_data(request.prim_path)
            return SensorData(lidar=LaserScanData(ranges=data))

def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    add_IsaacSimBridgeServicer_to_server(IsaacSimBridgeServicer(), server)
    server.add_insecure_port('[::]:50051')
    server.start()
    server.wait_for_termination()
```

#### **Option 3: Zenoh Bridge (Future - Multi-Robot)**

**Pros**: Same transport as multi-robot LAIR, discovery built-in
**Cons**: More complex, overkill for single robot sim

```rust
// Use Zenoh for pub/sub between LAIR and Isaac Sim
use zenoh::prelude::*;

pub struct ZenohIsaacBridge {
    session: zenoh::Session,
}

impl ZenohIsaacBridge {
    pub async fn new() -> LairResult<Self> {
        let session = zenoh::open(zenoh::config::Config::default()).await?;
        Ok(Self { session })
    }

    pub async fn subscribe_lidar(&self) -> impl Stream<Item = LaserScan> {
        self.session
            .declare_subscriber("/sim/lidar")
            .await
            .unwrap()
            .map(|sample| bincode::deserialize(&sample.payload).unwrap())
    }
}
```

### Recommended Path
1. **v0.3 MVP**: Python bridge (PyO3) - fastest to implement
2. **v0.4 Production**: gRPC bridge - clean, scalable, language-agnostic
3. **v0.5+ Fleet**: Zenoh bridge - unified multi-robot transport

---

## Simulated Sensors

### Architecture Pattern

All simulated sensors implement the same `LairSource` trait as real sensors:

```rust
// Real hardware driver
#[lair::task]
pub struct VelodyneVLP16 {
    socket: UdpSocket,
}

impl LairSource for VelodyneVLP16 {
    type Output = LaserScan;
    fn generate(&mut self, clock: &Clock, output: &mut LaserScan) {
        // Read from UDP socket
    }
}

// Isaac Sim driver (same interface!)
#[lair::task]
pub struct IsaacLidar {
    bridge: Arc<IsaacSimBridge>,
    prim_path: String,  // "/World/Robot/lidar"
}

impl LairSource for IsaacLidar {
    type Output = LaserScan;
    fn generate(&mut self, clock: &Clock, output: &mut LaserScan) {
        // Read from Isaac Sim via bridge
        let ranges = self.bridge.read_lidar(&self.prim_path)?;
        output.ranges = ranges;
        output.header.stamp = clock.now();
    }
}
```

### Sensor Implementations

#### **3D Lidar (Velodyne, Ouster, Livox)**

```rust
// crates/lair-isaac/src/sensors/lidar.rs
pub struct IsaacLidar3D {
    bridge: Arc<IsaacSimBridge>,
    config: LidarConfig,
}

pub struct LidarConfig {
    pub prim_path: String,
    pub horizontal_fov: f32,      // 360° for Velodyne
    pub vertical_fov: f32,         // 30° for VLP-16
    pub horizontal_resolution: u32, // 1800 points
    pub vertical_channels: u32,    // 16 for VLP-16
    pub max_range: f32,            // 100m
    pub rotation_rate: f32,        // 10 Hz
}

impl LairSource for IsaacLidar3D {
    type Output = PointCloud2;

    fn generate(&mut self, clock: &Clock, output: &mut PointCloud2) {
        // Isaac Sim RTX raytrace lidar
        let points = self.bridge.read_lidar_pointcloud(&self.config.prim_path)?;

        output.header.stamp = clock.now();
        output.header.frame_id = "lidar";
        output.points = points;
    }
}
```

#### **RGB-D Camera**

```rust
pub struct IsaacRGBDCamera {
    bridge: Arc<IsaacSimBridge>,
    prim_path: String,
}

impl LairSource for IsaacRGBDCamera {
    type Output = (Image, Image);  // (RGB, Depth)

    fn generate(&mut self, clock: &Clock, output: &mut (Image, Image)) {
        // Isaac Sim RTX rendering
        let (rgb, depth) = self.bridge.read_camera_rgbd(&self.prim_path)?;

        output.0.header.stamp = clock.now();
        output.0.data = rgb;
        output.1.data = depth;
    }
}
```

#### **IMU**

```rust
pub struct IsaacIMU {
    bridge: Arc<IsaacSimBridge>,
    prim_path: String,
    noise_config: ImuNoiseConfig,
}

pub struct ImuNoiseConfig {
    pub accel_noise_stddev: f64,
    pub gyro_noise_stddev: f64,
    pub accel_bias_stddev: f64,
}

impl LairSource for IsaacIMU {
    type Output = Imu;

    fn generate(&mut self, clock: &Clock, output: &mut Imu) {
        // Read from PhysX rigid body state
        let body_state = self.bridge.read_rigid_body_state(&self.prim_path)?;

        output.linear_acceleration = body_state.accel + self.add_noise();
        output.angular_velocity = body_state.gyro + self.add_noise();
    }
}
```

#### **GPS**

```rust
pub struct IsaacGPS {
    bridge: Arc<IsaacSimBridge>,
    world_origin_lat_lon: (f64, f64),  // Convert USD coords to GPS
}

impl LairSource for IsaacGPS {
    type Output = NavSatFix;

    fn generate(&mut self, clock: &Clock, output: &mut NavSatFix) {
        let pos = self.bridge.read_position(&self.prim_path)?;
        let (lat, lon) = self.usd_to_gps(pos);

        output.latitude = lat;
        output.longitude = lon;
        output.altitude = pos.z;
    }
}
```

---

## Configuration

### Unified Config Format

The same RON config works for real hardware and simulation:

```ron
// robot.ron
(
    runtime: (
        // Switch platforms via environment variable or CLI flag
        mode: FromEnv("LAIR_PLATFORM"),  // "linux", "isaac_sim", "replay"

        isaac_sim: Some((
            world: "assets/worlds/warehouse.usd",
            robot: "assets/robots/amr.urdf",
            headless: false,
            physics_dt: 0.001,  // 1ms physics timestep
        )),
    ),

    tasks: [
        (
            id: "lidar",
            // Type resolves to different impl based on platform
            type: "Lidar3D",
            config: (
                // Real hardware config
                ip: "192.168.1.201",
                port: 2368,

                // Isaac Sim config (ignored on real HW)
                prim_path: "/World/Robot/lidar",

                // Shared config
                frame_id: "lidar",
                max_range: 100.0,
            ),
        ),
    ],
)
```

### Platform Selection

```rust
// At runtime, resolve based on platform
pub fn create_lidar(config: &TaskConfig, platform: Platform) -> Box<dyn LairSource> {
    match platform {
        Platform::Linux => Box::new(VelodyneVLP16::new(config)),
        Platform::IsaacSim => Box::new(IsaacLidar::new(config)),
        Platform::Replay => Box::new(ReplayLidar::new(config)),
    }
}
```

---

## Testing Strategy

### Unit Tests (Mock Sensors)

```rust
#[cfg(test)]
mod tests {
    use lair_test::*;

    #[test]
    fn test_obstacle_detector() {
        let mut detector = ObstacleDetector::new();
        let mut clock = MockClock::new();

        // Create mock sensor data
        let scan = LaserScan {
            ranges: vec![1.0, 2.0, 0.5, 3.0],
            ..Default::default()
        };

        let mut obstacles = ObstacleList::default();
        detector.process(&clock, &scan, &mut obstacles).unwrap();

        assert_eq!(obstacles.len(), 1);  // One obstacle at 0.5m
    }
}
```

### Integration Tests (Replay)

```rust
#[test]
fn test_full_stack_replay() {
    let runtime = LairRuntime::new("test_config.ron")?;
    runtime.set_platform(Platform::Replay("test_data.lair"));

    runtime.run_for_duration(Duration::from_secs(10))?;

    // Assert expected outcomes
    assert!(runtime.get_task::<Navigator>().reached_goal());
}
```

### Simulation Tests (Isaac Sim - Headless)

```rust
#[test]
#[cfg(feature = "isaac")]
fn test_obstacle_avoidance_sim() {
    let runtime = LairRuntime::new("sim_test.ron")?;
    runtime.set_platform(Platform::IsaacSim {
        world: "test_worlds/obstacle_course.usd",
        headless: true,
    });

    runtime.run_until(|r| {
        r.get_task::<Navigator>().at_goal()
    })?;

    // Verify no collisions occurred
    let collisions = runtime.get_metric("collision_count");
    assert_eq!(collisions, 0);
}
```

### CI/CD Pipeline

```yaml
# .github/workflows/test.yml
name: LAIR Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --lib

  replay-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --features replay

  isaac-sim-tests:
    runs-on: ubuntu-latest-gpu  # Self-hosted runner with NVIDIA GPU
    steps:
      - uses: actions/checkout@v3
      - name: Start Isaac Sim
        run: ./scripts/start_isaac_sim.sh --headless
      - name: Run simulation tests
        run: cargo test --features isaac -- --test-threads=1
      - name: Stop Isaac Sim
        run: ./scripts/stop_isaac_sim.sh
```

---

## Phase 3: Bevy Alternative (Future)

**Goal**: Open-source, lightweight alternative for customers without NVIDIA hardware.

### When to Use Bevy

- **Prototyping**: Quick iteration without Isaac Sim setup
- **Local Development**: Laptop development without GPU
- **Open Source**: Customers who prefer fully open stack
- **Education**: Teaching robotics concepts

### Architecture

```rust
// crates/lair-bevy/src/lib.rs
use bevy::prelude::*;
use avian3d::prelude::*;

#[lair::runtime(platform = "bevy")]
struct BevyRobot {
    physics: PhysicsPlugin,
    render: RenderPlugin,
}

pub struct BevyLidar {
    raycast: RayCaster,
    config: LidarConfig,
}

impl LairSource for BevyLidar {
    type Output = LaserScan;

    fn generate(&mut self, clock: &Clock, output: &mut LaserScan) {
        // Raycast in Bevy physics world
        for (i, angle) in self.config.angles().enumerate() {
            let ray = Ray::new(self.position, angle.to_direction());
            if let Some(hit) = self.raycast.cast(ray, self.config.max_range) {
                output.ranges[i] = hit.distance;
            }
        }
    }
}
```

### Comparison

| Feature | Isaac Sim | Bevy |
|---------|-----------|------|
| **Cost** | Free (proprietary) | Free (MIT/Apache) |
| **Performance** | GPU-accelerated | CPU-based |
| **Realism** | Photorealistic RTX | Basic rendering |
| **Dependencies** | Omniverse, CUDA | Pure Rust |
| **Use Case** | Production testing | Prototyping, education |

---

## Roadmap Integration

### v0.1-v0.2: Foundation
- [ ] **Record/Replay system** (P0)
  - Parquet-based log format (Bagel-compatible)
  - Replay sources for all standard sensors
  - CLI: `lair record`, `lair replay`

### v0.3: Simulation Testing
- [ ] **Isaac Sim Python Bridge** (P0)
  - PyO3-based bridge to Isaac Sim
  - Standard sensor implementations (lidar, camera, IMU, GPS)
  - Headless mode for CI/CD
  - Example worlds (warehouse, outdoor, factory)
- [ ] **Simulation Testing Framework** (P0)
  - Automated test scenarios
  - Performance regression detection
  - Edge case generation

### v0.4: Production Simulation
- [ ] **Isaac Sim gRPC Bridge** (P1)
  - Production-grade protocol
  - Multi-instance support (fleet testing)
  - Hardware-in-loop support
- [ ] **Synthetic Data Generation** (P1)
  - Perception model training pipeline
  - Domain randomization
  - Integration with Cosmos (NVIDIA)

### v0.5+: Advanced Features
- [ ] **Bevy Simulation** (P2)
  - Open-source alternative
  - Lightweight local testing
- [ ] **Multi-Robot Simulation** (P1)
  - Fleet testing in Isaac Sim
  - Coordination scenario testing
- [ ] **Digital Twin** (P2)
  - Real-time sync: real robot ↔ simulation
  - Predictive maintenance

---

## Dependencies

### Isaac Sim Requirements

**Hardware:**
- NVIDIA RTX GPU (RTX 2070 or better recommended)
- 32GB RAM (64GB for large scenes)
- Ubuntu 22.04 or Windows 11

**Software:**
- NVIDIA Omniverse Launcher
- Isaac Sim 4.5+ (free download)
- CUDA 12.0+
- Python 3.10+

### Rust Dependencies

See updated `Cargo.toml` for:
- `pyo3` - Python FFI
- `tonic` - gRPC (future)
- `zenoh` - Multi-robot transport (future)

---

## CLI Commands

### Recording

```bash
# Record live data
lair record --output test_run_001.lair --duration 60s

# Record with topic filtering
lair record -o data.lair --topics /lidar,/camera,/imu
```

### Replay

```bash
# Replay at normal speed
lair replay test_run_001.lair

# Replay as fast as possible (CI testing)
lair replay test_run_001.lair --speed 0

# Replay with visualization
lair replay test_run_001.lair --visualize
```

### Simulation

```bash
# Launch Isaac Sim simulation
lair sim launch --world warehouse.usd --robot amr.urdf

# Headless mode (CI/CD)
lair sim launch --world test.usd --headless

# Run automated test scenario
lair sim test scenarios/obstacle_avoidance.yaml

# Generate synthetic data
lair sim datagen --scenario parking_lot --samples 10000
```

---

## Best Practices

### 1. **Same Code, Multiple Platforms**

Write tasks once, run everywhere:

```rust
#[lair::task]
pub struct ObstacleDetector;

impl LairTask for ObstacleDetector {
    type Input = LaserScan;
    type Output = ObstacleList;

    fn process(&mut self, clock: &Clock, scan: &LaserScan, obstacles: &mut ObstacleList) {
        // This code runs identically on:
        // - Real hardware
        // - Isaac Sim
        // - Replay
        // - Bevy sim
    }
}
```

### 2. **Test Pyramid**

```
         ▲
        / \
       /   \      1-2 Isaac Sim integration tests (expensive, slow)
      /─────\
     /       \
    /─────────\   10-20 replay integration tests (moderate cost)
   /           \
  /─────────────\ 100+ unit tests (cheap, fast)
 /               \
```

### 3. **Deterministic Testing**

```rust
// Use fixed seeds for reproducibility
let config = IsaacSimConfig {
    physics_seed: 42,
    random_seed: 42,
    deterministic: true,
};

// Same inputs → same outputs, every time
```

### 4. **Hardware-in-Loop**

```rust
// Real perception hardware + simulated environment
let config = HybridConfig {
    sensors: Platform::Linux,     // Real camera/lidar
    environment: Platform::IsaacSim, // Simulated world
    actuators: Platform::IsaacSim,  // Virtual motors
};
```

---

## Open Questions

1. **Isaac Sim Licensing**: Confirm free tier supports CI/CD usage
2. **USD vs URDF**: Primary robot description format?
3. **Cloud Rendering**: Offload Isaac Sim to cloud for teams without GPUs?
4. **Scenario DSL**: Define test scenarios in RON vs Python vs YAML?
5. **Fault Injection**: How to simulate sensor failures, network drops, etc.?

---

## References

- [NVIDIA Isaac Sim Documentation](https://docs.isaacsim.omniverse.nvidia.com/)
- [Isaac Sim GitHub](https://github.com/isaac-sim/IsaacSim)
- [Omniverse USD](https://developer.nvidia.com/usd)
- [Bevy Engine](https://bevyengine.org/)
- [Avian Physics](https://github.com/Jondolf/avian)

---

*LAIR Simulation is part of the Extelligence ecosystem alongside Bagel, Biscuit, and Matcha.*
