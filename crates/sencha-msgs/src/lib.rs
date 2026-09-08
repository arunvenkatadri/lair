//! Sencha standard message types for robotics.
//!
//! Provides ROS-compatible message types that satisfy Copper's `CuMsgPayload` trait
//! for use on the zero-copy message bus.

pub mod standard;
pub mod geometry;
pub mod sensors;
pub mod navigation;
pub mod vehicle;

pub use standard::Header;
pub use geometry::{
    Point, Pose, PoseStamped, PoseWithCovariance, Quaternion, Transform, TransformStamped,
    Twist, TwistStamped, TwistWithCovariance, Vector3, Wrench,
};
pub use sensors::{
    Image, Imu, LaserScan, NavSatFix, NavSatStatus, PointCloud2, PointField,
};
pub use navigation::{MapMetaData, OccupancyGrid, Odometry, Path};
pub use vehicle::{ControlCommand, Gear, VehicleState};

#[cfg(test)]
mod tests {
    use bincode::{config, decode_from_slice, encode_to_vec};

    macro_rules! roundtrip_test {
        ($name:ident, $ty:ty) => {
            #[test]
            fn $name() {
                let original = <$ty>::default();
                let cfg = config::standard();
                let encoded = encode_to_vec(&original, cfg).expect("encode failed");
                let (decoded, _): ($ty, _) =
                    decode_from_slice(&encoded, cfg).expect("decode failed");
                assert_eq!(original, decoded);
            }
        };
    }

    // standard
    roundtrip_test!(roundtrip_header, super::Header);

    // geometry
    roundtrip_test!(roundtrip_vector3, super::Vector3);
    roundtrip_test!(roundtrip_point, super::Point);
    roundtrip_test!(roundtrip_quaternion, super::Quaternion);
    roundtrip_test!(roundtrip_pose, super::Pose);
    roundtrip_test!(roundtrip_pose_stamped, super::PoseStamped);
    roundtrip_test!(roundtrip_transform, super::Transform);
    roundtrip_test!(roundtrip_transform_stamped, super::TransformStamped);
    roundtrip_test!(roundtrip_twist, super::Twist);
    roundtrip_test!(roundtrip_twist_stamped, super::TwistStamped);
    roundtrip_test!(roundtrip_wrench, super::Wrench);
    roundtrip_test!(roundtrip_pose_with_covariance, super::PoseWithCovariance);
    roundtrip_test!(roundtrip_twist_with_covariance, super::TwistWithCovariance);

    // sensors
    roundtrip_test!(roundtrip_laser_scan, super::LaserScan);
    roundtrip_test!(roundtrip_image, super::Image);
    roundtrip_test!(roundtrip_imu, super::Imu);
    roundtrip_test!(roundtrip_point_field, super::PointField);
    roundtrip_test!(roundtrip_point_cloud2, super::PointCloud2);
    roundtrip_test!(roundtrip_nav_sat_status, super::NavSatStatus);
    roundtrip_test!(roundtrip_nav_sat_fix, super::NavSatFix);

    // navigation
    roundtrip_test!(roundtrip_odometry, super::Odometry);
    roundtrip_test!(roundtrip_path, super::Path);
    roundtrip_test!(roundtrip_map_meta_data, super::MapMetaData);
    roundtrip_test!(roundtrip_occupancy_grid, super::OccupancyGrid);

    // vehicle
    roundtrip_test!(roundtrip_gear, super::Gear);
    roundtrip_test!(roundtrip_vehicle_state, super::VehicleState);
    roundtrip_test!(roundtrip_control_command, super::ControlCommand);

    #[test]
    fn quaternion_default_is_identity() {
        let q = super::Quaternion::default();
        assert_eq!(q.x, 0.0);
        assert_eq!(q.y, 0.0);
        assert_eq!(q.z, 0.0);
        assert_eq!(q.w, 1.0);
    }
}
