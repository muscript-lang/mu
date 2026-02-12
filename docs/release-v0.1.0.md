# µScript v0.1.0 Release Notes

## Summary

`muc v0.1.0` delivers a production-ready µScript toolchain with:
- EBNF-compliant lexer/parser and stable diagnostics (codes + spans)
- Canonical formatter (`muc fmt`) with enforced `--check` gate
- Hermetic module loading and import validation
- Type checking with strict effect checking
- Bytecode compiler (`.mub`, `MUB1`) and stack VM runtime
- Effect-gated stdlib host calls for `core.io`, `core.fs`, `core.json`, `core.proc`, `core.http`

## CLI

- `muc fmt <file|dir> [--check]`
- `muc check <file|dir>`
- `muc run <file.mu|file.mub> [-- args...]`
- `muc build <file.mu> -o out.mub`

## Examples

- `examples/hello.mu`
- `examples/json.mu`
- `examples/http.mu`

All examples are validated through:
- source `check` + `run`
- bytecode `build` + `.mub run`

## Quality Gates

Release gates:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- `cargo run -- fmt --check .`
- `cargo run -- check examples`

## Tag

- `v0.1.0`
