//! Sencha Core — the public API surface for Sencha.
//!
//! This crate provides Sencha-branded re-exports over the internal Copper runtime types,
//! giving beta testers a clean, stable API while preserving full cu29 interop.

pub use cu29_clock as clock;
pub use cu29_traits as traits;
pub use cu29_value as value;

// ── Sencha type aliases (concrete types) ──

/// Standard result type for all Sencha operations.
pub type SenchaResult<T> = cu29_traits::CuResult<T>;

/// Standard error type for all Sencha operations.
pub type SenchaError = cu29_traits::CuError;

/// Execution context passed to every task callback.
pub type SenchaContext = cu29_runtime::context::CuContext;

/// A timestamped message on the zero-copy bus.
pub type SenchaMsg<T> = cu29_runtime::cutask::CuMsg<T>;

/// Per-task configuration from the RON graph file.
pub type SenchaConfig = cu29_runtime::config::ComponentConfig;

/// High-precision monotonic clock.
pub type Clock = cu29_clock::RobotClock;

// ── Sencha trait re-exports ──

/// A source task that only produces messages (sensors, drivers).
pub use cu29_runtime::cutask::CuSrcTask as SenchaSource;

/// A transform task that consumes input and produces output.
pub use cu29_runtime::cutask::CuTask as SenchaTask;

/// A sink task that only consumes messages (actuators, loggers).
pub use cu29_runtime::cutask::CuSinkTask as SenchaSink;

/// Trait for snapshot/restore of task state (deterministic replay).
pub use cu29_runtime::cutask::Freezable;

/// Trait bound for message payloads on the zero-copy bus.
pub use cu29_runtime::cutask::CuMsgPayload as SenchaMsgPayload;

/// The prelude — import everything a Sencha task author needs.
pub mod prelude {
    pub use super::{Clock, Freezable, SenchaConfig, SenchaContext, SenchaError, SenchaMsg, SenchaMsgPayload, SenchaResult};

    // Trait re-exports with Sencha names
    pub use super::{SenchaSink, SenchaSource, SenchaTask};

    // Time types
    pub use cu29_clock::{CuDuration, CuTime};

    // Message convenience macros
    pub use cu29_runtime::input_msg;
    pub use cu29_runtime::output_msg;
}
