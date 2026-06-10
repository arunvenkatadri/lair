//! Typed record / replay for virtual sensors.
//!
//! This is the playback side of LAIR's record/replay loop. [`encode_payloads`] /
//! [`write_payloads`] serialize a typed message stream to MCAP (each message a
//! `bincode`-encoded payload on a topic), and [`ReplaySource`] is a [`LairSource`]
//! that reads such a log back and re-emits the messages onto the bus in order —
//! a *virtual sensor* you can wire into a graph in place of a real driver, so
//! downstream tasks can be exercised deterministically against recorded data.
//!
//! [`LairSource`]: cu29_runtime::cutask::CuSrcTask
//!
//! # Round trip
//!
//! ```
//! use lair_bagel::replay::{encode_payloads, read_payloads_from_bytes, ReplaySource};
//!
//! // Record a stream of readings...
//! let readings: Vec<f32> = vec![1.0, 2.0, 3.0];
//! let bytes = encode_payloads("/imu", "f32", &readings).unwrap();
//!
//! // ...and read it back.
//! let decoded: Vec<f32> = read_payloads_from_bytes(&bytes, "/imu").unwrap();
//! assert_eq!(decoded, readings);
//!
//! // A ReplaySource will re-emit them one per `process`.
//! let source = ReplaySource::from_messages(decoded);
//! assert_eq!(source.remaining(), 3);
//! ```

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;

use bincode::config::standard;
use bincode::{Decode, Encode};
use cu29_runtime::config::ComponentConfig;
use cu29_runtime::context::CuContext;
use cu29_runtime::cutask::{CuMsg, CuMsgPayload, CuSrcTask, Freezable};
use cu29_runtime::output_msg;
use cu29_traits::{CuError, CuResult};
use mcap::records::MessageHeader;

/// MCAP encoding tag for LAIR's bincode payload format.
const LAIR_BINCODE: &str = "lair/bincode";

fn err(context: &str, e: impl core::fmt::Display) -> CuError {
    CuError::from(format!("{context}: {e}"))
}

/// Serializes `msgs` to an in-memory MCAP buffer, one `bincode`-encoded payload
/// per message on `topic` (with the given `schema` name for tooling).
pub fn encode_payloads<T: Encode>(topic: &str, schema: &str, msgs: &[T]) -> CuResult<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut writer = mcap::Writer::new(Cursor::new(&mut buf))
            .map_err(|e| err("failed to start MCAP writer", e))?;
        let schema_id = writer
            .add_schema(schema, LAIR_BINCODE, b"")
            .map_err(|e| err("failed to add MCAP schema", e))?;
        let channel = writer
            .add_channel(schema_id, topic, LAIR_BINCODE, &BTreeMap::new())
            .map_err(|e| err("failed to add MCAP channel", e))?;
        for (i, msg) in msgs.iter().enumerate() {
            let bytes = bincode::encode_to_vec(msg, standard())
                .map_err(|e| err("failed to encode payload", e))?;
            writer
                .write_to_known_channel(
                    &MessageHeader {
                        channel_id: channel,
                        sequence: i as u32,
                        log_time: i as u64,
                        publish_time: i as u64,
                    },
                    &bytes,
                )
                .map_err(|e| err("failed to write MCAP message", e))?;
        }
        writer.finish().map_err(|e| err("failed to finish MCAP", e))?;
    }
    Ok(buf)
}

/// Writes `msgs` to an MCAP file at `path` (see [`encode_payloads`]).
pub fn write_payloads<T: Encode>(
    path: &Path,
    topic: &str,
    schema: &str,
    msgs: &[T],
) -> CuResult<()> {
    let buf = encode_payloads(topic, schema, msgs)?;
    std::fs::write(path, buf).map_err(|e| err("failed to write log file", e))
}

/// Decodes all `T` payloads recorded on `topic` from in-memory MCAP `data`.
///
/// Truncated logs (e.g. a killed robot) are tolerated: decoding stops at the first
/// unreadable record and returns everything read so far.
pub fn read_payloads_from_bytes<T: Decode<()>>(data: &[u8], topic: &str) -> CuResult<Vec<T>> {
    // Reject non-MCAP input up front so we only ever *tolerate* truncation of an
    // otherwise-valid file, not arbitrary garbage.
    if !data.starts_with(mcap::MAGIC) {
        return Err(CuError::from("not an MCAP file (bad magic header)".to_string()));
    }
    let stream =
        mcap::MessageStream::new(data).map_err(|e| err("failed to open MCAP stream", e))?;
    let mut out = Vec::new();
    for message in stream {
        let message = match message {
            Ok(m) => m,
            Err(_) => break, // tolerate truncation
        };
        if message.channel.topic != topic {
            continue;
        }
        let (value, _) = bincode::decode_from_slice::<T, _>(&message.data, standard())
            .map_err(|e| err("failed to decode payload", e))?;
        out.push(value);
    }
    Ok(out)
}

/// Reads all `T` payloads recorded on `topic` from the MCAP file at `path`.
pub fn read_payloads<T: Decode<()>>(path: &Path, topic: &str) -> CuResult<Vec<T>> {
    let data = std::fs::read(path).map_err(|e| err("failed to read log file", e))?;
    read_payloads_from_bytes(&data, topic)
}

/// A [`LairSource`](cu29_runtime::cutask::CuSrcTask) that replays a recorded
/// message stream onto the bus — a virtual sensor.
///
/// Each `process` emits the next recorded payload. When the recording is
/// exhausted the source emits nothing, unless looping is enabled (in which case it
/// wraps back to the start).
///
/// Wired in a `.ron` graph (config keys: `path`, `topic`, optional `loop`):
///
/// ```ron
/// ( id: "imu", type: "lair_bagel::replay::ReplaySource<lair_msgs::Imu>",
///   config: { "path": "logs/run1.mcap", "topic": "/imu", "loop": false } )
/// ```
pub struct ReplaySource<T> {
    messages: Vec<T>,
    index: usize,
    loop_playback: bool,
}

