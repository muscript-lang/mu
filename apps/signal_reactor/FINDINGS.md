# Signal Reactor Findings

## Summary

This constrained µScript demo was implemented without compiler/runtime changes and validated end-to-end.

## Verified Outcomes

- `muc fmt --mode=readable --check apps/signal_reactor/src` passes.
- `muc check apps/signal_reactor/src/signal_reactor.mu` passes.
- `muc run apps/signal_reactor/src/signal_reactor.mu` passes and emits deterministic compact action lines (`H`, `EL`, `XS`, etc.) by default.
- App-level tests pass:
  - `cargo test --test signal_reactor_app -- --nocapture`
  - `cargo test --test signal_reactor_token_economy -- --nocapture`

## Token Economy Measurements

Measured by `tests/signal_reactor_token_economy.rs` across:
- `apps/signal_reactor/src/model.mu`
- `apps/signal_reactor/src/rules.mu`
- `apps/signal_reactor/src/signal_reactor.mu`

- Readable bytes (total): `8266`
- Compressed bytes (total): `5838`
- Readable lexer-token count (total): `2753`
- Compressed lexer-token count (total): `2520`
- Compressed/readable byte ratio: `70.62%`
- Compressed/readable token ratio: `91.54%`
- Compressed symtab size (total): `76`
- Compressed `#n` width: avg `1.37`, max `2`

Threshold status:
- bytes target (`<=75%`): pass
- token target (`<=100%`): pass

## Compressed Excerpt

```mu
F #27:(#13,#19,i32)->#25=[i (>= arg2 70) [m arg1 {#3 [m arg0 {#10 #2(Long(),#15(arg2))} {Long #2(Long(),#5())} {#14 #2(#10(),#18())}]} {#4 [m arg0 {#10 #2(#14(),#16(arg2))} {Long #2(#10(),#17())} {#14 #2(#14(),#5())}]} {#6 #2(arg0,#5())} {#7 #2(arg0,#9("bad_signal"))}] [m arg1 {#7 #2(arg0,#9("bad_signal"))} {_ #2(arg0,#5())}]];
```

## Why This Is Significant

The gains are substantial and practical for agent-centric workflows:

- Repeated identifiers are heavily compacted via `$[...]` + `#n`, reducing repeated long symbol names.
- Compressed canonical output is highly regular, which improves prompt stability for LLM edit/review loops.
- Smaller source payloads reduce cumulative token spend across repeated cycles (plan, generate, review, patch).
- The format preserves semantics while making structural patterns (`[i ...]`, `[m ...]`, s-expr calls) denser and more uniform.

In short: this demo confirms practical compression benefits under strict language constraints, while preserving deterministic behavior and full type/effect checking.
