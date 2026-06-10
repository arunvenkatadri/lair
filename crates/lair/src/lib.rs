//! # LAIR Runtime & SDK
//!
//! LAIR is a Rust engine for building deterministic, safety-critical autonomous
//! robots. Define a task graph, compile once, and get sub-microsecond latency
//! from Linux workstations all the way down to bare-metal MPU builds.
//!
//! ## Quick start
//!
//! ```bash
//! cargo install lair-cli
//! lair new my_robot
//! cd my_robot
//! cargo run
//! ```
//!
//! ## Feature flags
//!
//! - `default` = `["std", "textlogs", "units"]`
//! - `units`: exposes `cu29::units` (re-export of `cu29-units`)
//! - `std`: host/runtime support
//! - `reflect`: reflection support for runtime and units types
//! - `textlogs`: text logging derive support
//! - `remote-debug`: remote debug transport support
//!
//! ## Key LAIR types
//!
//! - [`LairSource`](lair_core::prelude::LairSource): sensor / source tasks
//! - [`LairTask`](lair_core::prelude::LairTask): transform / processing tasks
//! - [`LairSink`](lair_core::prelude::LairSink): actuator / sink tasks
//! - [`LairContext`](lair_core::prelude::LairContext): execution context
//! - [`LairResult`](lair_core::prelude::LairResult): standard result type
//!
//! ## More information
//!
//! Visit <https://lair.dev> for documentation, tutorials, and examples.

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(not(feature = "std"))]
extern crate alloc;

pub use cu29_derive::{bundle_resources, resources};
pub use cu29_runtime::config;
pub use cu29_runtime::context;
pub use cu29_runtime::copperlist;
#[cfg(feature = "std")]
pub use cu29_runtime::cuasynctask;
pub use cu29_runtime::cubridge;
pub use cu29_runtime::curuntime;
pub use cu29_runtime::cutask;
#[cfg(feature = "std")]
pub use cu29_runtime::debug;
pub use cu29_runtime::input_msg;
pub use cu29_runtime::monitoring;
pub use cu29_runtime::output_msg;
pub use cu29_runtime::payload;
pub use cu29_runtime::reflect;
pub use cu29_runtime::reflect as bevy_reflect;
#[cfg(feature = "remote-debug")]
pub use cu29_runtime::remote_debug;
pub use cu29_runtime::resource;
pub use cu29_runtime::rx_channels;
#[cfg(feature = "std")]
pub use cu29_runtime::simulation;
pub use cu29_runtime::tx_channels;

#[cfg(feature = "rtsan")]
pub mod rtsan {
    pub use rtsan_standalone::*;
}

#[cfg(not(feature = "rtsan"))]
pub mod rtsan {
    use core::ffi::CStr;

    #[derive(Default)]
    pub struct ScopedSanitizeRealtime;

    #[derive(Default)]
    pub struct ScopedDisabler;

    #[inline]
    pub fn realtime_enter() {}

    #[inline]
    pub fn realtime_exit() {}

    #[inline]
    pub fn disable() {}

    #[inline]
    pub fn enable() {}

    #[inline]
    pub fn ensure_initialized() {}

    #[allow(unused_variables)]
    pub fn notify_blocking_call(_function_name: &'static CStr) {}
}

pub use bincode;
pub use cu29_clock as clock;
#[cfg(feature = "units")]
pub use cu29_units as units;
#[cfg(feature = "defmt")]
pub mod defmt {
    pub use defmt::{debug, error, info, warn};
}
#[cfg(feature = "std")]
pub use cu29_runtime::config::read_configuration;
pub use cu29_traits::*;

#[cfg(feature = "std")]
pub use rayon;

// defmt shims re-exported for proc-macro call sites
#[cfg(all(feature = "defmt", not(feature = "std")))]
#[macro_export]
macro_rules! defmt_debug {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::defmt::debug!($fmt $(, $arg)*);
    }
}
#[cfg(not(all(feature = "defmt", not(feature = "std"))))]
#[macro_export]
macro_rules! defmt_debug {
    ($($tt:tt)*) => {{}};
}

