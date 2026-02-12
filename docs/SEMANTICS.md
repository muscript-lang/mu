# Semantics Freeze (v0.1)

This document freezes the implemented v0.1 behavior in `muc`.
It describes the current implementation, including one v0.2 hardening bugfix:
integer overflow now traps deterministically instead of depending on Rust debug/release overflow behavior.

## Canonical Formatting

- `muc fmt` is canonical and deterministic.
- `muc fmt --check` fails if the source is not already canonical.
- Canonical formatting is idempotent:
  - `fmt(fmt(src)) == fmt(src)`
- Effect sets are canonicalized and checked as strictly ordered/unique: `io,fs,net,proc,rand,time,st`.

## Evaluation Order

Evaluation order is strict and left-to-right.

- Call arguments: evaluated left-to-right before the call.
- `let` (`v`): value expression is evaluated before binding, then body is evaluated.
- `if` (`i`): condition is evaluated first; exactly one branch is evaluated.
- `match` (`m`): scrutinee is evaluated once before arm checks; arms are checked in source order; first matching arm executes.

## Numeric Behavior

Integer operations use signed 64-bit runtime integers.

- Division/modulo by zero traps with `E4003`.
- Integer overflow traps with `E4003`.
  - This includes `+`, `-`, `*`, unary `neg`, and overflow cases in `/` and `%` (for example `i64::MIN / -1`).

## Equality

Runtime equality (`==`, `!=`) is structural (deep) over runtime values.

- Strings: by content.
- Arrays: element-wise.
- Maps: key/value structural equality.
- ADTs: tag equality plus field-wise structural equality.

Typechecker rules remain strict:

- Operands must have compatible types.
- Function values are rejected for equality (`E3004`).

## Match Exhaustiveness and Runtime Fallback

- Typechecker enforces exhaustiveness on booleans and ADTs/`Result` unless a wildcard arm exists.
- Non-exhaustive match is compile-time `E3008`.
- Bytecode lowering inserts an explicit runtime trap when there is no fallback arm:
  - trap code/message: `E4005: invalid match`
  - This is the runtime unreachable guard if static checking is bypassed.

## Contracts and Assert

- `a(cond)` and `a(cond,msg)` compile to runtime assertion checks.
  - Failure traps with `E4001`.
- `r(cond)` and `e(cond)` compile to runtime contract checks.
  - Failure traps with `E4002`.
- Assert/contract checks are always present in current builds; they are not stripped in release.
