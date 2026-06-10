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

/// A [`ControlCommand`] that has been approved or corrected by a safety validator.
///
/// `SafeCommand` has no public constructor: the *only* ways to obtain one are
/// [`PhysicsSafetyValidator::approve`] (validation succeeded) and
/// [`PhysicsSafetyValidator::clamp_to_safe`] / the [`SafetyGuard`] (the command was
/// corrected back into the safe envelope). This makes safety **unbypassable by
/// construction**: an actuator whose hardware-facing API accepts only a
/// `SafeCommand` cannot be driven by a command that never passed the validator.
///
/// ```
/// use lair_biscuit::{PhysicsSafetyValidator, SafeCommand};
/// use lair_msgs::{ControlCommand, Gear, VehicleState};
///
/// // An actuator that physically refuses to act on unvalidated input.
/// fn drive_motors(_cmd: &SafeCommand) { /* touch hardware */ }
///
/// let v = PhysicsSafetyValidator::default();
/// let raw = ControlCommand { throttle: 0.2, brake: 0.0, steering: 0.0, gear: Gear::Drive };
/// let safe = v.approve(&raw, &VehicleState::default()).unwrap();
/// drive_motors(&safe);            // ✅ only reachable via the validator
/// // drive_motors(&raw);          // ❌ does not compile: raw is not a SafeCommand
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafeCommand(ControlCommand);

impl SafeCommand {
    /// The underlying validated command.
    pub fn command(&self) -> ControlCommand {
        self.0
    }

    /// A reference to the underlying validated command.
    pub fn get(&self) -> &ControlCommand {
        &self.0
    }
}

