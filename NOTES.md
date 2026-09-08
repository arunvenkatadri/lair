# Sencha Design Notes

September 2026 positioning: Sencha explores a successor to ROS through original systems research on a Copper foundation. Industrial robots remain the application focus; ROS and Copper contributors and academic collaborators are welcome. See [RESEARCH_AGENDA.md](RESEARCH_AGENDA.md). The January notes below record historical design discussions, not verified implementation status.

## Open Question: Log Format (revisit later)

### Current Understanding
```
Robot writes:  .mcap (or .bag, .db3) - serialized stream
Bagel ingests: converts to Parquet - columnar, queryable  
Matcha stores: Parquet in MotherDuck - SQL queries
```

### Questions to Answer
1. Does Sencha write MCAP directly? Or something else?
2. Is `.sencha` a log format or just project config?
3. Do we need our own format or just use MCAP?

### Options Discussed
- **MCAP**: Industry standard, Bagel already reads it, why reinvent?
- **Custom .sencha**: Only if we add value MCAP doesn't have
- **DuckDB file**: Good for analysis, not for live recording

### Decision
Deferred. Probably just use MCAP for recording, Parquet for analysis.

---

## Other Decisions Made

### Application Markets
1. Autonomous Vehicles
2. Warehouse / Industrial
3. Agriculture / Construction
4. Commercial robotics

Research and academic collaboration are part of the project; these markets describe the application focus.

### Feature Flags
```toml
biscuit = []  # Optional - LLM safety, NOT for AV
bagel = []    # Optional - logging
matcha = []   # Optional - cloud sync
ml = []       # Optional - inference
```

### Biscuit
- Currently: LLM output validation with physics constraints
- Good for: Service robots, manipulation, voice commands
- NOT for: AV, safety-critical (no LLMs in the loop)
- Future: May evolve into true physical model (non-LLM)

### Business Model
- Sencha: Open source (Apache 2.0)
- Matcha: SaaS ($$)
- Enterprise support: Contracts ($$)

---

*Last updated: January 2026*
