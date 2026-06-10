//! Compile-time / build-time graph safety auditing.
//!
//! A [`SafetyGuardTask`](crate::SafetyGuardTask) only protects an actuator if it
//! is actually wired in front of it. This module inspects a parsed task graph and
//! reports any **actuator that receives a `ControlCommand` without a guard in
//! front of it** — so the mistake of forgetting the guard is caught at build time
//! rather than in the field.
//!
//! The rule is deliberately simple and conservative: a *terminal* node (one with
//! no outgoing connections — i.e. an actuator/sink) that consumes a
//! `ControlCommand` must be fed by a [`SafetyGuardTask`]. Any such node fed
//! directly by a non-guard task is flagged.
//!
//! [`ensure_safe_actuation`] is wired into `#[lair_runtime]`, which fails
//! compilation when the graph is unsafe. You can also call it yourself from a test
//! or `build.rs` for the same guarantee.

use std::collections::{BTreeMap, BTreeSet};

use cu29_runtime::config::{ConfigGraphs, CuConfig, CuGraph};

/// The (unqualified) message type a guard protects.
const CONTROL_COMMAND: &str = "ControlCommand";
/// The (unqualified) type name of the guard task.
const GUARD_TASK: &str = "SafetyGuardTask";

/// The last `::`-separated segment of a path, e.g. `"a::b::Foo"` -> `"Foo"`.
fn last_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

fn is_control_command(msg: &str) -> bool {
    last_segment(msg) == CONTROL_COMMAND
}

fn is_guard(type_path: &str) -> bool {
    last_segment(type_path) == GUARD_TASK
}

/// An actuator that receives a control command without a safety guard in front.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnguardedActuator {
    /// The id of the terminal node receiving an unguarded `ControlCommand`.
    pub actuator: String,
    /// The id of the upstream node feeding it directly (not a guard).
    pub source: String,
    /// The mission this graph belongs to (`"default"` for a single-graph config).
    pub mission: String,
}

fn audit_graph(graph: &CuGraph, mission: &str, out: &mut Vec<UnguardedActuator>) {
    // Map node id -> rust type path.
    let node_types: BTreeMap<String, String> = graph
        .get_all_nodes()
        .into_iter()
        .map(|(_, node)| (node.get_id(), node.get_type().to_string()))
        .collect();

    // Any node that is a source of an edge has outgoing connections.
    let has_outgoing: BTreeSet<&str> = graph.edges().map(|cnx| cnx.src.as_str()).collect();

    for cnx in graph.edges() {
        if !is_control_command(&cnx.msg) {
            continue;
        }
        // Only terminal consumers (no outgoing edges) are actuators we must protect.
        if has_outgoing.contains(cnx.dst.as_str()) {
            continue;
        }
        let dst_type = node_types.get(&cnx.dst).map(String::as_str).unwrap_or("");
        let src_type = node_types.get(&cnx.src).map(String::as_str).unwrap_or("");

        // A guard feeding the actuator (or the actuator itself being a guard) is safe.
        if is_guard(src_type) || is_guard(dst_type) {
            continue;
        }
        out.push(UnguardedActuator {
            actuator: cnx.dst.clone(),
            source: cnx.src.clone(),
            mission: mission.to_string(),
        });
    }
}

/// Audits every graph in `config`, returning all unguarded actuators found.
pub fn audit_actuation_safety(config: &CuConfig) -> Vec<UnguardedActuator> {
    let mut findings = Vec::new();
    match &config.graphs {
        ConfigGraphs::Simple(graph) => audit_graph(graph, "default", &mut findings),
        ConfigGraphs::Missions(graphs) => {
            for (mission, graph) in graphs {
                audit_graph(graph, mission, &mut findings);
            }
        }
    }
    findings
}

