# muScript v0.1

![CI](https://img.shields.io/badge/ci-fast%20checks-blue)
![Fuzz (manual)](https://img.shields.io/badge/fuzz-manual-orange)

`muc` is the reference µScript v0.1 toolchain in Rust.
It includes:
- Lexer + parser for the EBNF grammar
- Canonical formatter (`mufmt` behavior via `muc fmt`)
- Module loading + name resolution + type/effect checking
- Stack bytecode (`.mub`, header `MUB1`) + VM runtime

## Quickstart

1. Build:

```bash
cargo build
```

2. Canonical format gate:

```bash
cargo run -- fmt --check .
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

- `muc fmt <file|dir> [--check]`
- `muc check <file|dir>`
- `muc run <file.mu|file.mub> [-- args...]`
- `muc build <file.mu> -o out.mub`

Example modules:
- `examples/hello.mu`
- `examples/json.mu`
- `examples/http.mu`

## CI Gates

CI enforces:
- `cargo fmt --all -- --check`
- `cargo test`
- `cargo run -- fmt --check .`

Manual workflow:
- `fuzz` workflow dispatch runs each fuzz target for a short smoke duration
