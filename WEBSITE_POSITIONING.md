# Sencha: personal site and experiment journal

Decision recorded September 8, 2026. Sencha will begin as Arun Venkatadri's personal robotics systems research project, with a standalone site on one of his existing domains. The domain remains to be selected; this document records the publishing plan.

**Launch independently, publish the experiments, and bring Sencha onto the Extelligence site once the work has traction.** The first public home will carry personal authorship, the Copper foundation, and the ambition to explore a successor to ROS. Extelligence placement comes later. Bagel cross-linking can be reconsidered at that point.

## The initial site

Keep the first site small enough that conducting experiments remains the main work:

| Page | Purpose |
|---|---|
| Home | State the research question, introduce Arun, explain the Copper foundation, and link the latest experiment and repository |
| Experiments | Index research reports with their question, date, outcome, and artifact links |
| Blog | Publish development notes, ideas, and progress between completed experiments |
| About | Explain the personal project, research ambition, attribution, and how ROS and Copper contributors can participate |

Suggested introduction:

> **Sencha — Experiments in robotics systems**
>
> I'm Arun Venkatadri. Sencha is my personal research project exploring a successor to ROS, built in Rust on Copper. Every experiment aims to produce results the ROS and Copper communities can learn from. I publish the methods, code, and findings here so others can reproduce and build on the work.

The site should link to the [repository](https://github.com/arunvenkatadri/sencha) and make implementation status clear. Sencha is named after the tea Arun drinks every morning. It does not need an acronym or a company launch to explain its identity.

## Every experiment gets a research report

**Every experiment must aim to produce results that both the ROS and Copper communities can learn from.** Before implementation, identify the question and the potential lesson for each community. Afterward, report what the evidence supports, how it could inform existing systems, and which methods or artifacts others can reuse. The lessons may differ between communities; benefit does not depend on adopting Sencha or on Sencha outperforming either baseline.

Publish each completed experiment as a paper-style report with a readable web version, stable URL, named authors, publication date, and version. Use the [experiment report template](EXPERIMENT_REPORT_TEMPLATE.md). A short blog introduction can explain why the question matters and link to the full report. PDF export is useful for sharing, but the web report and reproducible artifact are the first requirements.

Each report should state a question and hypothesis, situate the work against relevant prior methods, describe an experiment another person can reproduce, show the results, and explain the limits of the conclusion. Distinguish inherited Copper behavior from Sencha changes. Link the exact code revision, configurations, run commands, and available measurements.

Negative results and replications deserve reports. State what was learned without forcing a novelty claim. Label self-published work as an **experiment report** or **technical report**; identify peer-reviewed versions separately if they exist. Preserve earlier versions and record material corrections so cited findings remain traceable.

Let completed experiments determine the report cadence. Use the blog for interim notes. The early-2027 paper should synthesize a contribution supported by this body of evidence and its reproducible artifact.

## When to add Extelligence

Revisit Extelligence placement when there is a coherent set of reports, a useful result that survives fair baseline comparisons, and evidence that someone outside the project can reproduce or use it. These are proposed signals of traction, not a fixed publication quota or a commitment to a date.

At that point, add a Sencha research entry to the Extelligence site linking to the established project home. Preserve report URLs and personal authorship. A later company affiliation can grow around the work without moving its publication history. Decide on Bagel placement separately based on relevance to its readers.
