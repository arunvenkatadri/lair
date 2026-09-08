//! Sencha Biscuit — safety policy engine.
//!
//! Provides traits and validators for enforcing safety constraints
//! on vehicle control commands before they reach actuators.

use sencha_core::prelude::SenchaResult;
use sencha_msgs::ControlCommand;

/// Validates a control command against safety constraints.
///
/// Implementations should check bounds on throttle, brake, steering,
/// and any domain-specific invariants.
pub trait SafetyValidator {
    /// Returns `Ok(())` if the command is safe, or an error describing the violation.
    fn validate(&self, cmd: &ControlCommand) -> SenchaResult<()>;
}

/// A no-op validator that passes all commands.
///
/// **Warning:** This is for development and testing only. Never use in production
/// without replacing it with a real safety validator.
pub struct NoOpSafetyValidator;

impl SafetyValidator for NoOpSafetyValidator {
    fn validate(&self, _cmd: &ControlCommand) -> SenchaResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sencha_msgs::Gear;

    #[test]
    fn noop_validator_passes_everything() {
        let validator = NoOpSafetyValidator;
        let cmd = ControlCommand {
            throttle: 1.0,
            brake: 0.0,
            steering: 0.5,
            gear: Gear::Drive,
        };
        assert!(validator.validate(&cmd).is_ok());
    }

    #[test]
    fn noop_validator_passes_default() {
        let validator = NoOpSafetyValidator;
        let cmd = ControlCommand::default();
        assert!(validator.validate(&cmd).is_ok());
    }
}
