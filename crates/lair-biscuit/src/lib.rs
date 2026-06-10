//! LAIR Biscuit — physics-constrained safety policy engine.
//!
//! Biscuit is the safety layer that every control command flows through before it
//! reaches a vehicle's actuators. It enforces two classes of constraints:
//!
//! 1. **Static bounds & invariants** — actuator commands must be finite and within
//!    their physical range (throttle/brake in `0.0..=1.0`, steering within the
//!    mechanical lock), and mutually-exclusive actuators must not be commanded
//!    together (you cannot brake and accelerate at the same time).
//! 2. **Physics-constrained, state-aware checks** — given the current
//!    [`VehicleState`], commands are rejected when they would violate the vehicle's
//!    dynamic envelope: shifting gears at speed, accelerating past the speed limit,
//!    or steering hard enough to roll the vehicle at the current velocity.
//!
//! Beyond *rejecting* unsafe commands, Biscuit supports **graceful degradation**:
//! [`PhysicsSafetyValidator::enforce`] clamps a command back into the safe envelope,
//! and [`PhysicsSafetyValidator::safe_stop`] produces a controlled-stop command for
//! limp-home behavior — the runtime never has to crash to stay safe.
//!
//! # Example
//!
//! ```
//! use lair_biscuit::{PhysicsSafetyValidator, SafetyLimits, SafetyValidator};
//! use lair_msgs::{ControlCommand, Gear};
//!
//! let validator = PhysicsSafetyValidator::new(SafetyLimits::default());
//!
//! // A reasonable command passes.
//! let ok = ControlCommand { throttle: 0.3, brake: 0.0, steering: 0.1, gear: Gear::Drive };
//! assert!(validator.validate(&ok).is_ok());
//!
//! // Braking and accelerating at once is rejected...
//! let conflict = ControlCommand { throttle: 0.5, brake: 0.5, steering: 0.0, gear: Gear::Drive };
//! assert!(validator.validate(&conflict).is_err());
//!
//! // ...but `enforce` degrades it to something safe (brake wins).
//! let safe = validator.enforce(&conflict);
//! assert_eq!(safe.throttle, 0.0);
//! assert_eq!(safe.brake, 0.5);
//! ```

use core::fmt;

use lair_core::prelude::LairResult;
use lair_msgs::{ControlCommand, Gear, VehicleState};

/// The physical envelope a [`PhysicsSafetyValidator`] enforces.
///
/// Defaults model a generic passenger-scale vehicle. Tune these to the platform
/// you are deploying on — the validator is only as correct as these limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafetyLimits {
    /// Maximum mechanical steering angle, magnitude in radians (the wheel lock).
    pub max_steering_angle: f64,
    /// Maximum forward speed the platform is permitted to reach, in m/s.
    pub max_speed: f64,
    /// Distance between front and rear axles, in meters (bicycle model).
    pub wheelbase: f64,
    /// Maximum tolerable lateral (cornering) acceleration before rollover / loss
    /// of traction risk, in m/s². This is what couples steering to speed.
    pub max_lateral_accel: f64,
    /// Speed below which a drive/reverse gear change is considered safe, in m/s.
    /// Shifting between Drive and Reverse above this damages the drivetrain.
    pub max_speed_for_gear_change: f64,
    /// Whether throttle and brake may be commanded simultaneously. Almost always
    /// `false`; exposed for unusual platforms (e.g. independent brake-by-wire test rigs).
    pub allow_simultaneous_throttle_brake: bool,
}

impl Default for SafetyLimits {
    fn default() -> Self {
        Self {
            max_steering_angle: 0.6,          // ~34°, typical front-wheel lock
            max_speed: 30.0,                  // m/s (~108 km/h)
            wheelbase: 2.7,                   // m, typical passenger car
            max_lateral_accel: 4.0,           // m/s² (~0.4 g), conservative
            max_speed_for_gear_change: 0.5,   // m/s, essentially stopped
            allow_simultaneous_throttle_brake: false,
        }
    }
}

