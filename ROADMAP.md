# LAIR Roadmap

## Vision

**LAIR** (Layered Autonomous Intelligence Runtime) is a production robotics operating system for commercial and industrial deployments. Built in Rust with safety-first design, deterministic execution, and cloud-native architecture via Matcha.

**Target Markets (in order):**
1. 🚗 **Autonomous Vehicles** - Trucks, delivery, shuttles, robotaxis
2. 🏭 **Industrial / Warehouse** - AMRs, forklifts, factory automation
3. 🚜 **Agriculture / Construction** - Autonomous tractors, dozers, haulers
4. 🤖 **Commercial Robotics** - Delivery robots, inspection, service

**NOT building for:** Research, academia, education, hobbyists.

---

## The Extelligence Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                    EXTELLIGENCE STACK                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  MATCHA (Cloud Platform)                                        │
│  ├─ Fleet management & monitoring                               │
│  ├─ 35+ MCP tools for data queries                             │
│  ├─ OTA updates                                                 │
│  ├─ Multi-tenant SaaS                                          │
│  └─ VS Code/Cursor extension                                   │
│                                                                 │
│  BAGEL (Data Intelligence)                                      │
│  ├─ Natural language log queries                               │
│  ├─ Multi-format support (ROS, PX4, ArduPilot)                │
│  └─ DuckDB/Parquet backend                                     │
│                                                                 │
│  BISCUIT (Physics-Constrained Safety) [Optional]               │
│  ├─ LLM output validation against physics                      │
│  ├─ For service robots, manipulation, voice commands           │
│  └─ Future: Evolve into true physical model for safety-critical│
│                                                                 │
│  LAIR (Robot OS)                                                │
│  ├─ Deterministic runtime                                       │
│  ├─ Production scheduling (DAG, Rate Monotonic, Time-Triggered)│
│  ├─ Zero-copy messaging                                        │
│  └─ Safety-critical architecture                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Release Phases

```
v0.1 Foundation    v0.2 Fleet Ops    v0.3 Certification    v0.4 Enterprise
────────────────   ──────────────    ─────────────────     ───────────────
"It runs"          "It scales"       "It's certified"      "It sells"

Q2 2026            Q3 2026           Q4 2026               2027
```

---

## v0.1 — Production Foundation

**Goal:** A working runtime that's better than ROS for production deployments.

### Core Runtime
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Deterministic DAG Scheduler | P0 | 🔴 | Execute tasks in dependency order |
| Rate Monotonic Scheduler | P0 | 🔴 | Priority by period |
| Zero-Copy Message Bus | P0 | 🔴 | Lock-free, bounded queues |
| Task Trait System | P0 | 🔴 | `LairSource`, `LairTask`, `LairSink` |
| RON Config Parser | P0 | 🔴 | Task graph definition |
| Monotonic Clock | P0 | 🔴 | High-precision, mockable |
| Health Monitoring | P0 | 🔴 | Heartbeats, watchdogs |
| Graceful Degradation | P0 | 🔴 | Limp home, don't crash |

### Message Types
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Standard Header | P0 | 🔴 | timestamp, frame_id, sequence |
| Geometry Messages | P0 | 🔴 | Point, Pose, Transform, Twist |
| Sensor Messages | P0 | 🔴 | LaserScan, Image, Imu, PointCloud2 |
| Vehicle Messages | P1 | 🔴 | VehicleState, ControlCommand |

### Integrations (All Optional via Feature Flags)
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Bagel Logging | P0 | 🔴 | Parquet/DuckDB format, Matcha-compatible |
| Matcha Cloud Sync | P1 | 🔴 | Fleet visibility, remote diagnostics |
| Biscuit Safety | P2 | 🔴 | Optional - for service robots, not AV |

### CLI (Basic)
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| `lair new` | P0 | 🔴 | Create new project from template |
| `lair build` | P0 | 🔴 | Compile with feature flags |
| `lair run` | P0 | 🔴 | Execute robot |
| `lair doctor` | P1 | 🔴 | Diagnose common issues |

---

## v0.2 — Fleet Operations

**Goal:** Deploy and manage hundreds of robots.

### Fleet Management
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| OTA Updates | P0 | 🔴 | Push updates to fleet |
| Rollback | P0 | 🔴 | Revert bad updates |
| Remote Diagnostics | P0 | 🔴 | Debug without site visit |
| Fleet Dashboard | P0 | 🔴 | Via Matcha web UI |
| Health Aggregation | P0 | 🔴 | Cross-fleet health view |

### Multi-Robot
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Zenoh Transport | P0 | 🔴 | Multi-machine messaging |
| Discovery | P0 | 🔴 | mDNS/DNS-SD |
| Coordination Primitives | P1 | 🔴 | Mutex, barriers for multi-robot |

