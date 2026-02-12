# Signal Reactor — Agent Instructions (µScript app)

This folder contains an end-to-end µScript v0.2 demo app focused on:
- Deterministic behavior
- Effects discipline
- Token economy (readable vs compressed canonical)
- High test coverage on decision logic

## Hard constraints
- DO NOT modify the µScript compiler/toolchain unless the app reveals a clear bug.
  - If a compiler fix is necessary: keep it minimal, add a regression test, and explain the bug clearly.
- DO NOT add new language features or stdlib functions.
- Keep all changes scoped to `apps/signal_reactor/**` unless a minimal bugfix is required.

## Coding style (µScript)
- Prefer small ADTs + `match` over ad-hoc JSON probing.
- Handle JSON errors explicitly with `Res[...]` and return `Action::Error(...)` deterministically.
- Keep the app deterministic: no time/rand usage.

## Required structure
- `model.mu`: ADTs and JSON decoding helpers
- `rules.mu`: pure decision logic `decide(state,event)->(state,action)`
- `signal_reactor.mu`: IO glue (fs/net + json parsing + output)

## Output contract
- Print exactly one JSON object per input event to stdout.
- The last line must be a valid action JSON record.
- Logging (if any) must be consistent and not interleaved with JSON output.

## Token economy demonstration
- Provide both:
  - readable source committed in repo
  - proof that `muc fmt --mode=compressed` yields substantially smaller output
- Include a small test or script that computes:
  - byte length readable vs compressed
  - a simple token-ish count
  - assert compressed <= 70% of readable (adjust only if necessary, document why)

## Tests
- Unit tests must cover:
  - Idle/Long/Short transitions on buy/sell/hold
  - confidence threshold boundary cases (0.69, 0.70, 0.71)
  - malformed JSON input -> Action::Error
- Integration test must:
  - run the main module on the fixture file
  - verify number of output lines and at least a few exact JSON outputs

## Verification commands
Run these before finishing:
- `muc fmt --mode=readable --check apps/signal_reactor/src`
- `muc fmt --mode=compressed --check apps/signal_reactor/src`
- `muc check apps/signal_reactor/src/signal_reactor.mu`
- `muc run apps/signal_reactor/src/signal_reactor.mu`
- run the repo’s test suite (or the minimal subset that covers this app)