impl SafetyLimits {
    /// The largest steering magnitude (radians) that keeps lateral acceleration
    /// within [`max_lateral_accel`](Self::max_lateral_accel) at the given speed.
    ///
    /// Derived from the kinematic bicycle model: a vehicle turning with steering
    /// angle `δ` at speed `v` follows a circle of radius `R = wheelbase / tan(δ)`,
    /// giving lateral acceleration `a = v² / R = v²·tan(δ) / wheelbase`. Solving
    /// `a ≤ max_lateral_accel` for `δ` yields the bound below. At low speeds the
    /// bound exceeds the mechanical lock, so it is clamped to `max_steering_angle`.
    pub fn steering_limit_at_speed(&self, speed: f64) -> f64 {
        let v = speed.abs();
        if v < 1e-3 {
            // Stationary or crawling: only the mechanical lock matters.
            return self.max_steering_angle;
        }
        let max_tan = self.max_lateral_accel * self.wheelbase / (v * v);
        let dynamic = max_tan.atan();
        dynamic.min(self.max_steering_angle)
    }
}

/// A specific reason a control command was rejected.
///
/// Carried inside the [`LairResult`] error message; also useful for telemetry so
/// operators can see *why* Biscuit intervened, not just that it did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetyViolation {
    /// A command field was `NaN` or infinite.
    NonFiniteValue { field: &'static str },
    /// Throttle was outside `0.0..=1.0`.
    ThrottleOutOfRange { value: f64 },
    /// Brake was outside `0.0..=1.0`.
    BrakeOutOfRange { value: f64 },
    /// Steering magnitude exceeded the mechanical lock.
    SteeringExceedsLock { value: f64, limit: f64 },
    /// Throttle and brake were both engaged.
    ThrottleBrakeConflict { throttle: f64, brake: f64 },
    /// Steering would exceed the lateral-acceleration limit at the current speed.
    SteeringUnsafeForSpeed { value: f64, limit: f64, speed: f64 },
    /// Throttle commanded while accelerating past the speed limit.
    OverspeedThrottle { speed: f64, max_speed: f64 },
    /// A drive/reverse gear change was commanded while still moving.
    GearChangeWhileMoving { from: Gear, to: Gear, speed: f64 },
    /// Throttle commanded while the transmission is in Park.
    ThrottleInPark { throttle: f64 },
}

impl fmt::Display for SafetyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue { field } => {
                write!(f, "control command field `{field}` is not finite")
            }
            Self::ThrottleOutOfRange { value } => {
                write!(f, "throttle {value} out of range 0.0..=1.0")
            }
            Self::BrakeOutOfRange { value } => {
                write!(f, "brake {value} out of range 0.0..=1.0")
            }
            Self::SteeringExceedsLock { value, limit } => {
                write!(f, "steering {value} rad exceeds mechanical lock ±{limit} rad")
            }
            Self::ThrottleBrakeConflict { throttle, brake } => {
                write!(f, "throttle ({throttle}) and brake ({brake}) engaged simultaneously")
            }
            Self::SteeringUnsafeForSpeed { value, limit, speed } => write!(
                f,
                "steering {value} rad exceeds safe limit ±{limit} rad at {speed} m/s (rollover risk)"
            ),
            Self::OverspeedThrottle { speed, max_speed } => {
                write!(f, "throttle applied at {speed} m/s, above speed limit {max_speed} m/s")
            }
            Self::GearChangeWhileMoving { from, to, speed } => write!(
                f,
                "gear change {from:?} -> {to:?} requested at {speed} m/s (must be near-stopped)"
            ),
            Self::ThrottleInPark { throttle } => {
                write!(f, "throttle ({throttle}) applied while in Park")
            }
        }
    }
}

impl SafetyViolation {
    fn into_err<T>(self) -> LairResult<T> {
        Err(format!("safety violation: {self}").into())
    }
}

/// Validates a control command against safety constraints.
///
/// Implementations check bounds on throttle, brake, steering, and any
/// domain-specific invariants that hold independent of vehicle state.
pub trait SafetyValidator {
    /// Returns `Ok(())` if the command is safe in isolation, or an error
    /// describing the first violation found.
    fn validate(&self, cmd: &ControlCommand) -> LairResult<()>;
}

