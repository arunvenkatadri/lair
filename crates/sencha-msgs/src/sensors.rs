use serde::{Deserialize, Serialize};
use bincode::{Decode, Encode};
use crate::standard::Header;
use crate::geometry::{Quaternion, Vector3};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct LaserScan {
    pub header: Header,
    /// Start angle of the scan (rad).
    pub angle_min: f32,
    /// End angle of the scan (rad).
    pub angle_max: f32,
    /// Angular distance between measurements (rad).
    pub angle_increment: f32,
    /// Time between measurements (s). 0 if not used.
    pub time_increment: f32,
    /// Time between full scans (s).
    pub scan_time: f32,
    /// Minimum valid range value (m).
    pub range_min: f32,
    /// Maximum valid range value (m).
    pub range_max: f32,
    /// Range data (m). Values outside [range_min, range_max] are invalid.
    pub ranges: Vec<f32>,
    /// Intensity data. Device-specific units. Empty if not available.
    pub intensities: Vec<f32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Image {
    pub header: Header,
    /// Image height in pixels.
    pub height: u32,
    /// Image width in pixels.
    pub width: u32,
    /// Encoding of pixels, e.g. "rgb8", "bgr8", "mono8", "16UC1".
    pub encoding: String,
    /// Is the data big-endian? Relevant for 16-bit encodings.
    pub is_bigendian: bool,
    /// Full row length in bytes.
    pub step: u32,
    /// Raw image data.
    pub data: Vec<u8>,
}

/// Inertial measurement unit data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct Imu {
    pub header: Header,
    pub orientation: Quaternion,
    /// Row-major 3x3 covariance. Set first element to -1 if unknown.
    pub orientation_covariance: [f64; 9],
    pub angular_velocity: Vector3,
    pub angular_velocity_covariance: [f64; 9],
    pub linear_acceleration: Vector3,
    pub linear_acceleration_covariance: [f64; 9],
}

impl Default for Imu {
    fn default() -> Self {
        Self {
            header: Header::default(),
            orientation: Quaternion::default(),
            orientation_covariance: [0.0; 9],
            angular_velocity: Vector3::default(),
            angular_velocity_covariance: [0.0; 9],
            linear_acceleration: Vector3::default(),
            linear_acceleration_covariance: [0.0; 9],
        }
    }
}

/// Describes a single field in a PointCloud2 message.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct PointField {
    pub name: String,
    /// Byte offset from the start of the point struct.
    pub offset: u32,
    /// Datatype (1=INT8, 2=UINT8, 3=INT16, 4=UINT16, 5=INT32, 6=UINT32, 7=FLOAT32, 8=FLOAT64).
    pub datatype: u8,
    /// Number of elements in the field.
    pub count: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct PointCloud2 {
    pub header: Header,
    /// Height of the point cloud (1 for unorganized).
    pub height: u32,
    /// Width of the point cloud.
    pub width: u32,
    /// Describes the channels and their layout.
    pub fields: Vec<PointField>,
    pub is_bigendian: bool,
    /// Length of a point in bytes.
    pub point_step: u32,
    /// Length of a row in bytes.
    pub row_step: u32,
    /// Point data.
    pub data: Vec<u8>,
    /// True if there are no invalid points.
    pub is_dense: bool,
}

/// GPS fix status.
#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct NavSatStatus {
    /// -1 = no fix, 0 = unaugmented fix, 1 = SBAS, 2 = GBAS.
    pub status: i8,
    /// Bitmask: 1=GPS, 2=GLONASS, 4=Compass, 8=Galileo.
    pub service: u16,
}

/// GPS fix with position covariance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct NavSatFix {
    pub header: Header,
    pub status: NavSatStatus,
    /// Degrees. Positive is north of equator; negative is south.
    pub latitude: f64,
    /// Degrees. Positive is east of prime meridian; negative is west.
    pub longitude: f64,
    /// Altitude in meters above the WGS 84 ellipsoid. NaN if unknown.
    pub altitude: f64,
    /// Row-major 3x3 position covariance in ENU frame (m^2).
    pub position_covariance: [f64; 9],
    /// 0=unknown, 1=approximated, 2=diagonal known, 3=known.
    pub position_covariance_type: u8,
}

impl Default for NavSatFix {
    fn default() -> Self {
        Self {
            header: Header::default(),
            status: NavSatStatus::default(),
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            position_covariance: [0.0; 9],
            position_covariance_type: 0,
        }
    }
}