#[cfg(all(feature = "defmt", not(feature = "std")))]
#[macro_export]
macro_rules! defmt_info {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::defmt::info!($fmt $(, $arg)*);
    }
}
#[cfg(not(all(feature = "defmt", not(feature = "std"))))]
#[macro_export]
macro_rules! defmt_info {
    ($($tt:tt)*) => {{}};
}

#[cfg(all(feature = "defmt", not(feature = "std")))]
#[macro_export]
macro_rules! defmt_warn {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::defmt::warn!($fmt $(, $arg)*);
    }
}
#[cfg(not(all(feature = "defmt", not(feature = "std"))))]
#[macro_export]
macro_rules! defmt_warn {
    ($($tt:tt)*) => {{}};
}

#[cfg(all(feature = "defmt", not(feature = "std")))]
#[macro_export]
macro_rules! defmt_error {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::defmt::error!($fmt $(, $arg)*);
    }
}
#[cfg(not(all(feature = "defmt", not(feature = "std"))))]
#[macro_export]
macro_rules! defmt_error {
    ($($tt:tt)*) => {{}};
}

/// LAIR public API — re-exports of `lair_core::prelude`.
pub mod lair {
    pub use lair_core::prelude::*;
    pub use lair_derive::{lair_runtime, lair_task};
}

pub mod prelude {
    pub use crate::bevy_reflect;
    #[cfg(feature = "units")]
    pub use crate::units;
    pub use crate::{defmt_debug, defmt_error, defmt_info, defmt_warn};
    #[cfg(feature = "std")]
    pub use ctrlc;
    pub use cu29_clock::*;
    pub use cu29_derive::*; // includes resources! proc macro
    pub use cu29_log::*;
    pub use cu29_log::{
        __cu29_defmt_debug, __cu29_defmt_error, __cu29_defmt_info, __cu29_defmt_warn,
    };
    pub use cu29_log_derive::*;
    pub use cu29_log_runtime::*;
    pub use cu29_runtime::app::*;
    pub use cu29_runtime::config::*;
    pub use cu29_runtime::context::*;
    pub use cu29_runtime::copperlist::*;
    pub use cu29_runtime::cubridge::*;
    pub use cu29_runtime::curuntime::*;
    pub use cu29_runtime::cutask::*;
    #[cfg(feature = "std")]
    pub use cu29_runtime::debug::*;
    pub use cu29_runtime::input_msg;
    pub use cu29_runtime::monitoring::*;
    pub use cu29_runtime::output_msg;
    pub use cu29_runtime::payload::*;
    #[cfg(feature = "reflect")]
    pub use cu29_runtime::reflect::serde as reflect_serde;
    #[cfg(feature = "reflect")]
    pub use cu29_runtime::reflect::serde::{
        ReflectSerializer, SerializationData, TypedReflectSerializer,
    };
    pub use cu29_runtime::reflect::{
        GetTypeRegistration, Reflect, ReflectTaskIntrospection, ReflectTypePath, TypeInfo,
        TypePath, TypeRegistry, dump_type_registry_schema,
    };
    #[cfg(feature = "remote-debug")]
    pub use cu29_runtime::remote_debug::*;
    pub use cu29_runtime::resource::*;
    pub use cu29_runtime::rx_channels;
    #[cfg(feature = "std")]
    pub use cu29_runtime::simulation::*;
    pub use cu29_runtime::tx_channels;
    pub use cu29_runtime::*;
    pub use cu29_traits::*;
    pub use cu29_unifiedlog::*;
    pub use cu29_value::Value;
    pub use cu29_value::to_value;
    #[cfg(feature = "std")]
    pub use pool::*;
    pub use serde::Serialize;

    // LAIR API names (re-exported from lair-core)
    pub use lair_core::prelude::{
        Clock, Freezable, LairConfig, LairContext, LairError, LairMsg, LairMsgPayload,
        LairResult, LairSink, LairSource, LairTask,
    };
    // Health monitoring / watchdogs
    pub use lair_core::prelude::{
        Criticality, Heartbeat, HealthMonitor, HealthReport, HealthState,
    };
    pub use lair_derive::{lair_runtime, lair_task};
}
