# µArena — Agent Instructions

Goal: Deterministic batch/tournament runner for µDungeon, showcasing µScript v0.2 token economy and reproducible simulation.

## Hard constraints
- Do NOT modify compiler/runtime or add language features.
- Keep changes inside `apps/mu_arena/**`.
- If µDungeon needs small refactor to expose pure entrypoints, keep it minimal and add regression tests so µDungeon output is unchanged.

## Determinism
- Same CLI args -> identical output.
- All randomness from explicit PRNG state (no time/rand).

## Policies
Implement at least:
- baseline
- aggressive
- defensive
Policies must be pure and must differ meaningfully.

## Output
Compact deterministic text:
- header
- policy blocks
- final BEST summary line

## Aggregation without maps
Assume limited data structures:
- Use ADTs and fixed tuples for aggregates.
- Avoid needing array/map indexing unless already supported.

## Tests
Must include:
- deterministic batch aggregation test
- policy difference test
- optional golden BEST line

## Token economy proof
Add test measuring readable vs compressed for µArena:
- compressed bytes <= 70% readable
- compressed tokens <= readable
- print symtab stats

## Before finishing
Run:
- `muc fmt --mode=readable --check apps/mu_arena/src`
- `muc check apps/mu_arena/src/main.mu`
- `muc run apps/mu_arena/src/main.mu -- --seeds 20 --start 1`
- repo test suite