/// Returns `Ok(())` if every actuator's control commands pass through a guard, or
/// an error message describing the unguarded paths otherwise.
pub fn ensure_safe_actuation(config: &CuConfig) -> Result<(), String> {
    let findings = audit_actuation_safety(config);
    if findings.is_empty() {
        return Ok(());
    }
    let mut msg = String::from(
        "unguarded actuation: ControlCommand reaches an actuator without a \
         SafetyGuardTask. Insert one on the connection(s):",
    );
    for f in &findings {
        msg.push_str(&format!(
            "\n  - `{}` -> `{}` (mission `{}`): route through a SafetyGuardTask, \
             e.g. `{}` -> `safety_guard` -> `{}`",
            f.source, f.actuator, f.mission, f.source, f.actuator
        ));
    }
    Err(msg)
}

/// Reads a RON config file, parses it, and runs [`ensure_safe_actuation`].
///
/// If the file cannot be read or parsed, this returns `Ok(())` rather than
/// failing — the caller (e.g. the `#[lair_runtime]` macro) is a *best-effort*
/// guard, and Copper's own codegen will surface real config errors.
pub fn audit_file(path: &str) -> Result<(), String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };
    match cu29_runtime::config::read_configuration_str(content, Some(path)) {
        Ok(config) => ensure_safe_actuation(&config),
        Err(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cu29_runtime::config::read_configuration_str;

    fn parse(ron: &str) -> CuConfig {
        read_configuration_str(ron.to_string(), None).expect("valid RON")
    }

    const GUARDED: &str = r#"(
        tasks: [
            ( id: "planner",      type: "crate::Planner" ),
            ( id: "safety_guard", type: "lair_biscuit::SafetyGuardTask" ),
            ( id: "actuator",     type: "crate::Actuator" ),
        ],
        cnx: [
            ( src: "planner",      dst: "safety_guard", msg: "lair_msgs::ControlCommand" ),
            ( src: "safety_guard", dst: "actuator",     msg: "lair_msgs::ControlCommand" ),
        ],
    )"#;

    const UNGUARDED: &str = r#"(
        tasks: [
            ( id: "planner",  type: "crate::Planner" ),
            ( id: "actuator", type: "crate::Actuator" ),
        ],
        cnx: [
            ( src: "planner", dst: "actuator", msg: "lair_msgs::ControlCommand" ),
        ],
    )"#;

    const NON_VEHICLE: &str = r#"(
        tasks: [
            ( id: "sensor",    type: "crate::Sensor" ),
            ( id: "processor", type: "crate::Processor" ),
            ( id: "actuator",  type: "crate::Actuator" ),
        ],
        cnx: [
            ( src: "sensor",    dst: "processor", msg: "f32" ),
            ( src: "processor", dst: "actuator",  msg: "f32" ),
        ],
    )"#;

    #[test]
    fn guarded_graph_passes() {
        assert!(audit_actuation_safety(&parse(GUARDED)).is_empty());
        assert!(ensure_safe_actuation(&parse(GUARDED)).is_ok());
    }

    #[test]
    fn unguarded_graph_is_flagged() {
        let findings = audit_actuation_safety(&parse(UNGUARDED));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].actuator, "actuator");
        assert_eq!(findings[0].source, "planner");
        let err = ensure_safe_actuation(&parse(UNGUARDED)).unwrap_err();
        assert!(err.contains("planner"));
        assert!(err.contains("actuator"));
        assert!(err.contains("SafetyGuardTask"));
    }

    #[test]
    fn non_control_command_graph_is_ignored() {
        assert!(audit_actuation_safety(&parse(NON_VEHICLE)).is_empty());
    }

    #[test]
    fn intermediate_control_command_hops_are_not_flagged() {
        // a -> b (CC, b non-terminal) -> guard -> actuator. Only terminal hops matter.
        let ron = r#"(
            tasks: [
                ( id: "a",            type: "crate::A" ),
                ( id: "b",            type: "crate::B" ),
                ( id: "safety_guard", type: "lair_biscuit::SafetyGuardTask" ),
                ( id: "actuator",     type: "crate::Actuator" ),
            ],
            cnx: [
                ( src: "a",            dst: "b",            msg: "lair_msgs::ControlCommand" ),
                ( src: "b",            dst: "safety_guard", msg: "lair_msgs::ControlCommand" ),
                ( src: "safety_guard", dst: "actuator",     msg: "lair_msgs::ControlCommand" ),
            ],
        )"#;
        assert!(audit_actuation_safety(&parse(ron)).is_empty());
    }
}
