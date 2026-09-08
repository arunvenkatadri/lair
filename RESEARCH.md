# LAIR: Copper differentiation and early-2027 research plan

Assessment date: September 8, 2026. This is a source and documentation review, not an experimental validation or an exhaustive novelty search. Proposed contributions below are hypotheses to test.

**Updated research ambition:** the objective is an original robotics contribution that establishes a research identity beyond data infrastructure. See [RESEARCH_AGENDA.md](RESEARCH_AGENDA.md) for the revised recommendation and a concrete novelty test. The safety-contract plan below remains an engineering foundation and an initial, narrower proposal; it should not be treated as sufficient novelty by itself.

The current main branch supplies useful packaging and a different logging backend, but does not yet establish a distinct research contribution. An existing unmerged branch provides a substantial starting point for the safety work.

The planning assumption is a public artifact and preprint in January–March 2027. Peer-reviewed submission and acceptance are separate milestones; acceptance timing cannot be promised. Industrial robots can remain the application domain while the runtime is published as research.

**1. What was reviewed and what counts as implemented**

The checked-out branch is `arunvenkatadri/docs-copper-comparison-differentiation`, at `387381d`, matching the locally recorded `origin/main`. The review covers the top-level design documents, public LAIR crates, selected Copper runtime/logger code, examples, and local Git history.

The locally recorded `origin/claude/project-vision-ls3wcp` branch is at `89c4f96a1590a243dfae5369ec116e0a8d0ac1e2`. It contains eight additional implementation commits after the common ancestor `b3b3eca`. Its code was inspected with `git show`; it was not merged or executed. Remote refs were not refreshed, so these statements describe the available local repository state.