/// A state-aware extension of [`SafetyValidator`] for physics-constrained checks.
///
/// These checks depend on the current [`VehicleState`] (speed, gear) and so cannot
/// be expressed by [`SafetyValidator::validate`] alone.
pub trait StatefulSafetyValidator: SafetyValidator {
    /// Validates `cmd` against both the static invariants and the dynamic envelope
    /// implied by `state`. Returns the first violation found.
    fn validate_with_state(&self, cmd: &ControlCommand, state: &VehicleState) -> LairResult<()>;
}

/// A no-op validator that passes all commands.
///
/// **Warning:** for development and testing only. Never deploy this in production —
/// it provides no safety guarantee whatsoever.
pub struct NoOpSafetyValidator;

impl SafetyValidator for NoOpSafetyValidator {
    fn validate(&self, _cmd: &ControlCommand) -> LairResult<()> {
        Ok(())
    }
}

impl StatefulSafetyValidator for NoOpSafetyValidator {
    fn validate_with_state(&self, _cmd: &ControlCommand, _state: &VehicleState) -> LairResult<()> {
        Ok(())
    }
}

/// A physics-constrained safety validator.
///
/// Enforces actuator bounds, mutual-exclusion invariants, and — when given a
/// [`VehicleState`] — the vehicle's dynamic envelope (speed limit, gear-change
/// safety, and a speed-dependent steering limit for rollover prevention).
#[derive(Debug, Clone, Copy)]
pub struct PhysicsSafetyValidator {
    limits: SafetyLimits,
}

impl PhysicsSafetyValidator {
    /// Creates a validator from an explicit set of [`SafetyLimits`].
    pub fn new(limits: SafetyLimits) -> Self {
        Self { limits }
    }

    /// The limits this validator enforces.
    pub fn limits(&self) -> &SafetyLimits {
        &self.limits
    }

    /// Checks the static (state-independent) invariants, returning the first
    /// violation as a structured [`SafetyViolation`].
    fn check_static(&self, cmd: &ControlCommand) -> Result<(), SafetyViolation> {
        if !cmd.throttle.is_finite() {
            return Err(SafetyViolation::NonFiniteValue { field: "throttle" });
        }
        if !cmd.brake.is_finite() {
            return Err(SafetyViolation::NonFiniteValue { field: "brake" });
        }
        if !cmd.steering.is_finite() {
            return Err(SafetyViolation::NonFiniteValue { field: "steering" });
        }
        if !(0.0..=1.0).contains(&cmd.throttle) {
            return Err(SafetyViolation::ThrottleOutOfRange { value: cmd.throttle });
        }
        if !(0.0..=1.0).contains(&cmd.brake) {
            return Err(SafetyViolation::BrakeOutOfRange { value: cmd.brake });
        }
        if cmd.steering.abs() > self.limits.max_steering_angle {
            return Err(SafetyViolation::SteeringExceedsLock {
                value: cmd.steering,
                limit: self.limits.max_steering_angle,
            });
        }
        if !self.limits.allow_simultaneous_throttle_brake
            && cmd.throttle > 0.0
            && cmd.brake > 0.0
        {
            return Err(SafetyViolation::ThrottleBrakeConflict {
                throttle: cmd.throttle,
                brake: cmd.brake,
            });
        }
        Ok(())
    }

