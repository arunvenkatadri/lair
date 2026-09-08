# LAIR website positioning — decision draft

September 8, 2026. Recommendation for discussion; no website change or publication is authorized by this document.

**Recommendation: feature LAIR under Extelligence Research, with an optional secondary link from Bagel.** This connects Extelligence to work beyond data tooling while keeping LAIR's experimental status clear. Bagel's public repository currently centers on robot-data queries and edge data reduction, with support for ROS and Copper data. That gives its visitors a reason to explore the research, but LAIR should have its own explanation and contribution path. [Bagel repository](https://github.com/Extelligence-ai/bagel).

The live Extelligence and Bagel sites could not be retrieved during this review. Placement below is proposed information architecture, not a claim about their current navigation.

| Surface | Proposed placement | Reason |
|---|---|---|
| Extelligence site | Research page with a LAIR project card | Establishes the broader robotics research program and company affiliation |
| Bagel site | Small Research or From Extelligence link in the footer/About area | Provides discovery while preserving the product's clear data-tooling purpose |
| Bagel hero, install flow, product feature list | Keep focused on Bagel | LAIR is a separate experimental runtime; presenting it here could imply a dependency or bundled capability |
| Research launch article / personal site | Explain the question, authorship, Copper foundation, and contribution opportunities | Helps readers associate the actual work with its authors and find experiments to reproduce |

**Suggested project-card copy**

> **LAIR — Robotics systems research**
>
> An experimental platform exploring a successor to ROS. Built on Copper, LAIR investigates new execution architectures and aims to share reusable results with the ROS and Copper communities.
>
> Early development · Targeting a paper and reproducible artifact in early 2027
>
> Explore the project →

The project link should point to the public repository README once these changes are merged and repository visibility is verified. A research page can follow when there is a first experiment to explain. Link individual results to versioned code, methods, and artifacts when they exist.

**Suggested Bagel footer label:** “Robotics research: LAIR”. If the footer links to an Extelligence research index instead, use “Extelligence Research”.

**When to increase prominence:** after a reproducible experiment demonstrates a useful result and an external contributor can run it. At that point, an article explaining the findings gives both ROS and Copper users something concrete to evaluate. Until then, the project card should describe the ambition and current status plainly.

The decision remains open. The recommendation is to provide modest research visibility once the public repo is ready, then let published results earn more prominent placement.

**Naming is also open.** A food-family name could connect the project to Bagel, Matcha, and Pancake. Babka is an initial candidate: short, pronounceable, and suitable as a standalone project name. This is a naming suggestion, not an availability determination. LAIR remains the working name until a replacement is selected and its repository, package, domain, and search collisions are checked. Brioche and Strudel already name established developer tools, so they are weaker candidates for a robotics runtime. [Brioche](https://brioche.dev/), [Strudel](https://strudel.cc/).
