use cu29_clock::CuTime;
use serde::{Deserialize, Serialize};
use bincode::{Decode, Encode};

/// Standard message header, present on all stamped messages.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Header {
    /// Monotonic timestamp from the robot clock.
    pub timestamp: CuTime,
    /// Coordinate frame this data is associated with.
    pub frame_id: String,
    /// Sequence number, incremented per message.
    pub sequence: u32,
}
