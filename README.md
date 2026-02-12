# muScript v0.2

![CI](https://img.shields.io/badge/ci-fast%20checks-blue)
![Fuzz (manual)](https://img.shields.io/badge/fuzz-manual-orange)

`muc` is the reference µScript v0.2 toolchain in Rust.
It includes:
- Lexer + parser for the EBNF grammar
- Canonical formatter (`mufmt` behavior via `muc fmt`) with readable/compressed modes
- Module loading + name resolution + type/effect checking
- Stack bytecode (`.mub`, header `MUB1`) + VM runtime

## Why v0.2 (Agent + Cost Focus)

µScript is designed by and for agents:
- Small, regular grammar and canonical formatting reduce parsing ambiguity.
- Fewer equivalent spellings means less prompt context spent on style negotiation.
- Deterministic output (`fmt`) makes agent loops stable (plan -> edit -> check -> diff).
- Explicit effects and static checks reduce trial-and-error execution.

v0.2 adds a compressed canonical form optimized for token economy:
- Module symbol table + numeric symbol refs (`$[...]`, `#n`) reduce repeated-name overhead.
- S-expression calls and bracket special forms reduce punctuation noise.
- Optional single-letter effect atoms in compressed mode (`I,F,N,P,R,T,S`).

Practical impact:
- Fewer source tokens usually means lower LLM prompt/completion cost for code-gen and review loops.
- Savings are strongest when identifiers repeat frequently across modules/functions.
- The repo includes token-efficiency regression tests (`tests/token_efficiency.rs`).

Recent app benchmark (`apps/signal_reactor`, measured by `tests/signal_reactor_token_economy.rs`):
- readable bytes: `8266`
- compressed bytes: `5838` (`70.62%`)
- readable lexer-token count: `2753`
- compressed lexer-token count: `2520` (`91.54%`)

## Quickstart

1. Build:

```bash
cargo build
```

2. Canonical format gates:

```bash
cargo run -- fmt --check examples
cargo run -- fmt --check tests/fixtures/parser
cargo run -- fmt --mode=compressed --check tests/fixtures/compressed
```

3. Type + effect check:

```bash
cargo run -- check examples
```

4. Run script:

```bash
cargo run -- run examples/hello.mu
```

You can also run the JSON roundtrip and HTTP examples:

```bash
cargo run -- run examples/json.mu
cargo run -- run examples/http.mu
```

5. Build bytecode:

```bash
cargo run -- build examples/hello.mu -o hello.mub
```

6. Run bytecode directly:

```bash
cargo run -- run hello.mub
```

You can also build/run JSON and HTTP bytecode:

```bash
cargo run -- build examples/json.mu -o json.mub
cargo run -- run json.mub
cargo run -- build examples/http.mu -o http.mub
cargo run -- run http.mub
```

7. Run tests:

```bash
cargo test
```

8. Run semantics/bytecode golden suites:

```bash
cargo test semantics_goldens
cargo test bytecode_golden
```

9. Run fuzz locally (nightly + cargo-fuzz):

```bash
cargo +nightly fuzz run fuzz_lexer -- -max_total_time=30
cargo +nightly fuzz run fuzz_parser -- -max_total_time=30
cargo +nightly fuzz run fuzz_fmt_roundtrip -- -max_total_time=30
cargo +nightly fuzz run fuzz_bytecode_decode -- -max_total_time=30
cargo +nightly fuzz run fuzz_vm_step -- -max_total_time=30
```

## CLI

- `muc fmt <file|dir> [--mode=readable|compressed] [--check]`
- `muc check <file|dir>`
- `muc run <file.mu|file.mub> [-- args...]`
- `muc build <file.mu> -o out.mub`

Example modules:
- `examples/hello.mu`
- `examples/json.mu`
- `examples/http.mu`

### Compressed Canonical Conversion

Convert a project/dir to compressed canonical form:

```bash
cargo run -- fmt --mode=compressed <file-or-dir>
```

Verify compressed canonical form:

```bash
cargo run -- fmt --mode=compressed --check <file-or-dir>
```

Convert back to readable canonical form:

```bash
cargo run -- fmt --mode=readable <file-or-dir>
```

Readable and compressed forms are syntax variants of the same semantics.

## CI Gates

CI enforces:
- `cargo fmt --all -- --check`
- `cargo test`
- readable fixture format checks
- compressed fixture format checks

Manual workflow:
- `fuzz` workflow dispatch runs each fuzz target for a short smoke duration