    /// Checks the state-dependent (dynamic) invariants.
    fn check_dynamic(
        &self,
        cmd: &ControlCommand,
        state: &VehicleState,
    ) -> Result<(), SafetyViolation> {
        let speed = state.velocity.abs();

        // Speed-dependent steering limit (rollover / traction).
        let steer_limit = self.limits.steering_limit_at_speed(speed);
        if cmd.steering.abs() > steer_limit {
            return Err(SafetyViolation::SteeringUnsafeForSpeed {
                value: cmd.steering,
                limit: steer_limit,
                speed,
            });
        }

        // Do not accelerate at or beyond the speed limit.
        if cmd.throttle > 0.0 && speed >= self.limits.max_speed {
            return Err(SafetyViolation::OverspeedThrottle {
                speed,
                max_speed: self.limits.max_speed,
            });
        }

        // Throttle while parked.
        if cmd.gear == Gear::Park && cmd.throttle > 0.0 {
            return Err(SafetyViolation::ThrottleInPark { throttle: cmd.throttle });
        }

        // Drive/Reverse gear change while still moving.
        if cmd.gear != state.gear
            && is_motion_gear(cmd.gear)
            && is_motion_gear(state.gear)
            && speed > self.limits.max_speed_for_gear_change
        {
            return Err(SafetyViolation::GearChangeWhileMoving {
                from: state.gear,
                to: cmd.gear,
                speed,
            });
        }

        Ok(())
    }

    /// Returns the first [`SafetyViolation`] in `cmd`, if any, checking both static
    /// and dynamic constraints. Returns `None` when the command is fully safe.
    pub fn first_violation(
        &self,
        cmd: &ControlCommand,
        state: &VehicleState,
    ) -> Option<SafetyViolation> {
        self.check_static(cmd)
            .err()
            .or_else(|| self.check_dynamic(cmd, state).err())
    }

    /// Clamps a command back into the safe static envelope (graceful degradation).
    ///
    /// This is the limp-home path: rather than rejecting an out-of-bounds command
    /// outright, produce the closest safe command so the vehicle keeps behaving
    /// predictably. Throttle/brake are clamped to `0.0..=1.0`, steering to the
    /// mechanical lock, throttle/brake conflicts are resolved in favor of braking,
    /// and any non-finite field collapses to a [`safe_stop`](Self::safe_stop).
    pub fn enforce(&self, cmd: &ControlCommand) -> ControlCommand {
        if !cmd.throttle.is_finite() || !cmd.brake.is_finite() || !cmd.steering.is_finite() {
            return self.safe_stop(cmd.gear);
        }

        let mut throttle = cmd.throttle.clamp(0.0, 1.0);
        let brake = cmd.brake.clamp(0.0, 1.0);
        let steering = cmd
            .steering
            .clamp(-self.limits.max_steering_angle, self.limits.max_steering_angle);

        // Brake wins any conflict: it is the fail-safe actuator.
        if !self.limits.allow_simultaneous_throttle_brake && throttle > 0.0 && brake > 0.0 {
            throttle = 0.0;
        }

        // No throttle in Park.
        if cmd.gear == Gear::Park {
            throttle = 0.0;
        }

        ControlCommand { throttle, brake, steering, gear: cmd.gear }
    }

    /// A controlled-stop command: no throttle, full brake, wheels centered,
    /// preserving the supplied gear. Used for limp-home / fail-operational states.
    pub fn safe_stop(&self, gear: Gear) -> ControlCommand {
        ControlCommand { throttle: 0.0, brake: 1.0, steering: 0.0, gear }
    }
}

impl Default for PhysicsSafetyValidator {
    fn default() -> Self {
        Self::new(SafetyLimits::default())
    }
}

impl SafetyValidator for PhysicsSafetyValidator {
    fn validate(&self, cmd: &ControlCommand) -> LairResult<()> {
        match self.check_static(cmd) {
            Ok(()) => Ok(()),
            Err(v) => v.into_err(),
        }
    }
}

impl StatefulSafetyValidator for PhysicsSafetyValidator {
    fn validate_with_state(&self, cmd: &ControlCommand, state: &VehicleState) -> LairResult<()> {
        match self.first_violation(cmd, state) {
            None => Ok(()),
            Some(v) => v.into_err(),
        }
    }
}

