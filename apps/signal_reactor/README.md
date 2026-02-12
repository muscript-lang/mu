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

By default, one compact action code per event:
- `H`
- `EL`
- `XL`
- `ES`
- `XS`
- `ERR`

`emit_json` in `src/signal_reactor.mu` can be toggled to emit JSON-ish lines instead.

## Files

- `src/model.mu`: ADTs + rendering helpers (reference model module)
- `src/rules.mu`: pure rule/parsing logic + executable assertions in `main`
- `src/signal_reactor.mu`: IO glue app (reads fixture, applies state machine, prints actions)
- `tests/rules_test.mu`: µScript test fixture module
- `fixtures/sample_signals.txt`
- `fixtures/sample_signals_alt.txt`

## Token Economy

Measured by `tests/signal_reactor_token_economy.rs`:
- readable bytes (total across app modules): `8266`
- compressed bytes (total across app modules): `5838`
- readable lexer-token count: `2753`
- compressed lexer-token count: `2520`
- compressed/readable byte ratio: `70.62%`
- compressed/readable token ratio: `91.54%`
- compressed symtab size (total): `76`
- compressed `#n` width: avg `1.37`, max `2`

Current thresholds:
- compressed bytes `<=75%` of readable
- compressed token count `<=100%` of readable

### Before/After excerpt (decide)

Readable:
```mu
F decide:(State,Signal,i32)->Decision=i(c(>=,arg2,70),m(arg1){Buy=>m(arg0){Idle=>Step(Long(),EnterLong(arg2));Long=>Step(Long(),Hold());Short=>Step(Idle(),ExitShort());};...},...);
```

Compressed:
```mu
F #44:(#32,#31,i32)->#7=[i (#1 #41 70) [m #40 {#6 [m #39 {#19 #33(#21(),#10(#41))} ...]} ...]];
```