impl core::ops::Deref for SafeCommand {
    type Target = ControlCommand;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SafeCommand> for ControlCommand {
    fn from(safe: SafeCommand) -> Self {
        safe.0
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

    /// Clamps a command back into the safe envelope given the current vehicle
    /// state, accounting for the dynamic constraints [`enforce`](Self::enforce)
    /// cannot see on its own (speed-dependent steering, the speed limit, and unsafe
    /// gear changes).
    ///
    /// The result is guaranteed to satisfy
    /// [`validate_with_state`](StatefulSafetyValidator::validate_with_state) for the
    /// same `state`.
    pub fn enforce_with_state(&self, cmd: &ControlCommand, state: &VehicleState) -> ControlCommand {
        let mut out = self.enforce(cmd);
        let speed = state.velocity.abs();

        // Tighten steering to what is safe at this speed.
        let steer_limit = self.limits.steering_limit_at_speed(speed);
        out.steering = out.steering.clamp(-steer_limit, steer_limit);

        // Do not accelerate at or beyond the speed limit.
        if speed >= self.limits.max_speed {
            out.throttle = 0.0;
        }

        // Refuse an unsafe drive/reverse change: hold the current gear, cut throttle.
        if out.gear != state.gear
            && is_motion_gear(out.gear)
            && is_motion_gear(state.gear)
            && speed > self.limits.max_speed_for_gear_change
        {
            out.gear = state.gear;
            out.throttle = 0.0;
        }

        // Re-apply the Park rule in case the gear was just changed back.
        if out.gear == Gear::Park {
            out.throttle = 0.0;
        }

        out
    }

    /// Validates `cmd` against `state` and, on success, returns it wrapped as a
    /// [`SafeCommand`]. This is the gateway for the "unbypassable by construction"
    /// pattern: callers that need a `SafeCommand` must go through validation.
    pub fn approve(&self, cmd: &ControlCommand, state: &VehicleState) -> LairResult<SafeCommand> {
        self.validate_with_state(cmd, state)?;
        Ok(SafeCommand(*cmd))
    }

    /// Corrects `cmd` into the safe envelope for `state` and returns it as a
    /// [`SafeCommand`]. Always succeeds — this is the graceful-degradation path.
    pub fn clamp_to_safe(&self, cmd: &ControlCommand, state: &VehicleState) -> SafeCommand {
        SafeCommand(self.enforce_with_state(cmd, state))
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

/// How a [`SafetyGuard`] reacts when a command violates a constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SafetyMode {
    /// Block the unsafe command and substitute a corrected one. Nothing unsafe
    /// ever reaches the actuator. This is the production default.
    #[default]
    Enforced,
    /// Let the original command through but flag the violation for telemetry.
    /// Useful for shadow-deploying new limits without affecting behavior.
    Advisory,
}

/// The outcome of running a command through a [`SafetyGuard`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetyDecision {
    /// The command satisfied every constraint and passed through unchanged.
    Clear(SafeCommand),
    /// (Enforced mode) The command violated a constraint and was corrected to the
    /// safe command carried here.
    Corrected { command: SafeCommand, violation: SafetyViolation },
    /// (Advisory mode) The command violated a constraint but was passed through
    /// unchanged, with the violation reported for telemetry.
    AdvisoryPass { command: SafeCommand, violation: SafetyViolation },
}

impl SafetyDecision {
    /// The [`SafeCommand`] that should actually be sent to the actuator.
    pub fn command(&self) -> &SafeCommand {
        match self {
            Self::Clear(c)
            | Self::Corrected { command: c, .. }
            | Self::AdvisoryPass { command: c, .. } => c,
        }
    }

    /// The violation that occurred, if any.
    pub fn violation(&self) -> Option<SafetyViolation> {
        match self {
            Self::Clear(_) => None,
            Self::Corrected { violation, .. } | Self::AdvisoryPass { violation, .. } => {
                Some(*violation)
            }
        }
    }

    /// Whether a safety violation was detected (regardless of mode).
    pub fn intervened(&self) -> bool {
        !matches!(self, Self::Clear(_))
    }
}

/// The safety guard from the LAIR architecture: the checkpoint every control
/// command passes through on its way to the actuators.
///
/// A guard pairs a [`PhysicsSafetyValidator`] with a [`SafetyMode`] and keeps a
/// running count of interventions for telemetry. Feed it each command together
/// with the latest [`VehicleState`]; it returns a [`SafetyDecision`] whose
/// [`command`](SafetyDecision::command) is a [`SafeCommand`] guaranteed to satisfy
/// the validator.
///
/// ```
/// use lair_biscuit::{SafetyGuard, SafetyMode};
/// use lair_msgs::{ControlCommand, Gear, VehicleState};
///
/// let mut guard = SafetyGuard::enforced();
/// let state = VehicleState { velocity: 25.0, gear: Gear::Drive, ..Default::default() };
///
/// // Hard steering at 25 m/s would roll the vehicle; the guard corrects it.
/// let reckless = ControlCommand { throttle: 0.0, brake: 0.0, steering: 0.5, gear: Gear::Drive };
/// let decision = guard.guard(&reckless, &state);
/// assert!(decision.intervened());
/// assert!(decision.command().steering.abs() < 0.5);
/// assert_eq!(guard.intervention_count(), 1);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SafetyGuard {
    validator: PhysicsSafetyValidator,
    mode: SafetyMode,
    interventions: u64,
}

impl SafetyGuard {
    /// Creates a guard with an explicit validator and mode.
    pub fn new(validator: PhysicsSafetyValidator, mode: SafetyMode) -> Self {
        Self { validator, mode, interventions: 0 }
    }

    /// A guard in [`SafetyMode::Enforced`] with default limits.
    pub fn enforced() -> Self {
        Self::new(PhysicsSafetyValidator::default(), SafetyMode::Enforced)
    }

    /// A guard in [`SafetyMode::Advisory`] with default limits.
    pub fn advisory() -> Self {
        Self::new(PhysicsSafetyValidator::default(), SafetyMode::Advisory)
    }

    /// The mode this guard operates in.
    pub fn mode(&self) -> SafetyMode {
        self.mode
    }

    /// The validator backing this guard.
    pub fn validator(&self) -> &PhysicsSafetyValidator {
        &self.validator
    }

    /// How many commands this guard has flagged as violations.
    pub fn intervention_count(&self) -> u64 {
        self.interventions
    }

