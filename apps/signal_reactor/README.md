# Signal Reactor (Constrained µScript v0.2 Demo)

Deterministic state-machine demo for token-economy experiments.

Detailed results: see `apps/signal_reactor/FINDINGS.md`.

## Run

```bash
muc run apps/signal_reactor/src/signal_reactor.mu
```

Current runtime limitations in this repo (no CLI args in VM main, no split/atoi/map access) mean this demo uses a default fixture path:
- `apps/signal_reactor/fixtures/sample_signals.txt`

## Input shape

Fixture payload is line-based:
- `b:<int>` buy
- `s:<int>` sell
- `h:<int>` hold
- unknown line -> error action

## Output

One JSON-ish action line per event:
- `{"a":"H"}`
- `{"a":"EL"}`
- `{"a":"XL"}`
- `{"a":"ES"}`
- `{"a":"XS"}`
- `{"a":"ERR"}`

## Files

- `src/model.mu`: ADTs + rendering helpers (reference model module)
- `src/rules.mu`: pure rule/parsing logic + executable assertions in `main`
- `src/signal_reactor.mu`: IO glue app (reads fixture, applies state machine, prints actions)
- `tests/rules_test.mu`: µScript test fixture module
- `fixtures/sample_signals.txt`
- `fixtures/sample_signals_alt.txt`

## Token Economy

Measured by `tests/signal_reactor_token_economy.rs`:
- readable bytes: `2756`
- compressed bytes: `2580`
- readable token-ish count: `487`
- compressed token-ish count: `535`
- compressed/readable byte ratio: `93.61%`

Threshold is set to `<=95%` for this constrained demo.

### Before/After excerpt (decide)

Readable:
```mu
F decide:(State,Signal,i32)->Decision=i(c(>=,arg2,70),m(arg1){Buy=>m(arg0){Idle=>Step(Long(),EnterLong(arg2));Long=>Step(Long(),Hold());Short=>Step(Idle(),ExitShort());};...},...);
```

Compressed:
```mu
F #44:(#32,#31,i32)->#7=[i (#1 #41 70) [m #40 {#6 [m #39 {#19 #33(#21(),#10(#41))} ...]} ...]];
```
