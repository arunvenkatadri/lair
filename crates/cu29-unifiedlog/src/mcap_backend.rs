//! MCAP file backend for the unified logger.
//!
//! Replaces the mmap-based binary format with industry-standard MCAP,
//! readable by Foxglove Studio, Matcha, Bagel, and the broader ROS ecosystem.

use crate::{SectionHandle, SectionHeader, SectionStorage, UnifiedLogStatus, UnifiedLogWrite};

use bincode::config::standard;
use bincode::error::EncodeError;
use bincode::Encode;
use cu29_traits::{CuResult, UnifiedLogType};
use mcap::records::MessageHeader;
use mcap::write::{WriteOptions, Writer};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Maps a `UnifiedLogType` to its MCAP channel topic name.
/// The legacy `/lair/*` namespace is retained for recorded-data compatibility.
fn topic_for(entry_type: UnifiedLogType) -> &'static str {
    match entry_type {
        UnifiedLogType::StructuredLogLine => "/lair/log",
        UnifiedLogType::CopperList => "/lair/copperlist",
        UnifiedLogType::FrozenTasks => "/lair/keyframes",
        UnifiedLogType::RuntimeLifecycle => "/lair/lifecycle",
        UnifiedLogType::LastEntry => "/lair/end",
        UnifiedLogType::Empty => "/lair/empty",
    }
}

// ── McapSectionStorage ──────────────────────────────────────────────

/// A section storage backed by an MCAP channel.
///
/// Each `append` call writes a single MCAP message (bincode-encoded payload)
/// to the shared MCAP writer under the channel assigned at construction time.
pub struct McapSectionStorage {
    writer: Arc<Mutex<Writer<BufWriter<File>>>>,
    channel_id: u16,
    sequence: u32,
    total_bytes: usize,
}

impl McapSectionStorage {
    pub fn new(writer: Arc<Mutex<Writer<BufWriter<File>>>>, channel_id: u16) -> Self {
        Self {
            writer,
            channel_id,
            sequence: 0,
            total_bytes: 0,
        }
    }
}

impl SectionStorage for McapSectionStorage {
    fn initialize<E: Encode>(&mut self, _header: &E) -> Result<usize, EncodeError> {
        // MCAP doesn't have mutable section headers — no-op.
        Ok(0)
    }

    fn post_update_header<E: Encode>(&mut self, _header: &E) -> Result<usize, EncodeError> {
        // MCAP doesn't have mutable section headers — no-op.
        Ok(0)
    }

    fn append<E: Encode>(&mut self, entry: &E) -> Result<usize, EncodeError> {
        let data = bincode::encode_to_vec(entry, standard())?;
        let len = data.len();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp_ns = now.as_nanos() as u64;

        let header = MessageHeader {
            channel_id: self.channel_id,
            sequence: self.sequence,
            log_time: timestamp_ns,
            publish_time: timestamp_ns,
        };

        self.writer
            .lock()
            .map_err(|_| EncodeError::Other("MCAP writer mutex poisoned"))?
            .write_to_known_channel(&header, &data)
            .map_err(|_| EncodeError::Other("failed to write MCAP message"))?;

        self.sequence += 1;
        self.total_bytes += len;
        Ok(len)
    }

    fn flush(&mut self) -> CuResult<usize> {
        self.writer
            .lock()
            .map_err(|e| {
                cu29_traits::CuError::from("MCAP writer mutex poisoned")
                    .add_cause(&e.to_string())
            })?
            .flush()
            .map_err(|e| {
                cu29_traits::CuError::from("failed to flush MCAP writer")
                    .add_cause(&e.to_string())
            })?;
        Ok(self.total_bytes)
    }
}

// ── McapUnifiedLoggerWrite ──────────────────────────────────────────

/// The MCAP-backed unified logger writer.
///
/// Implements `UnifiedLogWrite<McapSectionStorage>` so the entire upstream
/// pipeline (macros, LogStream, stream_write) works unchanged.
pub struct McapUnifiedLoggerWrite {
    writer: Arc<Mutex<Writer<BufWriter<File>>>>,
    total_bytes: usize,
}

