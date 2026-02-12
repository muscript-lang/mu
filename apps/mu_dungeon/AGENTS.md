# µDungeon — Agent Instructions (µScript v0.2 demo)

Goal: build a playful deterministic roguelike combat replay to showcase µScript’s token economy and LLM-friendly structure.

## Hard constraints
- Do NOT modify the µScript compiler/runtime unless a clear bug is exposed; if so, minimal fix + regression test.
- Do NOT add language features or stdlib functions.
- Keep changes inside `apps/mu_dungeon/**` unless absolutely necessary.

## Determinism
- No time, no real randomness, no external state.
- All pseudo-randomness must be from a pure PRNG in `rng.mu`, seeded from an integer CLI arg.
- Same seed must produce identical replay output.

## Structure
- `model.mu`: all ADTs (Class, Monster, Status, Action, Event, Outcome)
- `rng.mu`: PRNG + helpers (next, range, choose)
- `rules.mu`: PURE combat step logic; emits events
- `dungeon.mu`: PURE encounter generator from seed
- `main.mu`: IO glue + printing replay

## Output contract
- Print one compact text line per event.
- Print exactly one final `RESULT ...` line.
- Keep output stable and easy to diff.

## Tests
Required:
- PRNG determinism tests
- Status effect behavior tests (Poison, Shield)
- Win/Lose condition tests
- Optional integration test: seed=1 exact RESULT line

## Token economy demo
- Store readable sources.
- Provide a test/script that formats at least `main.mu` and `rules.mu` in readable vs compressed mode and reports:
  - bytes
  - token-like count (using the repo’s improved metric)
- Assert compressed <= 70% bytes and <= 100% tokens of readable.

## Before finishing
Run:
- `muc fmt --mode=readable --check apps/mu_dungeon/src`
- `muc check apps/mu_dungeon/src/main.mu`
- `muc run apps/mu_dungeon/src/main.mu -- 1`
- full test suite
