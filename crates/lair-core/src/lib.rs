//! LAIR Core — the public API surface for LAIR.
//!
//! This crate provides LAIR-branded re-exports over the internal Copper runtime types,
//! giving beta testers a clean, stable API while preserving full cu29 interop.

pub use cu29_clock as clock;
pub use cu29_traits as traits;
pub use cu29_value as value;

pub mod health;

// ── LAIR type aliases (concrete types) ──

/// Standard result type for all LAIR operations.
pub type LairResult<T> = cu29_traits::CuResult<T>;

/// Standard error type for all LAIR operations.
pub type LairError = cu29_traits::CuError;

/// Execution context passed to every task callback.
pub type LairContext = cu29_runtime::context::CuContext;

/// A timestamped message on the zero-copy bus.
pub type LairMsg<T> = cu29_runtime::cutask::CuMsg<T>;

/// Per-task configuration from the RON graph file.
pub type LairConfig = cu29_runtime::config::ComponentConfig;

/// High-precision monotonic clock.
pub type Clock = cu29_clock::RobotClock;

// ── LAIR trait re-exports ──

/// A source task that only produces messages (sensors, drivers).
pub use cu29_runtime::cutask::CuSrcTask as LairSource;

/// A transform task that consumes input and produces output.
pub use cu29_runtime::cutask::CuTask as LairTask;

/// A sink task that only consumes messages (actuators, loggers).
pub use cu29_runtime::cutask::CuSinkTask as LairSink;

/// Trait for snapshot/restore of task state (deterministic replay).
pub use cu29_runtime::cutask::Freezable;

/// Trait bound for message payloads on the zero-copy bus.
pub use cu29_runtime::cutask::CuMsgPayload as LairMsgPayload;

/// The prelude — import everything a LAIR task author needs.
pub mod prelude {
    pub use super::{Clock, Freezable, LairConfig, LairContext, LairError, LairMsg, LairMsgPayload, LairResult};

    // Trait re-exports with LAIR names
    pub use super::{LairSink, LairSource, LairTask};

    // Time types
    pub use cu29_clock::{CuDuration, CuTime};

    // Health monitoring
    pub use super::health::{
        Criticality, Heartbeat, HealthMonitor, HealthReport, HealthState,
    };

    // Message convenience macros
    pub use cu29_runtime::input_msg;
    pub use cu29_runtime::output_msg;
}