### Data Pipeline
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Log Upload to Matcha | P0 | 🔴 | Automatic sync |
| Bagel Queries | P0 | 🔴 | "Why did robot 47 stop?" |
| Cross-Fleet Analytics | P1 | 🔴 | Pattern detection |

---

## v0.3 — Certification Ready

**Goal:** Ready for ISO 26262, ISO 13482, IEC 62443.

### Scheduling
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Time-Triggered Scheduler | P0 | 🔴 | Pre-computed schedule tables |
| WCET Analysis Tools | P1 | 🔴 | Worst-case execution time |
| Scheduling Verification | P1 | 🔴 | Prove timing guarantees |

### Safety Architecture
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Redundancy Patterns | P0 | 🔴 | Dual compute, failover |
| Watchdog Framework | P0 | 🔴 | Hardware + software watchdogs |
| Safe State Definition | P0 | 🔴 | What happens on failure |
| Fail-Operational Mode | P1 | 🔴 | Continue with reduced capability |

### Compliance
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Audit Logging | P0 | 🔴 | Tamper-proof, timestamped |
| Traceability Matrix | P0 | 🔴 | Requirements → code → tests |
| Documentation Generation | P1 | 🔴 | Auto-generate compliance docs |
| Security Hardening | P0 | 🔴 | IEC 62443 ready |

### Testing
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| V-Model Test Framework | P0 | 🔴 | Requirements tracing |
| Simulation Testing | P0 | 🔴 | Headless, CI-friendly |
| Fault Injection | P1 | 🔴 | Test failure modes |

---

## v0.4 — Enterprise

**Goal:** Enterprise sales, support contracts, monetization.

### Business Features
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| Multi-Tenant Fleet Mgmt | P0 | 🔴 | Multiple customers on Matcha |
| SSO/SAML | P0 | 🔴 | Enterprise IT requirements |
| Usage Metering | P0 | 🔴 | Per-robot pricing |
| SLA Dashboard | P0 | 🔴 | Contract compliance |
| Support Ticketing | P1 | 🔴 | Enterprise support |

### Advanced Features
| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| ML Model Registry | P1 | 🔴 | Version and deploy models |
| Model Hot-Swap | P1 | 🔴 | Update models without restart |
| A/B Testing | P2 | 🔴 | Test changes on subset |

---

## v0.5 — Embedded (lair-micro)

**Goal:** Same API on microcontrollers.

| Feature | Priority | Status | Notes |
|---------|----------|--------|-------|
| no_std Runtime | P0 | 🔴 | Bare metal executor |
| Static Allocation | P0 | 🔴 | No heap in hot path |
| STM32 Support | P0 | 🔴 | F4, H7, L4 families |
| ESP32 Support | P1 | 🔴 | Via esp-hal |
| Serial Bridge | P0 | 🔴 | UART to host |
| CAN Bus | P1 | 🔴 | Industrial protocol |

---

## Future

### Biscuit Evolution
> Biscuit currently provides LLM output validation against physics constraints.
> Future evolution: True physical model for safety-critical systems (not LLM-based).
> This would enable Biscuit for AV and other high-stakes domains.

### Other Future Features
| Feature | Notes |
|---------|-------|
| DDS Transport | ROS2 interoperability |
| Behavior Trees | Visual behavior design |
| Native Simulation | Bevy/Avian3D based |
| Gazebo Bridge | Use existing worlds |

---

## Feature Flags

All integrations are optional via Cargo features:

```toml
[features]
default = []
biscuit = ["dep:lair-biscuit"]   # LLM safety - service robots
bagel = ["dep:lair-bagel"]       # Smart logging
matcha = ["dep:lair-matcha"]     # Cloud sync
ml = ["dep:lair-ml"]             # ML inference
```

Build for your market:
```bash
lair build --features bagel,matcha           # AV, Industrial
lair build --features biscuit,bagel,matcha   # Service robots
lair build --no-default-features             # Minimal embedded
```

---

## Priority Legend

- **P0**: Must have for this release
- **P1**: Should have
- **P2**: Nice to have
- 🔴 Not started
- 🟡 In progress
- 🟢 Complete

---

## Philosophy

1. **Commercial/Industrial ONLY** - We build for production, not research
2. **Reliability > Features** - 24/7 uptime matters more than cool demos
3. **Fleet-First** - Single robot is easy; 500 robots is the real problem
4. **Certification-Ready** - Design for ISO 26262 from day one
5. **Modular** - Use what you need, leave what you don't

---

*LAIR is part of the Extelligence ecosystem alongside Bagel, Biscuit, and Matcha.*
