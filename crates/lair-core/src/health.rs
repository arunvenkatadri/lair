//! Health monitoring and watchdogs.
//!
//! Production robots must notice when a component goes silent and react before a
//! stale sensor or a wedged planner causes harm. This module provides the two
//! primitives LAIR uses for that:
//!
//! - [`Heartbeat`] — a single liveness timer. A component "beats" it each cycle;
//!   if too long passes without a beat, it is considered dead.
//! - [`HealthMonitor`] — aggregates many heartbeats into one [`HealthReport`],
//!   classifying the whole system as [`Healthy`](HealthState::Healthy),
//!   [`Degraded`](HealthState::Degraded), or [`Critical`](HealthState::Critical)
//!   based on which components are stale and how important they are.
//!
//! Everything is driven by an explicit [`CuTime`] timestamp rather than reading a
//! wall clock, so watchdog behavior is fully deterministic and testable with the
//! mockable [`RobotClock`](cu29_clock::RobotClock).
//!
//! When [`HealthReport::requires_safe_state`] is `true`, the application should
//! transition to its safe state — e.g. command `lair-biscuit`'s
//! `PhysicsSafetyValidator::safe_stop` to the actuators (limp-home / fail-safe).
//!
//! # Example
//!
//! ```
//! use lair_core::health::{Criticality, HealthMonitor, HealthState};
//! use lair_core::clock::CuDuration;
//!
//! let t0 = CuDuration::from_millis(0);
//! let mut monitor = HealthMonitor::new();
//! monitor.register("lidar", CuDuration::from_millis(100), Criticality::Critical, t0);
//! monitor.register("telemetry", CuDuration::from_millis(500), Criticality::Optional, t0);
//!
//! // Right away, everything is healthy.
//! assert_eq!(monitor.assess(t0).state, HealthState::Healthy);
//!
//! // 200 ms later with no lidar beat: the critical sensor is stale -> safe state.
//! let report = monitor.assess(CuDuration::from_millis(200));
//! assert_eq!(report.state, HealthState::Critical);
//! assert!(report.requires_safe_state());
//! assert_eq!(report.stale, vec!["lidar"]);
//! ```

use cu29_clock::{CuDuration, CuTime};

/// How important a component is to continued safe operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Criticality {
    /// Losing this component requires transitioning to the safe state.
    Critical,
    /// Losing this component degrades capability but operation can continue.
    Optional,
}

/// Overall system health, ordered from best to worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthState {
    /// Every component is beating within its deadline.
    Healthy,
    /// One or more *optional* components are stale; core function intact.
    Degraded,
    /// One or more *critical* components are stale; safe state required.
    Critical,
}

/// A single liveness timer.
///
/// A component calls [`beat`](Self::beat) every cycle. If the gap between the last
/// beat and "now" exceeds [`timeout`](Self::timeout), the heartbeat is no longer
/// [`alive`](Self::is_alive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heartbeat {
    last_beat: CuTime,
    timeout: CuDuration,
    beats: u64,
}

impl Heartbeat {
    /// Creates a heartbeat that is considered alive as of `now`, allowing at most
    /// `timeout` between beats.
    pub fn new(now: CuTime, timeout: CuDuration) -> Self {
        Self { last_beat: now, timeout, beats: 0 }
    }

    /// Records a beat at time `now`.
    pub fn beat(&mut self, now: CuTime) {
        self.last_beat = now;
        self.beats += 1;
    }

    /// Time elapsed since the last beat as of `now` (saturating; never negative).
    pub fn elapsed(&self, now: CuTime) -> CuDuration {
        CuDuration(now.as_nanos().saturating_sub(self.last_beat.as_nanos()))
    }

    /// Whether the heartbeat is still within its deadline as of `now`.
    pub fn is_alive(&self, now: CuTime) -> bool {
        self.elapsed(now).as_nanos() <= self.timeout.as_nanos()
    }

    /// The configured maximum interval between beats.
    pub fn timeout(&self) -> CuDuration {
        self.timeout
    }

    /// The timestamp of the most recent beat (or creation time).
    pub fn last_beat(&self) -> CuTime {
        self.last_beat
    }

    /// The total number of beats recorded.
    pub fn beats(&self) -> u64 {
        self.beats
    }
}

/// A registered component and its liveness timer.
#[derive(Debug, Clone, Copy)]
struct Component {
    id: &'static str,
    criticality: Criticality,
    heartbeat: Heartbeat,
}

/// The verdict of a [`HealthMonitor::assess`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    /// Aggregate state across all components.
    pub state: HealthState,
    /// IDs of every component that missed its deadline.
    pub stale: Vec<&'static str>,
    /// IDs of the stale components marked [`Criticality::Critical`].
    pub critical_stale: Vec<&'static str>,
}

impl HealthReport {
    /// Whether the system is fully healthy.
    pub fn is_healthy(&self) -> bool {
        self.state == HealthState::Healthy
    }

    /// Whether the system must transition to its safe state (a critical component
    /// is stale).
    pub fn requires_safe_state(&self) -> bool {
        self.state == HealthState::Critical
    }
}

/// Aggregates many [`Heartbeat`]s into a single system [`HealthReport`].
///
/// Register each component once with its deadline and criticality, [`beat`] it as
/// it runs, and call [`assess`] to get the current health verdict.
///
/// [`beat`]: Self::beat
/// [`assess`]: Self::assess
#[derive(Debug, Clone, Default)]
pub struct HealthMonitor {
    components: Vec<Component>,
}

impl HealthMonitor {
    /// Creates an empty monitor.
    pub fn new() -> Self {
        Self { components: Vec::new() }
    }

