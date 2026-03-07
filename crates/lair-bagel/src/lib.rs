//! LAIR Bagel — structured logging and telemetry.
//!
//! Re-exports the MCAP logging backend from `cu29-unifiedlog` and provides
//! convenience helpers for setting up LAIR logging with sensible defaults.

#[cfg(feature = "mcap")]
pub use cu29_unifiedlog::mcap_backend::*;

pub use cu29_unifiedlog::{stream_write, UnifiedLoggerBuilder, UnifiedLoggerWrite, UnifiedLogger};

use cu29_clock::RobotClock;
use cu29_log_runtime::LoggerRuntime;
use cu29_runtime::curuntime::CopperContext;
use cu29_traits::{CuResult, UnifiedLogType, with_cause, CuError};
use simplelog::TermLogger;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Set up LAIR logging with MCAP defaults (LZ4 compression).
///
/// This is a convenience wrapper around `basic_copper_setup` that
/// uses good defaults for LAIR applications.
pub fn basic_lair_setup(
    log_path: &Path,
    clock: Option<RobotClock>,
) -> CuResult<CopperContext> {
    let logger = UnifiedLoggerBuilder::new()
        .write(true)
        .create(true)
        .file_base_name(log_path)
        .preallocated_size(1024 * 1024 * 10)
        .build()
        .map_err(|e| with_cause("Failed to create LAIR logger", e))?;

    let logger = match logger {
        UnifiedLogger::Write(logger) => logger,
        UnifiedLogger::Read(_) => {
            return Err(CuError::from(
                "UnifiedLoggerBuilder did not create a write-capable logger",
            ));
        }
    };

    let unified_logger = Arc::new(Mutex::new(logger));
    let structured_stream = stream_write(
        unified_logger.clone(),
        UnifiedLogType::StructuredLogLine,
        4096 * 10,
    )?;

    let extra: Option<TermLogger> = None;
    let clock = clock.unwrap_or_default();
    let structured_logging = LoggerRuntime::init(clock.clone(), structured_stream, extra);

    Ok(CopperContext {
        unified_logger: unified_logger.clone(),
        logger_runtime: structured_logging,
        clock,
    })
}
