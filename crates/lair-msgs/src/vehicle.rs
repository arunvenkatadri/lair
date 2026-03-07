//! Vehicle-specific message types for autonomous driving and mobile robots.

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Transmission gear state.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum Gear {
    #[default]
    Park,
    Drive,
    Reverse,
    Neutral,
}

/// Full vehicle state snapshot.
#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct VehicleState {
    /// Position X (meters)
    pub x: f64,
    /// Position Y (meters)
    pub y: f64,
    /// Position Z (meters)
    pub z: f64,
    /// Heading angle (radians)
    pub heading: f64,
    /// Forward velocity (m/s)
    pub velocity: f64,
    /// Front wheel steering angle (radians)
    pub steering_angle: f64,
    /// Current gear
    pub gear: Gear,
    /// Battery state of charge (0.0–1.0)
    pub battery_level: f64,
}

/// Command sent to vehicle actuators.
#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct ControlCommand {
    /// Throttle command (0.0–1.0)
    pub throttle: f64,
    /// Brake command (0.0–1.0)
    pub brake: f64,
    /// Desired steering angle (radians)
    pub steering: f64,
    /// Desired gear
    pub gear: Gear,
}
