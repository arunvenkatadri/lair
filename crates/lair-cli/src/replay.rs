//! `lair replay` — inspect a recorded MCAP log.
//!
//! LAIR records every message on the bus to MCAP (see `lair record`). This module
//! reads that log back: it summarizes what was captured (channels, message counts,
//! time span) and can dump individual records. It is the read side of the
//! record/replay loop and the foundation for replay-driven testing.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

/// Per-channel statistics gathered from a log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelStats {
    /// The channel's topic name.
    pub topic: String,
    /// The message schema name, if the channel declared one.
    pub schema: Option<String>,
    /// Number of messages on this channel.
    pub count: u64,
    /// Total payload bytes across all messages on this channel.
    pub total_bytes: u64,
}

/// A summary of a recorded MCAP log.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplaySummary {
    /// Total messages across all channels.
    pub message_count: u64,
    /// Earliest message `log_time` in nanoseconds, if any messages exist.
    pub start_time_ns: Option<u64>,
    /// Latest message `log_time` in nanoseconds, if any messages exist.
    pub end_time_ns: Option<u64>,
    /// Per-channel breakdown, sorted by topic.
    pub channels: Vec<ChannelStats>,
    /// True if the log ended mid-record (e.g. the robot was killed while writing).
    /// The summary still reflects every message read before the truncation.
    pub truncated: bool,
}

impl ReplaySummary {
    /// The wall-clock span of the recording in seconds (0 if fewer than two
    /// distinct timestamps).
    pub fn duration_secs(&self) -> f64 {
        match (self.start_time_ns, self.end_time_ns) {
            (Some(start), Some(end)) if end > start => (end - start) as f64 / 1e9,
            _ => 0.0,
        }
    }
}

/// Parses MCAP bytes into a [`ReplaySummary`].
pub fn summarize(data: &[u8]) -> Result<ReplaySummary> {
    // Reject input that isn't an MCAP at all up front, so we only ever *tolerate*
    // truncation of an otherwise-valid file (not arbitrary garbage).
    if !data.starts_with(mcap::MAGIC) {
        anyhow::bail!("not an MCAP file (bad magic header)");
    }
    let stream = mcap::MessageStream::new(data).context("failed to open MCAP stream")?;

    let mut per_channel: BTreeMap<String, ChannelStats> = BTreeMap::new();
    let mut summary = ReplaySummary::default();

    for message in stream {
        // A truncated log (killed robot) errors mid-stream. Keep what we read.
        let message = match message {
            Ok(m) => m,
            Err(_) => {
                summary.truncated = true;
                break;
            }
        };
        summary.message_count += 1;

        let t = message.log_time;
        summary.start_time_ns = Some(summary.start_time_ns.map_or(t, |s| s.min(t)));
        summary.end_time_ns = Some(summary.end_time_ns.map_or(t, |e| e.max(t)));

        let topic = message.channel.topic.clone();
        let entry = per_channel.entry(topic.clone()).or_insert_with(|| ChannelStats {
            topic,
            schema: message.channel.schema.as_ref().map(|s| s.name.clone()),
            count: 0,
            total_bytes: 0,
        });
        entry.count += 1;
        entry.total_bytes += message.data.len() as u64;
    }

    summary.channels = per_channel.into_values().collect();
    Ok(summary)
}

/// Reads `path` and prints a human-readable summary; with `limit`, also prints the
/// first `limit` message records.
pub fn replay_file(path: &Path, limit: Option<usize>) -> Result<()> {
    let data = std::fs::read(path)
        .with_context(|| format!("failed to read log file {}", path.display()))?;

    let summary = summarize(&data)?;

    println!("Replay summary for {}", path.display());
    println!("  messages : {}", summary.message_count);
    println!("  channels : {}", summary.channels.len());
    println!("  duration : {:.3} s", summary.duration_secs());
    if summary.truncated {
        println!("  warning  : log is truncated (ended mid-record); showing partial data");
    }
    if summary.channels.is_empty() {
        println!("  (no messages recorded)");
    } else {
        println!("  per channel:");
        for c in &summary.channels {
            let schema = c.schema.as_deref().unwrap_or("-");
            println!(
                "    {:<28} {:>8} msgs  {:>10} B  [{}]",
                c.topic, c.count, c.total_bytes, schema
            );
        }
    }

    if let Some(limit) = limit {
        println!("\n  first {limit} records:");
        let stream = mcap::MessageStream::new(&data).context("failed to reopen MCAP stream")?;
        for message in stream.take(limit) {
            let Ok(m) = message else { break };
            println!(
                "    t={:>15}ns  seq={:>6}  {:<24} {} B",
                m.log_time,
                m.sequence,
                m.channel.topic,
                m.data.len()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcap::records::MessageHeader;
    use std::io::Cursor;

    /// Builds a small in-memory MCAP with two channels for testing.
    fn sample_log() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = mcap::Writer::new(Cursor::new(&mut buf)).unwrap();
            let sensor_schema = w.add_schema("f32", "raw", b"f32").unwrap();
            let sensor = w
                .add_channel(sensor_schema, "/sensor", "raw", &BTreeMap::new())
                .unwrap();
            let actuator = w
                .add_channel(0, "/actuator", "raw", &BTreeMap::new())
                .unwrap();

            for i in 0..3u32 {
                w.write_to_known_channel(
                    &MessageHeader {
                        channel_id: sensor,
                        sequence: i,
                        log_time: 1_000 + i as u64 * 1_000_000,
                        publish_time: 1_000 + i as u64 * 1_000_000,
                    },
                    &[0u8; 4],
                )
                .unwrap();
            }
            w.write_to_known_channel(
                &MessageHeader {
                    channel_id: actuator,
                    sequence: 0,
                    log_time: 2_500,
                    publish_time: 2_500,
                },
                &[1u8; 8],
            )
            .unwrap();
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn summarize_counts_messages_and_channels() {
        let summary = summarize(&sample_log()).unwrap();
        assert_eq!(summary.message_count, 4);
        assert_eq!(summary.channels.len(), 2);
    }

    #[test]
    fn summarize_breaks_down_per_channel() {
        let summary = summarize(&sample_log()).unwrap();
        // Channels are sorted by topic: /actuator before /sensor.
        let actuator = &summary.channels[0];
        assert_eq!(actuator.topic, "/actuator");
        assert_eq!(actuator.count, 1);
        assert_eq!(actuator.total_bytes, 8);

        let sensor = &summary.channels[1];
        assert_eq!(sensor.topic, "/sensor");
        assert_eq!(sensor.count, 3);
        assert_eq!(sensor.total_bytes, 12);
        assert_eq!(sensor.schema.as_deref(), Some("f32"));
    }

    #[test]
    fn summarize_tracks_time_span() {
        let summary = summarize(&sample_log()).unwrap();
        assert_eq!(summary.start_time_ns, Some(1_000));
        assert_eq!(summary.end_time_ns, Some(2_000_000 + 1_000));
        assert!(summary.duration_secs() > 0.0);
    }

    #[test]
    fn summarize_empty_log() {
        let mut buf = Vec::new();
        {
            let mut w = mcap::Writer::new(Cursor::new(&mut buf)).unwrap();
            w.finish().unwrap();
        }
        let summary = summarize(&buf).unwrap();
        assert_eq!(summary.message_count, 0);
        assert!(summary.channels.is_empty());
        assert_eq!(summary.duration_secs(), 0.0);
    }

    #[test]
    fn summarize_rejects_garbage() {
        assert!(summarize(b"not an mcap file at all").is_err());
    }
}
