//! Integration: when the watchdog reports a critical component stale, the safety
//! layer's fail-safe (`safe_stop`) is what reaches the actuators.
//!
//! This stitches together `lair-core`'s health monitoring (item #2) and
//! `lair-biscuit`'s graceful-degradation path (item #1).

use lair_biscuit::{PhysicsSafetyValidator, SafeCommand, SafetyGuard, StatefulSafetyValidator};
use lair_core::health::{Criticality, HealthMonitor};
use lair_core::clock::CuDuration;
use lair_msgs::{ControlCommand, Gear, VehicleState};

fn ms(n: u64) -> CuDuration {
    CuDuration::from_millis(n)
}

/// The fail-safe controller: while healthy it runs the planner's command through
/// the safety guard; when a critical component goes stale it commands a safe stop.
/// Either way it can only ever emit a `SafeCommand`.
fn decide(
    monitor: &HealthMonitor,
    guard: &mut SafetyGuard,
    validator: &PhysicsSafetyValidator,
    planner_cmd: &ControlCommand,
    state: &VehicleState,
    now: CuDuration,
) -> SafeCommand {
    if monitor.assess(now).requires_safe_state() {
        // A controlled stop, still produced through the validator so it is a
        // genuinely safe command (and we know it validates).
        validator
            .approve(&validator.safe_stop(state.gear), state)
            .expect("safe_stop must validate")
    } else {
        *guard.guard(planner_cmd, state).command()
    }
}

#[test]
fn critical_stale_drives_a_safe_stop() {
    let validator = PhysicsSafetyValidator::default();
    let mut guard = SafetyGuard::enforced();

    let mut monitor = HealthMonitor::new();
    monitor.register("perception", ms(100), Criticality::Critical, ms(0));

    let state = VehicleState { velocity: 12.0, gear: Gear::Drive, ..Default::default() };
    let planner_cmd =
        ControlCommand { throttle: 0.4, brake: 0.0, steering: 0.1, gear: Gear::Drive };

    // Perception is fresh: the planner's command flows through unchanged.
    monitor.beat("perception", ms(50));
    let healthy = decide(&monitor, &mut guard, &validator, &planner_cmd, &state, ms(60));
    assert_eq!(healthy.throttle, 0.4);
    assert_eq!(healthy.brake, 0.0);

    // Perception goes silent past its deadline: we must stop.
    let failed = decide(&monitor, &mut guard, &validator, &planner_cmd, &state, ms(300));
    assert_eq!(failed.throttle, 0.0);
    assert_eq!(failed.brake, 1.0);
    assert_eq!(failed.steering, 0.0);

    // Whatever happens, the actuator only ever sees validated commands.
    assert!(validator.validate_with_state(failed.get(), &state).is_ok());
}
