use serde::{Deserialize, Serialize};
use bincode::{Decode, Encode};
use crate::standard::Header;
use crate::geometry::{Pose, PoseStamped, PoseWithCovariance, TwistWithCovariance};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Odometry {
    pub header: Header,
    pub child_frame_id: String,
    pub pose: PoseWithCovariance,
    pub twist: TwistWithCovariance,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Path {
    pub header: Header,
    pub poses: Vec<PoseStamped>,
}

/// Metadata about a 2D occupancy grid map.
#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct MapMetaData {
    /// Resolution of the map (m/cell).
    pub resolution: f32,
    /// Map width in cells.
    pub width: u32,
    /// Map height in cells.
    pub height: u32,
    /// Origin of the map (position of cell (0,0)).
    pub origin: Pose,
}

/// 2D occupancy grid map. Values: -1 = unknown, 0-100 = probability of occupancy.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct OccupancyGrid {
    pub header: Header,
    pub info: MapMetaData,
    pub data: Vec<i8>,
}
