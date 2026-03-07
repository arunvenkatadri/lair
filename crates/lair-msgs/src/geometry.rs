use serde::{Deserialize, Deserializer, Serialize, Serializer};
use bincode::{Decode, Encode};
use crate::standard::Header;

mod cov36_serde {
    use super::*;

    pub fn serialize<S: Serializer>(data: &[f64; 36], serializer: S) -> Result<S::Ok, S::Error> {
        data.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[f64; 36], D::Error> {
        let v: Vec<f64> = Vec::deserialize(deserializer)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 36 elements for covariance"))
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Unit quaternion representing orientation.
/// Default is identity rotation (0, 0, 0, 1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Quaternion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Default for Quaternion {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Pose {
    pub position: Point,
    pub orientation: Quaternion,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct PoseStamped {
    pub header: Header,
    pub pose: Pose,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Transform {
    pub translation: Vector3,
    pub rotation: Quaternion,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct TransformStamped {
    pub header: Header,
    pub child_frame_id: String,
    pub transform: Transform,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Twist {
    pub linear: Vector3,
    pub angular: Vector3,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct TwistStamped {
    pub header: Header,
    pub twist: Twist,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Wrench {
    pub force: Vector3,
    pub torque: Vector3,
}

/// 6x6 row-major covariance matrix (x, y, z, rotation about X, Y, Z).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct PoseWithCovariance {
    pub pose: Pose,
    #[serde(with = "cov36_serde")]
    pub covariance: [f64; 36],
}

impl Default for PoseWithCovariance {
    fn default() -> Self {
        Self { pose: Pose::default(), covariance: [0.0; 36] }
    }
}

/// 6x6 row-major covariance matrix (x, y, z, rotation about X, Y, Z).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct TwistWithCovariance {
    pub twist: Twist,
    #[serde(with = "cov36_serde")]
    pub covariance: [f64; 36],
}

impl Default for TwistWithCovariance {
    fn default() -> Self {
        Self { twist: Twist::default(), covariance: [0.0; 36] }
    }
}
