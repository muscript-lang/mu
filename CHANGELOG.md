# Changelog

## v0.2.0

Date: 2026-02-12

Summary:
- Added backward-compatible compressed canonical surface syntax for token savings.
- No runtime or semantic feature additions.

Highlights:
- Optional module symbol table directive: `$[...]`.
- Symbol references: `#<int>` with deterministic resolution against module symtab.
- Formatter modes: `muc fmt --mode=readable|compressed` (default `readable`).
- Added s-expression call form: `(fn arg1 arg2 ...)`.
- Added bracket special forms:
  - `[v name expr body]`
  - `[i cond then else]`
  - `[m expr {pat expr} ...]`
  - `[l (x:T,...) :R !{...}? body]`
- Added compressed effect atom aliases in parser/formatter support:
  - `I,F,N,P,R,T,S` for `io,fs,net,proc,rand,time,st`.

Canonical compressed mode:
- Emits `$[...]` once at module start.
- Rewrites eligible names to `#n`.
- Uses s-expr calls and bracket special forms.
- Uses compressed effect atoms in canonical effect order.
- Deterministic symbol table ordering: sorted unique symbols in lexicographic order.

Compatibility:
- v0.1 sources remain valid.
- Semantics preserved between readable and compressed forms.