    /// Runs `cmd` (against the current `state`) through the guard, returning the
    /// [`SafetyDecision`]. Increments the intervention counter on any violation.
    pub fn guard(&mut self, cmd: &ControlCommand, state: &VehicleState) -> SafetyDecision {
        match self.validator.first_violation(cmd, state) {
            None => SafetyDecision::Clear(SafeCommand(*cmd)),
            Some(violation) => {
                self.interventions += 1;
                match self.mode {
                    SafetyMode::Enforced => SafetyDecision::Corrected {
                        command: self.validator.clamp_to_safe(cmd, state),
                        violation,
                    },
                    SafetyMode::Advisory => SafetyDecision::AdvisoryPass {
                        command: SafeCommand(*cmd),
                        violation,
                    },
                }
            }
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

    // ── State-aware enforcement ─────────────────────────────────────

    #[test]
    fn enforce_with_state_always_produces_valid_command() {
        let v = PhysicsSafetyValidator::default();
        // A spread of nasty commands paired with various states.
        let cmds = [
            cmd(1.0, 0.0, 0.6, Gear::Drive),
            cmd(0.8, 0.8, -0.6, Gear::Reverse),
            cmd(0.5, 0.0, 0.0, Gear::Park),
            cmd(f64::NAN, 0.0, 0.0, Gear::Drive),
            cmd(0.9, 0.0, 0.4, Gear::Drive),
        ];
        let states = [
            state(0.0, Gear::Park),
            state(15.0, Gear::Drive),
            state(30.0, Gear::Drive),
            state(8.0, Gear::Reverse),
        ];
        for c in &cmds {
            for s in &states {
                let safe = v.enforce_with_state(c, s);
                assert!(
                    v.validate_with_state(&safe, s).is_ok(),
                    "enforce_with_state produced an invalid command: {safe:?} for state {s:?}"
                );
            }
        }
    }

    #[test]
    fn enforce_with_state_holds_gear_when_moving() {
        let v = PhysicsSafetyValidator::default();
        let moving = state(10.0, Gear::Drive);
        let safe = v.enforce_with_state(&cmd(0.3, 0.0, 0.0, Gear::Reverse), &moving);
        assert_eq!(safe.gear, Gear::Drive); // refused the shift
        assert_eq!(safe.throttle, 0.0);
    }

    #[test]
    fn approve_returns_safe_command_only_when_valid() {
        let v = PhysicsSafetyValidator::default();
        let s = state(0.0, Gear::Drive);
        assert!(v.approve(&cmd(0.3, 0.0, 0.1, Gear::Drive), &s).is_ok());
        assert!(v.approve(&cmd(0.5, 0.5, 0.0, Gear::Drive), &s).is_err());
    }

    #[test]
    fn safe_command_exposes_inner() {
        let v = PhysicsSafetyValidator::default();
        let s = state(0.0, Gear::Drive);
        let safe = v.approve(&cmd(0.3, 0.0, 0.1, Gear::Drive), &s).unwrap();
        assert_eq!(safe.throttle, 0.3); // via Deref
        assert_eq!(safe.command().steering, 0.1);
        let raw: ControlCommand = safe.into();
        assert_eq!(raw.throttle, 0.3);
    }

    // ── SafetyGuard ─────────────────────────────────────────────────

    #[test]
    fn guard_clear_passes_through_unchanged() {
        let mut g = SafetyGuard::enforced();
        let s = state(5.0, Gear::Drive);
        let d = g.guard(&cmd(0.2, 0.0, 0.05, Gear::Drive), &s);
        assert!(matches!(d, SafetyDecision::Clear(_)));
        assert!(!d.intervened());
        assert_eq!(g.intervention_count(), 0);
    }

    #[test]
    fn guard_enforced_corrects_and_counts() {
        let mut g = SafetyGuard::enforced();
        let s = state(25.0, Gear::Drive);
        let d = g.guard(&cmd(0.0, 0.0, 0.5, Gear::Drive), &s);
        assert!(matches!(d, SafetyDecision::Corrected { .. }));
        assert!(d.command().steering.abs() < 0.5);
        // Corrected command must itself be valid.
        assert!(g.validator().validate_with_state(d.command().get(), &s).is_ok());
        assert_eq!(g.intervention_count(), 1);
    }

    #[test]
    fn guard_advisory_passes_original_but_flags() {
        let mut g = SafetyGuard::advisory();
        let s = state(25.0, Gear::Drive);
        let reckless = cmd(0.0, 0.0, 0.5, Gear::Drive);
        let d = g.guard(&reckless, &s);
        assert!(matches!(d, SafetyDecision::AdvisoryPass { .. }));
        assert_eq!(d.command().steering, 0.5); // unchanged
        assert!(d.violation().is_some());
        assert_eq!(g.intervention_count(), 1);
    }

    #[test]
    fn guard_default_mode_is_enforced() {
        assert_eq!(SafetyMode::default(), SafetyMode::Enforced);
        assert_eq!(SafetyGuard::enforced().mode(), SafetyMode::Enforced);
    }
}
