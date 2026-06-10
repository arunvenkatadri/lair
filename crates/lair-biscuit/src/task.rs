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
//! ## Watchdog
//!
//! The guard doubles as a **fail-operational watchdog**. Configure a
//! `watchdog_timeout_ms` and, if the upstream planner goes silent for longer than
//! that, the guard stops clearing its output and instead emits a controlled
//! [`safe_stop`](PhysicsSafetyValidator::safe_stop) — so a wedged or crashed
//! planner brings the vehicle to a stop rather than leaving the last command
//! latched. The deadline is measured against the runtime clock, so it is
//! deterministic and mockable.
//!
//! In a `.ron` graph:
//!
//! ```ron
//! tasks: [
//!     ( id: "planner",      type: "crate::Planner" ),
//!     ( id: "safety_guard", type: "lair_biscuit::SafetyGuardTask",
//!       config: { watchdog_timeout_ms: 100 } ),
//!     ( id: "actuator",     type: "crate::Actuator" ),
//! ],
//! cnx: [
//!     ( src: "planner",      dst: "safety_guard", msg: "lair_msgs::ControlCommand" ),
//!     ( src: "safety_guard", dst: "actuator",     msg: "lair_msgs::ControlCommand" ),
//! ],
//! ```

use cu29_runtime::cutask::CuMsg;
use cu29_runtime::{input_msg, output_msg};
use lair_core::prelude::{CuDuration, CuTime, Freezable, LairConfig, LairContext, LairResult, LairTask};
use lair_msgs::{ControlCommand, Gear};

use crate::PhysicsSafetyValidator;

/// A [`LairTask`] that enforces the static safety envelope on every
/// [`ControlCommand`] passing through it, with an optional staleness watchdog.
#[derive(Debug, Default, Clone, Copy)]
pub struct SafetyGuardTask {
    validator: PhysicsSafetyValidator,
    /// Maximum time without a fresh command before failing to a safe stop.
    /// `None` disables the watchdog (the guard simply passes nothing through when
    /// it has no input).
    watchdog: Option<CuDuration>,
    /// Time the last valid command was forwarded.
    last_command: Option<CuTime>,
    /// Gear of the last valid command — used so a watchdog `safe_stop` keeps the
    /// vehicle in a sensible gear.
    last_gear: Gear,
}

impl SafetyGuardTask {
    /// Creates a guard task with an explicit validator (watchdog disabled).
    pub fn with_validator(validator: PhysicsSafetyValidator) -> Self {
        Self { validator, ..Self::default() }
    }

    /// Enables the staleness watchdog with the given timeout.
    pub fn with_watchdog(mut self, timeout: CuDuration) -> Self {
        self.watchdog = Some(timeout);
        self
    }

    /// The validator this task applies.
    pub fn validator(&self) -> &PhysicsSafetyValidator {
        &self.validator
    }

    /// The configured watchdog timeout, if any.
    pub fn watchdog(&self) -> Option<CuDuration> {
        self.watchdog
    }

    /// Whether the watchdog has expired as of `now` (a command was seen, and more
    /// than the timeout has elapsed since).
    fn watchdog_expired(&self, now: CuTime) -> bool {
        match (self.watchdog, self.last_command) {
            (Some(timeout), Some(last)) => {
                now.as_nanos().saturating_sub(last.as_nanos()) > timeout.as_nanos()
            }
            _ => false,
        }
    }
}

impl Freezable for SafetyGuardTask {}

impl LairTask for SafetyGuardTask {
    type Resources<'r> = ();
    type Input<'m> = input_msg!(ControlCommand);
    type Output<'m> = output_msg!(ControlCommand);

