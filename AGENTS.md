Repo context
- This repo implements µScript v0.1 (tagged v0.1.0).
- Authoritative docs: SPEC.md, AGENTS.md, EBNF.md.
- Goal of this task: produce a “v0.2 hardening” PR: freeze semantics + bytecode contract, add fuzzing/property tests, and strengthen determinism/CI.
- DO NOT add new language features beyond SPEC.md.

Non-negotiable constraints (AGENTS.md)
- No feature creep.
- Canonical formatting is mandatory and must remain deterministic.
- Strict effect checking stays strict.
- Deterministic runtime; stable error codes; spans in diagnostics.

Deliverable: ONE PR (can be multiple commits) with:
A) “Semantics freeze” documentation + golden tests
B) Bytecode format freeze documentation + encode/decode golden tests
C) Fuzzing harnesses (cargo-fuzz) for lexer/parser/bytecode decode/VM step
D) CI updates to run fast checks always + fuzz as optional/manual

──────────────────────────────────────────────────────────────────────────────
Part A — Semantics Freeze (docs + tests)
1) Create docs/SEMANTICS.md that precisely defines v0.1 behavior for:
   - canonical formatting invariants (idempotence)
   - evaluation order for call arguments, let binding, if, match
   - numeric behavior: division/mod by zero (trap), integer overflow behavior (define: trap OR wrap; match current implementation and document it)
   - equality semantics for strings/arrays/maps/ADTs (deep structural or not; match current impl)
   - match behavior: exhaustiveness guarantees; what happens at runtime if checker missed (must be “unreachable trap”)
   - contract/assert: when enabled, what error, and whether stripped in release

2) Add tests that “lock” these semantics:
   - tests/semantics/*.mu plus expected output/exitcode
   - include at least:
     a) evaluation order test (side effects prove order)
     b) structural equality test
     c) division by zero trap test
     d) match exhaustiveness compile-time failure test + stable error code

Rules:
- Do not change behavior to match your preference; document and freeze whatever the current implementation does.
- If behavior is currently inconsistent across code paths, make it consistent (bugfix allowed) and then freeze it.

──────────────────────────────────────────────────────────────────────────────
Part B — Bytecode Freeze (docs + golden vectors)
1) Create docs/BYTECODE.md describing the exact .mub format:
   - header magic (e.g., MUB1)
   - version field
   - endianness
   - integer widths for offsets/lengths
   - string encoding
   - function table layout
   - constant pool layout
   - export table
   - checksum or not (if none, document “none”)
   This doc must match the current encoder/decoder implementation.

2) Implement a strict decoder/validator:
   - decode must never panic; return structured errors with stable codes
   - bounds-check all lengths and indices
   - reject unknown opcodes
   - reject truncated sections

3) Add golden bytecode tests:
   - tests/bytecode/*.mub (small)
   - tests ensure decode(encode(module)) roundtrips deterministically
   - add “corrupt bytecode” fixtures that must fail with specific error codes

──────────────────────────────────────────────────────────────────────────────
Part C — Fuzzing (cargo-fuzz)
Add fuzzing via cargo-fuzz (libFuzzer). Follow rust-fuzz/book conventions. :contentReference[oaicite:1]{index=1}

1) Add a fuzz/ directory (cargo fuzz init), with targets:
   - fuzz_lexer: arbitrary bytes → lexer; must not panic/hang
   - fuzz_parser: arbitrary bytes → parse as module; must not panic/hang
   - fuzz_fmt_roundtrip: bytes → (try parse) → fmt → parse; must not panic; if parse succeeds, fmt must be idempotent
   - fuzz_bytecode_decode: bytes → decode .mub; must not panic/hang
   - fuzz_vm_step: if you can construct a minimal valid bytecode blob, fuzz decode+execute with a step limit; must not panic or OOM

2) Add a deterministic step limit / fuel to VM execution for fuzzing targets.
3) Ensure fuzz targets are isolated from real network/filesystem:
   - use a “host” trait; fuzz host must be pure/no-IO, returning deterministic errors.

Note:
- cargo-fuzz requires nightly; document that in docs/FUZZING.md. :contentReference[oaicite:2]{index=2}
- Keep fuzz targets minimal and fast.

──────────────────────────────────────────────────────────────────────────────
Part D — CI hardening
1) Update GitHub Actions:
   - Always run: cargo fmt --check, cargo test
   - Always run: muc fmt --check . (built from the repo)
   - Add an OPTIONAL workflow (manual dispatch) for fuzz:
     - installs nightly + cargo-fuzz
     - runs each fuzz target for a short bounded time (e.g., 30–60s) just to ensure harness works

2) Add badges/notes in README for:
   - how to run fuzz locally
   - how to run the semantics/bytecode golden tests

──────────────────────────────────────────────────────────────────────────────
Definition of Done (must satisfy)
- No new language features beyond SPEC.md.
- docs/SEMANTICS.md and docs/BYTECODE.md exist and match implementation.
- Golden semantics tests pass.
- Bytecode encode/decode roundtrip tests pass.
- Decoder rejects malformed inputs without panics.
- Fuzz targets compile and run locally (document commands).
- CI runs the fast suite on every PR; fuzz is optional/manual.

Output
After you implement, print:
- commands to run locally (fmt/check/test/fuzz)
- summary of new docs/tests
- any behavior you had to standardize (and why it was a bugfix).
