# Sencha research agenda: a credible successor to ROS

September 8, 2026. The primary objective is to build a successor to ROS. Original systems research should establish its technical case and invite the robotics community to evaluate and develop it. Candidate contributions here are not established novelty claims.

**Product ambition: replace ROS as the foundation of a robot application. Research obligation: demonstrate why a different architecture produces capabilities or guarantees that existing approaches do not adequately provide.** A robotics runtime can itself be the research artifact. It does not need an unrelated learning algorithm to justify publication, but it does need more than a new brand, language, API, or collection of integrations.

**Proposed architectural bet: make end-to-end execution requirements part of the robot program.** Explore a model in which sensing-to-actuation paths declare data freshness, timing, and fault-response requirements, and composition rules determine what can be guaranteed when components are connected. Compilation and runtime enforcement should agree about those requirements. Where a black-box component or external ROS boundary prevents a guarantee, represent that limit explicitly rather than implying whole-system assurance.

This goes beyond adding a scheduler switch or validating an individual command. The scientific question is whether a new composition/enforcement method can establish useful system-level properties across realistic heterogeneous components, with acceptable cost and substantially less manual coordination. Pick one property for the first paper—such as bounded observation age at actuation with a specified response to violations—and investigate the strongest existing methods before choosing the final contribution.

This must be compared against **both Copper and serious ROS-based alternatives**. Copper already supplies the compiled task-graph foundation. ROS's own documentation discusses executor limitations but also deterministic sequences through WaitSet and logical execution time in rclc. PiCAS addresses chain-aware scheduling; a recent Lingua Franca paper addresses deterministic execution of existing ROS 2 code. Merely bringing deterministic execution to ROS components is therefore already occupied territory. [ROS executor documentation](https://docs.ros.org/en/rolling/Concepts/Intermediate/About-Executors.html), [PiCAS paper and implementation](https://github.com/rtenlab/ros2-picas), [ROS 2 execution through Lingua Franca](https://arxiv.org/abs/2606.09203).

**Adoption should be incremental even if the intended destination is replacement.** Begin with an existing robot's bounded sensor-to-actuator path. Make it possible to keep useful drivers, algorithms, message definitions, and visualization tools while that path moves into Sencha. Provide a native Sencha implementation and a measured compatibility boundary; do not silently grant native timing/replay guarantees to opaque external nodes. Expand toward a complete robot only after the initial migration demonstrates an advantage.

For early 2027, a credible deliverable would be one new execution/composition method, a clear account of what is inherited from Copper, an end-to-end robot demonstration, a fair comparison against the strongest applicable baselines, and a migration example another ROS developer can reproduce. Measure robot behavior, timing/freshness violations, resource overhead, fault response, and integration effort. Matching an existing method in Rust would establish an implementation result, not automatically the original contribution sought here.

**Community strategy: make ROS users collaborators in solving their problems.** Describe precise failure modes, credit existing solutions, publish reproducible comparisons, and make the software useful before requiring a wholesale rewrite. Public positioning can state the ambition directly: “Sencha explores a new execution architecture for dependable robot applications, with an incremental migration path from ROS.” Commercial deployment and academic collaboration can support the same architecture.

**Copper is also a beneficiary and potential collaborator.** Sencha can host architectural experiments while Copper supplies the execution foundation. Publish reusable methods, benchmarks, negative results, and small implementation changes so Copper contributors can adopt useful pieces independently of Sencha. Upstream adoption is a successful research outcome, not a loss of project identity. Preserve explicit attribution and distinguish inherited machinery from experimental changes. The long-term ambition to succeed ROS does not require every successful experiment to remain exclusive to Sencha.

A concise project statement: “Sencha is an experimental robotics systems platform exploring a successor to ROS. Built on Copper, it develops and evaluates new execution architectures, with reusable results for the ROS and Copper communities.” This describes the intended program, not an assertion that a new architecture has already been demonstrated.

The next research step is a focused comparison of composition models and an experiment that exposes their limits—not implementing the entire roadmap. The earlier repository assessment remains the factual inventory of the starting code.


**First experiment and early-2027 target.**

1. Identify a repeatable limitation on a real sensor-to-actuator path and reproduce the strongest applicable Copper and ROS baselines.
2. State a candidate composition or enforcement rule, its assumptions, and the behavior it should improve. Select one property for the first experiment.
3. Implement the smallest method that tests the hypothesis; count enforcement overhead and compatibility boundaries in the evaluation.
4. Compare robot outcomes, timing/freshness violations, fault response, resource use, and integration effort. Retain held-out scenarios and simple heuristic baselines.
5. Publish versioned code, reproducible experiments, limitations, and a paper. Share reusable pieces upstream.

The early-2027 target is a paper and artifact, with the first architectural contribution selected by evidence. Publication acceptance and production readiness are separate milestones. Broader fleet infrastructure, simulator support, and product packaging should follow the experiments that justify them.

**Public discovery.** See [WEBSITE_POSITIONING.md](WEBSITE_POSITIONING.md) for proposed Extelligence/Bagel link placement and copy. The project is now named Sencha; website placement remains an open decision.