/// Whether a gear puts power to the wheels (Drive or Reverse).
fn is_motion_gear(gear: Gear) -> bool {
    matches!(gear, Gear::Drive | Gear::Reverse)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(throttle: f64, brake: f64, steering: f64, gear: Gear) -> ControlCommand {
        ControlCommand { throttle, brake, steering, gear }
    }

    fn state(velocity: f64, gear: Gear) -> VehicleState {
        VehicleState { velocity, gear, ..Default::default() }
    }

    // ── NoOp validator ──────────────────────────────────────────────

    #[test]
    fn noop_validator_passes_everything() {
        let v = NoOpSafetyValidator;
        assert!(v.validate(&cmd(1.0, 1.0, 99.0, Gear::Drive)).is_ok());
        assert!(v
            .validate_with_state(&cmd(1.0, 1.0, 99.0, Gear::Drive), &state(50.0, Gear::Reverse))
            .is_ok());
    }

    // ── Static bounds ───────────────────────────────────────────────

    #[test]
    fn accepts_reasonable_command() {
        let v = PhysicsSafetyValidator::default();
        assert!(v.validate(&cmd(0.3, 0.0, 0.1, Gear::Drive)).is_ok());
        assert!(v.validate(&cmd(0.0, 0.5, -0.2, Gear::Drive)).is_ok());
    }

    #[test]
    fn rejects_throttle_out_of_range() {
        let v = PhysicsSafetyValidator::default();
        assert!(v.validate(&cmd(1.5, 0.0, 0.0, Gear::Drive)).is_err());
        assert!(v.validate(&cmd(-0.1, 0.0, 0.0, Gear::Drive)).is_err());
    }

    #[test]
    fn rejects_brake_out_of_range() {
        let v = PhysicsSafetyValidator::default();
        assert!(v.validate(&cmd(0.0, 2.0, 0.0, Gear::Drive)).is_err());
    }

    #[test]
    fn rejects_steering_beyond_lock() {
        let v = PhysicsSafetyValidator::default();
        assert!(v.validate(&cmd(0.0, 0.0, 1.5, Gear::Drive)).is_err());
        assert!(v.validate(&cmd(0.0, 0.0, -1.5, Gear::Drive)).is_err());
    }

    #[test]
    fn rejects_non_finite() {
        let v = PhysicsSafetyValidator::default();
        assert!(v.validate(&cmd(f64::NAN, 0.0, 0.0, Gear::Drive)).is_err());
        assert!(v.validate(&cmd(0.0, f64::INFINITY, 0.0, Gear::Drive)).is_err());
        assert!(v.validate(&cmd(0.0, 0.0, f64::NAN, Gear::Drive)).is_err());
    }

    #[test]
    fn rejects_throttle_brake_conflict() {
        let v = PhysicsSafetyValidator::default();
        assert!(v.validate(&cmd(0.5, 0.5, 0.0, Gear::Drive)).is_err());
    }

    #[test]
    fn allows_conflict_when_configured() {
        let limits = SafetyLimits { allow_simultaneous_throttle_brake: true, ..Default::default() };
        let v = PhysicsSafetyValidator::new(limits);
        assert!(v.validate(&cmd(0.5, 0.5, 0.0, Gear::Drive)).is_ok());
    }

    // ── Dynamic / physics-constrained ───────────────────────────────

    #[test]
    fn rejects_hard_steering_at_high_speed() {
        let v = PhysicsSafetyValidator::default();
        // Near-lock steering is fine when stationary...
        assert!(v
            .validate_with_state(&cmd(0.0, 0.0, 0.5, Gear::Drive), &state(0.0, Gear::Drive))
            .is_ok());
        // ...but dangerous at 25 m/s.
        assert!(v
            .validate_with_state(&cmd(0.0, 0.0, 0.5, Gear::Drive), &state(25.0, Gear::Drive))
            .is_err());
    }

    #[test]
    fn steering_limit_shrinks_with_speed() {
        let limits = SafetyLimits::default();
        let slow = limits.steering_limit_at_speed(2.0);
        let fast = limits.steering_limit_at_speed(25.0);
        assert!(fast < slow, "limit should tighten as speed rises ({fast} !< {slow})");
        // At a crawl, the mechanical lock dominates.
        assert_eq!(limits.steering_limit_at_speed(0.0), limits.max_steering_angle);
    }

    #[test]
    fn steering_limit_respects_lateral_accel() {
        // At the computed limit, lateral accel should be ~max_lateral_accel.
        let limits = SafetyLimits::default();
        let speed = 20.0;
        let delta = limits.steering_limit_at_speed(speed);
        let lat_accel = speed * speed * delta.tan() / limits.wheelbase;
        assert!((lat_accel - limits.max_lateral_accel).abs() < 1e-6);
    }

    #[test]
    fn rejects_throttle_over_speed_limit() {
        let v = PhysicsSafetyValidator::default();
        let s = state(30.0, Gear::Drive); // at the limit
        assert!(v.validate_with_state(&cmd(0.1, 0.0, 0.0, Gear::Drive), &s).is_err());
        // Coasting (no throttle) at the limit is allowed.
        assert!(v.validate_with_state(&cmd(0.0, 0.2, 0.0, Gear::Drive), &s).is_ok());
    }

    #[test]
    fn rejects_throttle_in_park() {
        let v = PhysicsSafetyValidator::default();
        let s = state(0.0, Gear::Park);
        assert!(v.validate_with_state(&cmd(0.2, 0.0, 0.0, Gear::Park), &s).is_err());
    }

    #[test]
    fn rejects_gear_change_while_moving() {
        let v = PhysicsSafetyValidator::default();
        let moving = state(10.0, Gear::Drive);
        // Drive -> Reverse at 10 m/s: rejected.
        assert!(v.validate_with_state(&cmd(0.0, 0.3, 0.0, Gear::Reverse), &moving).is_err());
        // Same change while essentially stopped: allowed.
        let stopped = state(0.1, Gear::Drive);
        assert!(v.validate_with_state(&cmd(0.0, 0.5, 0.0, Gear::Reverse), &stopped).is_ok());
    }

    // ── Graceful degradation ────────────────────────────────────────

    #[test]
    fn enforce_clamps_into_range() {
        let v = PhysicsSafetyValidator::default();
        let safe = v.enforce(&cmd(2.0, 0.0, 5.0, Gear::Drive));
        assert_eq!(safe.throttle, 1.0);
        assert_eq!(safe.steering, v.limits().max_steering_angle);
        assert!(v.validate(&safe).is_ok());
    }

    #[test]
    fn enforce_resolves_conflict_in_favor_of_brake() {
        let v = PhysicsSafetyValidator::default();
        let safe = v.enforce(&cmd(0.8, 0.4, 0.0, Gear::Drive));
        assert_eq!(safe.throttle, 0.0);
        assert_eq!(safe.brake, 0.4);
        assert!(v.validate(&safe).is_ok());
    }

    #[test]
    fn enforce_non_finite_yields_safe_stop() {
        let v = PhysicsSafetyValidator::default();
        let safe = v.enforce(&cmd(f64::NAN, 0.0, 0.0, Gear::Drive));
        assert_eq!(safe.throttle, 0.0);
        assert_eq!(safe.brake, 1.0);
        assert_eq!(safe.steering, 0.0);
        assert!(v.validate(&safe).is_ok());
    }

    #[test]
    fn enforce_no_throttle_in_park() {
        let v = PhysicsSafetyValidator::default();
        let safe = v.enforce(&cmd(0.5, 0.0, 0.0, Gear::Park));
        assert_eq!(safe.throttle, 0.0);
    }

    #[test]
    fn safe_stop_is_valid_and_full_brake() {
        let v = PhysicsSafetyValidator::default();
        let s = v.safe_stop(Gear::Drive);
        assert_eq!(s.brake, 1.0);
        assert_eq!(s.throttle, 0.0);
        assert!(v.validate(&s).is_ok());
    }

    #[test]
    fn first_violation_reports_structured_reason() {
        let v = PhysicsSafetyValidator::default();
        let viol = v.first_violation(&cmd(0.5, 0.5, 0.0, Gear::Drive), &state(0.0, Gear::Drive));
        assert!(matches!(viol, Some(SafetyViolation::ThrottleBrakeConflict { .. })));
    }
}