impl McapUnifiedLoggerWrite {
    /// Create a new MCAP logger writing to the given file.
    pub fn new<W: Into<BufWriter<File>>>(
        buf_writer: W,
        opts: WriteOptions,
    ) -> io::Result<Self> {
        let writer = opts
            .create(buf_writer.into())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            total_bytes: 0,
        })
    }

    fn register_channel(&mut self, entry_type: UnifiedLogType) -> CuResult<u16> {
        let topic = topic_for(entry_type);

        let mut w = self.writer.lock().map_err(|e| {
            cu29_traits::CuError::from("MCAP writer mutex poisoned")
                .add_cause(&e.to_string())
        })?;

        let schema_id = w
            .add_schema(topic, "bincode", &[])
            .map_err(|e| {
                cu29_traits::CuError::from("failed to add MCAP schema")
                    .add_cause(&e.to_string())
            })?;

        let channel_id = w
            .add_channel(schema_id, topic, "bincode", &BTreeMap::new())
            .map_err(|e| {
                cu29_traits::CuError::from("failed to add MCAP channel")
                    .add_cause(&e.to_string())
            })?;

        Ok(channel_id)
    }
}

impl UnifiedLogWrite<McapSectionStorage> for McapUnifiedLoggerWrite {
    fn add_section(
        &mut self,
        entry_type: UnifiedLogType,
        _requested_section_size: usize,
    ) -> CuResult<SectionHandle<McapSectionStorage>> {
        let channel_id = self.register_channel(entry_type)?;

        let header = SectionHeader {
            entry_type,
            ..SectionHeader::default()
        };
        let storage = McapSectionStorage::new(Arc::clone(&self.writer), channel_id);
        SectionHandle::create(header, storage)
    }

    fn flush_section(&mut self, section: &mut SectionHandle<McapSectionStorage>) {
        section.mark_closed();
        if let Ok(bytes) = section.get_storage_mut().flush() {
            self.total_bytes += bytes;
        }
    }

    fn status(&self) -> UnifiedLogStatus {
        UnifiedLogStatus {
            total_used_space: self.total_bytes,
            total_allocated_space: self.total_bytes, // MCAP grows dynamically
        }
    }
}

impl Drop for McapUnifiedLoggerWrite {
    fn drop(&mut self) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.finish();
        }
    }
}

// ── McapLoggerBuilder ───────────────────────────────────────────────

/// Holds either read or write side of the MCAP datalogger.
///
/// The `Read` variant exists for API compatibility with the mmap backend's
/// `UnifiedLogger` enum; MCAP reading is handled by the `mcap` crate directly.
pub enum McapUnifiedLogger {
    Read(()),
    Write(McapUnifiedLoggerWrite),
}

/// Builder mirroring `MmapUnifiedLoggerBuilder`'s API for MCAP output.
pub struct McapLoggerBuilder {
    file_path: Option<PathBuf>,
    write_options: WriteOptions,
    write: bool,
    create: bool,
}

impl Default for McapLoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl McapLoggerBuilder {
    pub fn new() -> Self {
        Self {
            file_path: None,
            write_options: WriteOptions::new().compression(Some(mcap::Compression::Lz4)),
            write: false,
            create: false,
        }
    }

    /// Set the output file path.
    /// Unlike the mmap builder, this is the exact path (no _0, _1 suffixes).
    pub fn file_base_name(mut self, path: &Path) -> Self {
        self.file_path = Some(path.to_path_buf());
        self
    }

    /// Ignored for MCAP (kept for API compatibility with mmap builder).
    pub fn preallocated_size(self, _size: usize) -> Self {
        self
    }

    pub fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    pub fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Set MCAP compression (None, Lz4, or Zstd).
    pub fn compression(mut self, compression: Option<mcap::Compression>) -> Self {
        self.write_options = self.write_options.compression(compression);
        self
    }

    /// Set custom WriteOptions (replaces defaults).
    pub fn write_options(mut self, opts: WriteOptions) -> Self {
        self.write_options = opts;
        self
    }

    pub fn build(self) -> io::Result<McapUnifiedLogger> {
        let path = self.file_path.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "File path is required")
        })?;

        if !self.write || !self.create {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "MCAP backend currently only supports write+create mode",
            ));
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(&path)?;
        let buf_writer = BufWriter::new(file);
        let logger = McapUnifiedLoggerWrite::new(buf_writer, self.write_options)?;
        Ok(McapUnifiedLogger::Write(logger))
    }
}