impl<T> ReplaySource<T> {
    /// Builds a replay source directly from an in-memory message list.
    pub fn from_messages(messages: Vec<T>) -> Self {
        Self { messages, index: 0, loop_playback: false }
    }

    /// Enables or disables looping playback.
    pub fn looping(mut self, loop_playback: bool) -> Self {
        self.loop_playback = loop_playback;
        self
    }

    /// Number of messages not yet replayed (always the full length when looping).
    pub fn remaining(&self) -> usize {
        if self.loop_playback {
            self.messages.len()
        } else {
            self.messages.len().saturating_sub(self.index)
        }
    }

    /// Whether playback has finished (never true when looping or empty-but-looping).
    pub fn is_exhausted(&self) -> bool {
        !self.loop_playback && self.index >= self.messages.len()
    }

    /// Total number of recorded messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the recording is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl<T: CuMsgPayload + 'static> Freezable for ReplaySource<T> {}

impl<T: CuMsgPayload + 'static> CuSrcTask for ReplaySource<T> {
    type Output<'m> = output_msg!(T);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where
        Self: Sized,
    {
        let path = config.and_then(|c| c.get::<String>("path").ok().flatten());
        let topic = config
            .and_then(|c| c.get::<String>("topic").ok().flatten())
            .unwrap_or_else(|| "/replay".to_string());
        let loop_playback = config
            .and_then(|c| c.get::<bool>("loop").ok().flatten())
            .unwrap_or(false);

        let messages = match path {
            Some(p) => read_payloads::<T>(Path::new(&p), &topic)?,
            None => Vec::new(),
        };
        Ok(Self { messages, index: 0, loop_playback })
    }

    fn process(&mut self, _ctx: &CuContext, output: &mut Self::Output<'_>) -> CuResult<()> {
        if self.index >= self.messages.len() {
            if self.loop_playback && !self.messages.is_empty() {
                self.index = 0;
            } else {
                output.clear_payload();
                return Ok(());
            }
        }
        output.set_payload(self.messages[self.index].clone());
        self.index += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_payloads_through_mcap() {
        let readings: Vec<f32> = vec![1.0, 2.5, -3.0, 4.25];
        let bytes = encode_payloads("/imu", "f32", &readings).unwrap();
        let decoded: Vec<f32> = read_payloads_from_bytes(&bytes, "/imu").unwrap();
        assert_eq!(decoded, readings);
    }

    #[test]
    fn read_filters_by_topic() {
        let bytes = encode_payloads("/imu", "f32", &[1.0f32, 2.0]).unwrap();
        let other: Vec<f32> = read_payloads_from_bytes(&bytes, "/lidar").unwrap();
        assert!(other.is_empty());
    }

    #[test]
    fn rejects_non_mcap_bytes() {
        let r: CuResult<Vec<f32>> = read_payloads_from_bytes(b"not mcap", "/imu");
        assert!(r.is_err());
    }

    #[test]
    fn replay_source_emits_in_order_then_stops() {
        let (ctx, _clock) = CuContext::new_mock_clock();
        let mut src = ReplaySource::from_messages(vec![10.0f32, 20.0, 30.0]);
        assert_eq!(src.remaining(), 3);

        let mut seen = Vec::new();
        for _ in 0..5 {
            let mut out: CuMsg<f32> = CuMsg::new(None);
            src.process(&ctx, &mut out).unwrap();
            if let Some(v) = out.payload() {
                seen.push(*v);
            }
        }
        assert_eq!(seen, vec![10.0, 20.0, 30.0]);
        assert!(src.is_exhausted());
        assert_eq!(src.remaining(), 0);
    }

    #[test]
    fn replay_source_loops_when_configured() {
        let (ctx, _clock) = CuContext::new_mock_clock();
        let mut src = ReplaySource::from_messages(vec![1.0f32, 2.0]).looping(true);

        let mut seen = Vec::new();
        for _ in 0..5 {
            let mut out: CuMsg<f32> = CuMsg::new(None);
            src.process(&ctx, &mut out).unwrap();
            seen.push(*out.payload().unwrap());
        }
        assert_eq!(seen, vec![1.0, 2.0, 1.0, 2.0, 1.0]);
        assert!(!src.is_exhausted());
    }

    #[test]
    fn empty_replay_source_emits_nothing() {
        let (ctx, _clock) = CuContext::new_mock_clock();
        let mut src = ReplaySource::<f32>::from_messages(vec![]);
        let mut out: CuMsg<f32> = CuMsg::new(Some(99.0));
        src.process(&ctx, &mut out).unwrap();
        assert_eq!(out.payload(), None);
    }

    #[test]
    fn full_record_then_replay_loop() {
        // Record -> read -> replay reproduces the original sequence.
        let original: Vec<f32> = vec![0.1, 0.2, 0.3];
        let bytes = encode_payloads("/sensor", "f32", &original).unwrap();
        let decoded: Vec<f32> = read_payloads_from_bytes(&bytes, "/sensor").unwrap();

        let (ctx, _clock) = CuContext::new_mock_clock();
        let mut src = ReplaySource::from_messages(decoded);
        let mut replayed = Vec::new();
        while !src.is_exhausted() {
            let mut out: CuMsg<f32> = CuMsg::new(None);
            src.process(&ctx, &mut out).unwrap();
            if let Some(v) = out.payload() {
                replayed.push(*v);
            }
        }
        assert_eq!(replayed, original);
    }
}
