# LAIR Design Notes

## Open Question: Log Format (revisit later)

### Current Understanding
```
Robot writes:  .mcap (or .bag, .db3) - serialized stream
Bagel ingests: converts to Parquet - columnar, queryable  
Matcha stores: Parquet in MotherDuck - SQL queries
```

### Questions to Answer
1. Does LAIR write MCAP directly? Or something else?
2. Is `.lair` a log format or just project config?
3. Do we need our own format or just use MCAP?

### Options Discussed
- **MCAP**: Industry standard, Bagel already reads it, why reinvent?
- **Custom .lair**: Only if we add value MCAP doesn't have
- **DuckDB file**: Good for analysis, not for live recording

### Decision
Deferred. Probably just use MCAP for recording, Parquet for analysis.

---

## Other Decisions Made

### Markets (Commercial/Industrial Only)
1. Autonomous Vehicles
2. Warehouse / Industrial
3. Agriculture / Construction
4. Commercial robotics

NOT: Research, academia, education

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
- LAIR: Open source (Apache 2.0)
- Matcha: SaaS ($$)
- Enterprise support: Contracts ($$)

---

*Last updated: January 2026*