    /// Registers a component, starting its heartbeat as alive at `now`.
    ///
    /// Re-registering an existing `id` replaces its configuration and resets its
    /// timer.
    pub fn register(
        &mut self,
        id: &'static str,
        timeout: CuDuration,
        criticality: Criticality,
        now: CuTime,
    ) -> &mut Self {
        let heartbeat = Heartbeat::new(now, timeout);
        if let Some(existing) = self.components.iter_mut().find(|c| c.id == id) {
            existing.criticality = criticality;
            existing.heartbeat = heartbeat;
        } else {
            self.components.push(Component { id, criticality, heartbeat });
        }
        self
    }

    /// Records a beat for `id` at `now`. Returns `false` if `id` is not registered.
    pub fn beat(&mut self, id: &str, now: CuTime) -> bool {
        match self.components.iter_mut().find(|c| c.id == id) {
            Some(component) => {
                component.heartbeat.beat(now);
                true
            }
            None => false,
        }
    }

    /// Whether `id` has been registered.
    pub fn is_registered(&self, id: &str) -> bool {
        self.components.iter().any(|c| c.id == id)
    }

    /// The heartbeat for `id`, if registered.
    pub fn heartbeat(&self, id: &str) -> Option<&Heartbeat> {
        self.components.iter().find(|c| c.id == id).map(|c| &c.heartbeat)
    }

    /// Number of registered components.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Whether no components are registered.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Evaluates the health of every component as of `now`.
    pub fn assess(&self, now: CuTime) -> HealthReport {
        let mut stale = Vec::new();
        let mut critical_stale = Vec::new();

        for c in &self.components {
            if !c.heartbeat.is_alive(now) {
                stale.push(c.id);
                if c.criticality == Criticality::Critical {
                    critical_stale.push(c.id);
                }
            }
        }

        let state = if !critical_stale.is_empty() {
            HealthState::Critical
        } else if !stale.is_empty() {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };

        HealthReport { state, stale, critical_stale }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> CuDuration {
        CuDuration::from_millis(n)
    }

    // ── Heartbeat ───────────────────────────────────────────────────

    #[test]
    fn heartbeat_alive_within_timeout() {
        let hb = Heartbeat::new(ms(0), ms(100));
        assert!(hb.is_alive(ms(0)));
        assert!(hb.is_alive(ms(100))); // exactly at deadline still counts
        assert!(!hb.is_alive(ms(101)));
    }

    #[test]
    fn heartbeat_beat_resets_timer() {
        let mut hb = Heartbeat::new(ms(0), ms(100));
        assert!(!hb.is_alive(ms(150)));
        hb.beat(ms(150));
        assert!(hb.is_alive(ms(200)));
        assert_eq!(hb.beats(), 1);
        assert_eq!(hb.last_beat(), ms(150));
    }

    #[test]
    fn heartbeat_elapsed_saturates() {
        let hb = Heartbeat::new(ms(100), ms(50));
        // "now" before last beat must not underflow.
        assert_eq!(hb.elapsed(ms(40)).as_nanos(), 0);
        assert_eq!(hb.elapsed(ms(160)), ms(60));
    }

    // ── HealthMonitor ───────────────────────────────────────────────

    #[test]
    fn monitor_healthy_when_all_fresh() {
        let mut m = HealthMonitor::new();
        m.register("a", ms(100), Criticality::Critical, ms(0));
        m.register("b", ms(200), Criticality::Optional, ms(0));
        let r = m.assess(ms(50));
        assert_eq!(r.state, HealthState::Healthy);
        assert!(r.is_healthy());
        assert!(r.stale.is_empty());
    }

    #[test]
    fn monitor_degraded_when_optional_stale() {
        let mut m = HealthMonitor::new();
        m.register("critical", ms(100), Criticality::Critical, ms(0));
        m.register("optional", ms(100), Criticality::Optional, ms(0));
        m.beat("critical", ms(150)); // keep critical alive
        let r = m.assess(ms(180));
        assert_eq!(r.state, HealthState::Degraded);
        assert!(!r.requires_safe_state());
        assert_eq!(r.stale, vec!["optional"]);
        assert!(r.critical_stale.is_empty());
    }

    #[test]
    fn monitor_critical_when_critical_stale() {
        let mut m = HealthMonitor::new();
        m.register("lidar", ms(100), Criticality::Critical, ms(0));
        let r = m.assess(ms(200));
        assert_eq!(r.state, HealthState::Critical);
        assert!(r.requires_safe_state());
        assert_eq!(r.critical_stale, vec!["lidar"]);
    }

    #[test]
    fn monitor_recovers_after_beat() {
        let mut m = HealthMonitor::new();
        m.register("lidar", ms(100), Criticality::Critical, ms(0));
        assert!(m.assess(ms(200)).requires_safe_state());
        m.beat("lidar", ms(200));
        assert_eq!(m.assess(ms(250)).state, HealthState::Healthy);
    }

    #[test]
    fn beat_unknown_component_returns_false() {
        let mut m = HealthMonitor::new();
        m.register("a", ms(100), Criticality::Optional, ms(0));
        assert!(m.beat("a", ms(10)));
        assert!(!m.beat("ghost", ms(10)));
    }

    #[test]
    fn register_is_idempotent_on_id() {
        let mut m = HealthMonitor::new();
        m.register("a", ms(100), Criticality::Optional, ms(0));
        m.register("a", ms(50), Criticality::Critical, ms(0));
        assert_eq!(m.len(), 1);
        // New criticality takes effect: stale "a" is now critical.
        assert_eq!(m.assess(ms(60)).state, HealthState::Critical);
    }

    #[test]
    fn health_state_ordering() {
        assert!(HealthState::Healthy < HealthState::Degraded);
        assert!(HealthState::Degraded < HealthState::Critical);
    }
}
