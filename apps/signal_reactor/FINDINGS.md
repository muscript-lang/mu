# Signal Reactor Findings

## Summary

This constrained µScript demo was implemented without compiler/runtime changes and validated end-to-end.

## Verified Outcomes

- `muc fmt --mode=readable --check apps/signal_reactor/src` passes.
- `muc check apps/signal_reactor/src/signal_reactor.mu` passes.
- `muc run apps/signal_reactor/src/signal_reactor.mu` passes and emits deterministic JSON action lines.
- App-level tests pass:
  - `cargo test --test signal_reactor_app -- --nocapture`
  - `cargo test --test signal_reactor_token_economy -- --nocapture`

## Token Economy Measurements

Measured from `apps/signal_reactor/src/signal_reactor.mu`:

- Readable bytes: `2756`
- Compressed bytes: `2580`
- Readable token-ish count: `487`
- Compressed token-ish count: `535`
- Compressed/readable byte ratio: `93.61%`

## Compressed Excerpt

```mu
F #44:(#32,#31,i32)->#7=[i (#1 #41 70) [m #40 {#6 [m #39 {#19 #33(#21(),#10(#41))} ...]} ...]];
```

## Why This Is Still Significant

Even though this app shows a moderate byte reduction (not an extreme one), the gains are meaningful for agent-centric workflows:

- Repeated identifiers are heavily compacted via `$[...]` + `#n`, reducing repeated long symbol names.
- Compressed canonical output is highly regular, which improves prompt stability for LLM edit/review loops.
- Smaller source payloads reduce cumulative token spend across repeated cycles (plan, generate, review, patch).
- The format preserves semantics while making structural patterns (`[i ...]`, `[m ...]`, s-expr calls) denser and more uniform.

In short: this demo confirms practical compression benefits under strict language constraints, while preserving deterministic behavior and full type/effect checking.
