//! # Sencha Runtime & SDK
//!
//! Sencha is an experimental robotics systems platform built on Copper.
//! Timing, safety, and migration guarantees are research objectives; see the
//! repository README for implementation status and the research agenda.
//!
//! ## Quick start
//!
//! From a source checkout:
//!
//! ```bash
//! cargo run --locked -p sencha-cli -- --help
//! cargo run --locked -p simple-robot
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
//! ## Key Sencha types
//!
//! - [`SenchaSource`](sencha_core::prelude::SenchaSource): sensor / source tasks
//! - [`SenchaTask`](sencha_core::prelude::SenchaTask): transform / processing tasks
//! - [`SenchaSink`](sencha_core::prelude::SenchaSink): actuator / sink tasks
//! - [`SenchaContext`](sencha_core::prelude::SenchaContext): execution context
//! - [`SenchaResult`](sencha_core::prelude::SenchaResult): standard result type
//!
//! ## More information
//!
//! Visit <https://github.com/arunvenkatadri/sencha> for documentation, tutorials, and examples.

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

/// Sencha public API — re-exports of `sencha_core::prelude`.
pub mod sencha {
    pub use sencha_core::prelude::*;
    pub use sencha_derive::{sencha_runtime, sencha_task};
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

    // Sencha API names (re-exported from sencha-core)
    pub use sencha_core::prelude::{
        Clock, Freezable, SenchaConfig, SenchaContext, SenchaError, SenchaMsg, SenchaMsgPayload,
        SenchaResult, SenchaSink, SenchaSource, SenchaTask,
    };
    pub use sencha_derive::{sencha_runtime, sencha_task};
}