Copper's release notes list **v1.1.1, August 31, 2026**. Its rolling API documentation identifies itself as **1.2.0-dev**; that development documentation is not evidence that every listed feature ships in the stable release. The fork's upstream base SHA is not identified in the reviewed import commit, so this is a capability comparison, not a complete line-by-line attribution audit. Pin an upstream SHA before publishing benchmarks. [Copper release history](https://github.com/copper-project/copper-rs/wiki/Copper-Release-Notes), [rolling API documentation](https://copper-project.github.io/copper-rs/api/cu29/index.html).

**2. How LAIR currently differs from Copper**

| Area | Evidence in LAIR main | Assessment |
|---|---|---|
| Runtime, scheduling, graph compilation, clock, task lifecycle, snapshot interface | Forked `cu29-*` crates; `lair_runtime` delegates to `copper_runtime` | Inherited foundation, not a LAIR contribution. No implemented alternative scheduling framework was found in the public LAIR layer. |
| Public API | `lair-core` aliases Copper concrete types and renames Copper traits | A vocabulary and packaging choice; it does not introduce new execution semantics. |
| Task macro | `lair_task` generates an empty `Freezable` implementation | Convenience for stateless tasks. It does not serialize fields of stateful tasks. |
| CLI | `new`, `build`, `run`, `doctor` | LAIR-specific scaffolding and checks. Build/run wrap Cargo. Useful engineering, weak research novelty. |
| Messages | Geometry, sensors, navigation, and vehicle structs | An opinionated message bundle; vehicle commands/state are concrete application vocabulary. Serialization tests do not establish ROS wire compatibility. |
| Logging | Default host logger switches to an MCAP writer under the `mcap` feature | A concrete implementation choice distinct from Copper's documented native binary logging plus export workflow. Performance and decoder interoperability need measurement. |
| Safety | `SafetyValidator` and `NoOpSafetyValidator` | Interface and passthrough only. No call site enforcing validation at actuator boundaries was found in main. |
| Simulation | `SimBridge` and `MockSimBridge` | Interface plus in-memory mock; no Isaac transport or physics implementation. |
| Bagel/Matcha integration | Logging setup/re-exports; fleet operations in planning documents | No implemented natural-language query, fleet rollout, or cloud analytics pipeline was found in this checkout. |

Local evidence: [API aliases](crates/lair-core/src/lib.rs), [macro implementation](crates/lair-derive/src/lib.rs), [CLI](crates/lair-cli/src/main.rs), [scaffolding](crates/lair-cli/src/new_project.rs), [messages](crates/lair-msgs/src/lib.rs), [MCAP backend](crates/cu29-unifiedlog/src/mcap_backend.rs), [safety stub](crates/lair-biscuit/src/lib.rs), [simulator mock](crates/lair-isaac/src/lib.rs), [logging wrapper](crates/lair-bagel/src/lib.rs).

Several README comparisons need qualification. Copper already has a canonical prelude and project scaffolding through `cargo-cunew`, simulation hooks and a simulated balancebot example, and distributed replay in its documented API. Its component catalog includes sensor payloads and `cu-safetymon`, with watchdog/panic/fault behavior. Thus “no project toolchain,” “no simulator integration,” and a blanket absence of safety support are unsuitable comparisons. A specific actuator command contract could still differ from runtime fault monitoring. [Copper templates](https://copper-project.github.io/copper-rs/Project-Templates/), [Copper API](https://copper-project.github.io/copper-rs/api/cu29/index.html), [component catalog](https://cdn.copper-robotics.com/catalog/index.html).

Copper also documents safety-case IDs and requirement-check macros, and v1.1 adds bounded anytime computation. Generic traceability, watchdogs, and bounded refinement therefore should not be claimed as new LAIR concepts. [Copper release notes](https://github.com/copper-project/copper-rs/wiki/Copper-Release-Notes).

The MCAP distinction deserves care. Copper documents native `.copper` logging and MCAP export. LAIR's backend writes bincode-encoded sections to `/lair/*` channels directly. An MCAP container does not by itself make those payloads understandable to every viewer. `append` calls `encode_to_vec`, reads wall time, takes a mutex, and writes through a buffered file writer; `preallocated_size` is ignored and the builder currently supports only write/create. These code paths justify measuring allocation, write latency, and replay compatibility rather than carrying over Copper's performance numbers. [Copper logging documentation](https://copper-project.github.io/copper-rs-book/logging-replay.html), [LAIR backend](crates/cu29-unifiedlog/src/mcap_backend.rs).

**3. Existing work outside main changes the starting point**

At `89c4f96`, the unmerged branch adds physics bounds, a bicycle-model steering envelope, state-aware validation, `SafeCommand`, `SafetyGuard`, a graph guard task, graph auditing, health monitoring, watchdog behavior, CLI recording/inspection, and typed MCAP replay sources. These are real implementations, not just roadmap entries. Their correctness is not established by commit descriptions saying tests passed.

Before presenting that branch as safety enforcement, resolve these specific gaps:

| Code at `89c4f96` | Observed limitation | Research implication |
|---|---|---|
| `lair-biscuit/src/lib.rs`: `SafetyGuard::guard` | Advisory mode wraps a rejected command in `SafeCommand`; the type is also `Copy` and has no expiry/context binding | “Safe” does not imply approved for the present actuator state or time. Separate advisory outcomes from actuation authority. |
| `lair-biscuit/src/task.rs`: `SafetyGuardTask` | Consumes/outputs raw `ControlCommand`; calls static `enforce`, without `VehicleState` | State-aware physics checks are not actually applied by this graph task. |
| `lair-biscuit/src/audit.rs` | Matches final string segments of type names; treats terminal consumers as actuators | This is a wiring heuristic. Aliases, lookalike names, and actuator tasks with outgoing telemetry need explicit handling. |
| `lair-biscuit/src/task.rs`: watchdog | Refreshes on any present payload, without checking its source timestamp; executes on the same runtime | Repeated stale data is not silence. A hung executor cannot run its own guard. |
| `lair-biscuit/src/task.rs` and `lair-bagel/src/replay.rs` | Stateful tasks use empty `Freezable` implementations | Watchdog state and replay index are not restored by those implementations. |
| `lair-bagel/src/replay.rs` | Emits one payload per cycle; fixture writer uses ordinal timestamps; reader discards timing | Ordered playback is not faithful timed replay or a closed-loop counterfactual experiment. |

Inspect the exact files without changing branches using, for example, `git show 89c4f96:crates/lair-biscuit/src/task.rs`. Main has the same empty-freeze default through [Copper's `Freezable`](crates/cu29-runtime/src/cutask.rs) and [LAIR's task macro](crates/lair-derive/src/lib.rs).

These gaps suggest a tractable research question: **can a compiled task graph enforce state- and time-dependent actuation contracts, with a precise software guarantee and a reproducible record of every intervention?**

**4. Ranked directions for further differentiation**

| Priority | Direction | Potential contribution | Early-2027 scope |
|---|---|---|---|
| 1 | Actuator contracts that include state freshness and validity duration | Graph checking plus enforcement at the resource that can issue hardware commands; explicit assumptions about state, timing, and fallback | Best fit for the existing safety branch. One vehicle model and one robot class. |
| 2 | Replayable safety evidence and regression generation | Reconstruct each decision from its actual inputs, monitor state, policy version, and clock; turn failures into small repeatable tests | Build as supporting evidence for direction 1; expand into its own paper only if the methodology is distinct. |
| 3 | Fleet-derived contract tests under a data budget | Select incident windows that preserve safety failures and rare-condition coverage under constrained upload/storage | Fits Bagel/Matcha commercially, but requires representative fleet traces or a credible substitute. Higher data risk. |

For direction 1, define a limited contract language: command bounds, required state fields, maximum observation age, finite/valid inputs, validity interval, actuator identity, policy version, and a specified fallback. Example: a steering command approved using a low-speed estimate must be checked again or refused if that estimate is stale when actuation occurs.

Compile those declarations into graph checks and a small Rust enforcement path. Explicitly identify actuator resources instead of inferring them from graph out-degree. Bind authorization to the command and relevant state/policy/time, and have the trusted driver enforce that binding. A sealed type alone cannot prevent arbitrary application code from opening a device or replaying an old approval. State whether the model covers accidental application errors or adversarial code; OS/device isolation is additional work for the latter.

The proof target should be narrow: under declared access, timing, and state assumptions, a conforming actuator driver accepts only commands with a currently valid contract decision, or emits the specified fallback. Separately evaluate whether the contract and fallback preserve a physical invariant under the chosen dynamics and uncertainty bounds. A generic full-brake command is not, by itself, a proof of a safe trajectory. Keep SQL/NLP for offline authoring and analysis unless bounded runtime behavior is demonstrated.

For direction 2, record the raw command, state estimate and timestamp, monitor state, decision/reason, corrected output, policy/config/build identity, and source message IDs. Restore those inputs and internal state to reproduce the decision. Distinguish three experiments: reproducing the original run, evaluating another policy on fixed recorded inputs, and closed-loop resimulation where changed actions alter subsequent sensor data. Only the last can estimate physical outcomes after a counterfactual action. Define exact replay equality for deterministic CPU components and explicit tolerances for any nondeterministic simulator/GPU outputs.

For direction 3, ask a measurable question: how much data can be removed while retaining failure reproduction and contract coverage? Compare fixed windows, random sampling, anomaly-based selection, and a proposed contract-aware selector. Measure reproduction rate, retained failure classes, bytes transferred, and regression yield. A natural-language front end or fleet dashboard alone would provide little research distinction.

**5. Prior work sets a higher bar than “Copper plus safety”**

SOTER already provides declarative runtime-assurance modules combining an advanced controller, a safe controller, and safety specifications, with formal guarantees and robot experiments. SOTER on ROS extends that line to ROS and distributed mobile robots. LAIR needs a specific advance over those approaches, not merely a Rust implementation. [SOTER, DSN 2019](https://escholarship.org/uc/item/78g5j78r), [SOTER on ROS, RV 2020](https://arxiv.org/abs/2008.09707).

Temporal monitoring also exists: RTAMT supports online/offline monitoring of signal temporal logic. Measurement-robust control barrier functions address safety with erroneous state estimates, including physical experiments. Therefore neither “monitor temporal rules” nor “include state uncertainty” is sufficient as a novelty claim. [RTAMT implementation](https://github.com/nickovic/rtamt), [measurement-robust CBF research](https://authors.library.caltech.edu/records/k65xp-y9147).

Scenic and VerifAI already address scenario specification, generation, and analysis of AI-based systems. Use them as related work or experimental infrastructure rather than presenting simulator-driven falsification as new. [VerifAI authors' publication list](https://github.com/BerkeleyLearnVerify/VerifAI/blob/main/docs/publications.rst).

The candidate distinction is the **specific connection between compiled graph/resource ownership, time-valid actuator authorization, and replay of enforcement state**, with measured cost and stated guarantees. This focused search does not establish that combination is novel. Before committing the paper, examine the closest systems' full papers and implementations, including language-based capabilities, runtime assurance, and timed dataflow work.

**6. What the first paper should demonstrate**

Suggested working title: *LAIR: Compiled Actuation Contracts with Reproducible Runtime Assurance for Robot Task Graphs*. This describes a proposed outcome, not a current capability.

Use one controlled warehouse-style mobile-robot workload with the same planner, controller, plant, and workloads across baselines. Start with a deterministic CPU simulation; add one physical platform or hardware-in-the-loop experiment if available. Isaac is useful if it answers an experimental question, but implementing a broad Isaac integration need not be on the critical path.

| Question | Comparison | Evidence to collect |
|---|---|---|
| Does the method prevent unsafe software paths? | Unguarded graph, handwritten guard, typed guard, full contract enforcement | Compile-fail tests and runtime fault cases for aliases, nonterminal actuators, expired approvals, stale state, NaNs, startup silence, policy changes, and bypass attempts within the stated model |
| Does it improve physical outcomes without simply stopping constantly? | Same safety policy in a conventional application-level runtime-assurance wrapper | Invariant violations, stopping distance, intervention frequency, unnecessary interventions, task completion, behavior under model mismatch |
| What is its runtime cost? | Pinned upstream Copper and the same application with LAIR enforcement | Median/p99/p99.9 latency, deadline misses, CPU/memory, allocations, logging bandwidth; log-disabled, native logger, and MCAP variants |
| Are decisions reproducible? | Replay from start and from checkpoints; ablate monitor-state/timestamp capture | Decision/output agreement, successful failure reproduction, artifact size, and failure localization time |

Use the same underlying validator in the application-wrapper and LAIR variants to isolate the value of enforcement placement. Include a relevant SOTER-style assurance baseline or a clearly documented adaptation; do not compare only against an unprotected system. Copper alone is the essential engine baseline. ROS 2 is optional context if there is time for a fair equivalent implementation.

Predeclare scenarios and metrics, retain held-out seeds, report repeated-trial uncertainty, and publish failures as well as successes. Observed maximum latency is not a WCET proof, and zero failures in a finite campaign is not a universal safety guarantee. Define a small analytical guarantee separately from empirical results.

**7. Proposed delivery sequence**

| Window | Deliverable and completion gate |
|---|---|
| September 2026 | Identify upstream fork provenance; review/reconcile the existing safety branch; choose robot, property, and failure model; reproduce a motivating stale-state or stale-command failure. |
| October 2026 | Implement the bounded contract/driver path, state-aware graph guard, configuration validation, and correct monitor snapshots. Establish the software guarantee and bypass/fault tests. |
| November 2026 | Add decision traces and checkpoint replay; run baseline and ablation experiments. An independent user should be able to reproduce one failure and its prevention. |
| December 2026 | Freeze the evaluated implementation; complete held-out experiments and a hardware/HIL case if available; write results and limitations; prepare reproducible scripts and data. |
| January–March 2027 | Release a versioned artifact and preprint, obtain external replication feedback, and submit to a venue matched to the resulting contribution. |

This schedule assumes focused engineering and access to a suitable test platform. It is not a staffing estimate. If the hardware or proof effort expands, keep the first release scoped to an honest simulation-backed systems result. Defer fleet SaaS, Kubernetes, multiple schedulers, general package management, broad simulator coverage, and certification tooling unless an experiment specifically needs them.

**8. Documentation and release issues to resolve before publication**

- [SPEC.md](SPEC.md) says “built from scratch” and presents planned schedulers, SQL safety, package commands, and cloud functions as available. Label it as historical design intent and reconcile it with the implementation.
- [AGENTS.md](AGENTS.md), [NOTES.md](NOTES.md), and [ROADMAP.md](ROADMAP.md) describe a design-only project or exclude research. The code has moved on, and the requested publication goal supersedes that earlier positioning. Preserve the industrial application focus while recognizing research as an output.
- [README.md](README.md) accurately calls safety a stub in one place but also says every command flows through a validator. Main does not enforce that path. Refresh the Copper comparison and separate main, unmerged prototype, and planned functionality.
- Document actual package/feature names. `crates/lair` is still published under package name `cu29`; the main manifest does not expose the advertised `bagel`, `biscuit`, `matcha`, and `isaac` integration switches, and the CLI does not forward `--features`. Verify generated projects against the intended dependency source/version before claiming a working public quickstart.
- Establish real state snapshot coverage before promising deterministic replay. Keep current performance and physical-safety claims tied to measured configurations and explicit assumptions.
- Preserve Copper attribution and license notices, identify the upstream base commit, and publish a change inventory. Prefer upstream dependencies or a small maintained patch set if feasible; a large fork increases the work needed to isolate and evaluate LAIR's contribution.

No runtime code was changed and no build, test suite, robot experiment, or performance benchmark was run for this assessment. Existing tests and branch commit messages were inspected as evidence of development activity, not treated as independently verified results.
