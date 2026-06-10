//! The safety guard as a first-class runtime task.
//!
//! [`SafetyGuardTask`] turns the safety checkpoint into a node you can place in a
//! LAIR task graph. Wire it on the edge between whatever produces control commands
//! and the actuator that consumes them:
//!
//! ```text
//! planner ──(ControlCommand)──> safety_guard ──(ControlCommand)──> actuator
//! ```
//!
//! Because it sits *on the wire*, the actuator can only ever receive commands that
//! have passed through the validator — the checkpoint cannot be skipped by the
//! downstream task. It applies the state-independent envelope via
//! [`PhysicsSafetyValidator::enforce`] (bounds, finiteness, throttle/brake
//! exclusion, no-throttle-in-Park), so it needs no inputs beyond the command and
//! can be dropped onto any `ControlCommand` connection.
//!
//! In a `.ron` graph:
//!
//! ```ron
//! tasks: [
//!     ( id: "planner",      type: "crate::Planner" ),
//!     ( id: "safety_guard", type: "lair_biscuit::SafetyGuardTask" ),
//!     ( id: "actuator",     type: "crate::Actuator" ),
//! ],
//! cnx: [
//!     ( src: "planner",      dst: "safety_guard", msg: "lair_msgs::ControlCommand" ),
//!     ( src: "safety_guard", dst: "actuator",     msg: "lair_msgs::ControlCommand" ),
//! ],
//! ```

use cu29_runtime::cutask::CuMsg;
use cu29_runtime::{input_msg, output_msg};
use lair_core::prelude::{Freezable, LairConfig, LairContext, LairResult, LairTask};
use lair_msgs::ControlCommand;

use crate::PhysicsSafetyValidator;

/// A [`LairTask`] that enforces the static safety envelope on every
/// [`ControlCommand`] passing through it.
#[derive(Debug, Default, Clone, Copy)]
pub struct SafetyGuardTask {
    validator: PhysicsSafetyValidator,
}

impl SafetyGuardTask {
    /// Creates a guard task with an explicit validator.
    pub fn with_validator(validator: PhysicsSafetyValidator) -> Self {
        Self { validator }
    }

    /// The validator this task applies.
    pub fn validator(&self) -> &PhysicsSafetyValidator {
        &self.validator
    }
}

impl Freezable for SafetyGuardTask {}

impl LairTask for SafetyGuardTask {
    type Resources<'r> = ();
    type Input<'m> = input_msg!(ControlCommand);
    type Output<'m> = output_msg!(ControlCommand);

    fn new(_config: Option<&LairConfig>, _resources: Self::Resources<'_>) -> LairResult<Self>
    where
        Self: Sized,
    {
        Ok(Self::default())
    }

    fn process(
        &mut self,
        _ctx: &LairContext,
        input: &Self::Input<'_>,
        output: &mut Self::Output<'_>,
    ) -> LairResult<()> {
        match input.payload() {
            // Every command that leaves this task is inside the safe envelope.
            Some(cmd) => output.set_payload(self.validator.enforce(cmd)),
            // No command in — pass nothing on rather than fabricate one.
            None => output.clear_payload(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lair_core::prelude::LairContext;
    use lair_msgs::Gear;

    fn cmd(throttle: f64, brake: f64, steering: f64, gear: Gear) -> ControlCommand {
        ControlCommand { throttle, brake, steering, gear }
    }

    #[test]
    fn task_clamps_unsafe_command_on_the_wire() {
        let (ctx, _clock) = LairContext::new_mock_clock();
        let mut task = SafetyGuardTask::new(None, ()).unwrap();

        // An out-of-bounds, conflicting command arrives from upstream.
        let input: CuMsg<ControlCommand> = CuMsg::new(Some(cmd(2.0, 0.5, 5.0, Gear::Drive)));
        let mut output: CuMsg<ControlCommand> = CuMsg::new(None);

        task.process(&ctx, &input, &mut output).unwrap();

        let out = output.payload().expect("guard should emit a command");
        assert_eq!(out.throttle, 0.0); // conflict resolved toward brake
        assert_eq!(out.brake, 0.5);
        assert_eq!(out.steering, task.validator().limits().max_steering_angle);
    }

    #[test]
    fn task_passes_safe_command_unchanged() {
        let (ctx, _clock) = LairContext::new_mock_clock();
        let mut task = SafetyGuardTask::default();

        let safe = cmd(0.3, 0.0, 0.1, Gear::Drive);
        let input: CuMsg<ControlCommand> = CuMsg::new(Some(safe));
        let mut output: CuMsg<ControlCommand> = CuMsg::new(None);

        task.process(&ctx, &input, &mut output).unwrap();
        assert_eq!(output.payload(), Some(&safe));
    }

    #[test]
    fn task_emits_nothing_for_empty_input() {
        let (ctx, _clock) = LairContext::new_mock_clock();
        let mut task = SafetyGuardTask::default();

        let input: CuMsg<ControlCommand> = CuMsg::new(None);
        let mut output: CuMsg<ControlCommand> = CuMsg::new(Some(cmd(0.5, 0.0, 0.0, Gear::Drive)));

        task.process(&ctx, &input, &mut output).unwrap();
        assert_eq!(output.payload(), None);
    }
}
