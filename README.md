# muScript v0.1 (Rust scaffold)

This repository contains an initial Rust scaffold for `muc`, following the project constraints in `AGENTS.md` and the language definitions in `SPEC.md` and `EBNF.md`.

## Quickstart

1. Build:

```bash
cargo build
```

2. Run formatter in check mode (canonical formatting gate):

```bash
cargo run -- fmt --check .
```

3. Format files:

```bash
cargo run -- fmt .
```

4. Parse + typecheck stubs:

```bash
cargo run -- check examples
```

5. Run tests:

```bash
cargo test
```

## CLI

- `muc fmt <file|dir> [--check]`
- `muc check <file|dir>`
- `muc run <file.mu> [-- args...]`
- `muc build <file.mu> -o out.mub`

Current status:

- Lexer/parser/AST/formatter are scaffolded with deterministic output.
- Typechecking, bytecode, VM, and stdlib are intentionally minimal stubs to keep boundaries in place for incremental implementation.