    fn new(config: Option<&LairConfig>, _resources: Self::Resources<'_>) -> LairResult<Self>
    where
        Self: Sized,
    {
        let watchdog = config
            .and_then(|c| c.get::<u64>("watchdog_timeout_ms").ok().flatten())
            .map(CuDuration::from_millis);
        Ok(Self { watchdog, ..Self::default() })
    }

    fn process(
        &mut self,
        ctx: &LairContext,
        input: &Self::Input<'_>,
        output: &mut Self::Output<'_>,
    ) -> LairResult<()> {
        let now = ctx.clock.now();
        match input.payload() {
            // Every command that leaves this task is inside the safe envelope.
            Some(cmd) => {
                let safe = self.validator.enforce(cmd);
                self.last_command = Some(now);
                self.last_gear = safe.gear;
                output.set_payload(safe);
            }
            // No command in. If the upstream has been silent past the watchdog
            // deadline, fail to a controlled stop; otherwise pass nothing on.
            None => {
                if self.watchdog_expired(now) {
                    output.set_payload(self.validator.safe_stop(self.last_gear));
                } else {
                    output.clear_payload();
                }
            }
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
    fn task_emits_nothing_for_empty_input_without_watchdog() {
        let (ctx, _clock) = LairContext::new_mock_clock();
        let mut task = SafetyGuardTask::default();

        let input: CuMsg<ControlCommand> = CuMsg::new(None);
        let mut output: CuMsg<ControlCommand> = CuMsg::new(Some(cmd(0.5, 0.0, 0.0, Gear::Drive)));

        task.process(&ctx, &input, &mut output).unwrap();
        assert_eq!(output.payload(), None);
    }

    #[test]
    fn watchdog_fails_to_safe_stop_when_upstream_goes_silent() {
        let (ctx, clock) = LairContext::new_mock_clock();
        let mut task = SafetyGuardTask::default().with_watchdog(CuDuration::from_millis(100));

        // A valid command arrives at t=0, in Drive.
        let good: CuMsg<ControlCommand> = CuMsg::new(Some(cmd(0.3, 0.0, 0.1, Gear::Drive)));
        let mut out: CuMsg<ControlCommand> = CuMsg::new(None);
        task.process(&ctx, &good, &mut out).unwrap();
        assert_eq!(out.payload().unwrap().throttle, 0.3);

        // Upstream goes silent but we're still within the deadline: nothing emitted.
        clock.increment(CuDuration::from_millis(80));
        let empty: CuMsg<ControlCommand> = CuMsg::new(None);
        let mut out: CuMsg<ControlCommand> = CuMsg::new(None);
        task.process(&ctx, &empty, &mut out).unwrap();
        assert_eq!(out.payload(), None);

        // Past the deadline: fail to a controlled stop, keeping the last gear.
        clock.increment(CuDuration::from_millis(40)); // total 120ms > 100ms
        let empty: CuMsg<ControlCommand> = CuMsg::new(None);
        let mut out: CuMsg<ControlCommand> = CuMsg::new(None);
        task.process(&ctx, &empty, &mut out).unwrap();
        let stop = out.payload().expect("watchdog should emit a safe stop");
        assert_eq!(stop.throttle, 0.0);
        assert_eq!(stop.brake, 1.0);
        assert_eq!(stop.steering, 0.0);
        assert_eq!(stop.gear, Gear::Drive);
    }

    #[test]
    fn watchdog_reads_timeout_from_config() {
        use cu29_runtime::config::{read_configuration_str, ConfigGraphs};

        let ron = r#"(
            tasks: [
                ( id: "safety_guard", type: "lair_biscuit::SafetyGuardTask",
                  config: { "watchdog_timeout_ms": 250 } ),
                ( id: "actuator", type: "crate::Actuator" ),
            ],
            cnx: [
                ( src: "safety_guard", dst: "actuator", msg: "lair_msgs::ControlCommand" ),
            ],
        )"#;
        let config = read_configuration_str(ron.to_string(), None).unwrap();
        let graph = match &config.graphs {
            ConfigGraphs::Simple(g) => g,
            _ => panic!("expected a simple graph"),
        };
        let node = graph
            .get_all_nodes()
            .into_iter()
            .find(|(_, n)| n.get_id() == "safety_guard")
            .map(|(_, n)| n)
            .expect("guard node");

        let task = SafetyGuardTask::new(node.get_instance_config(), ()).unwrap();
        assert_eq!(task.watchdog(), Some(CuDuration::from_millis(250)));
    }

    #[test]
    fn no_safe_stop_before_any_command_seen() {
        let (ctx, clock) = LairContext::new_mock_clock();
        let mut task = SafetyGuardTask::default().with_watchdog(CuDuration::from_millis(10));

        clock.increment(CuDuration::from_millis(1000));
        let empty: CuMsg<ControlCommand> = CuMsg::new(None);
        let mut out: CuMsg<ControlCommand> = CuMsg::new(None);
        task.process(&ctx, &empty, &mut out).unwrap();
        // Never received a command, so nothing to fall back from.
        assert_eq!(out.payload(), None);
    }
}

