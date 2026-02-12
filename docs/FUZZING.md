# Fuzzing

`cargo-fuzz` is configured under `fuzz/` with libFuzzer targets.

## Requirements

- Rust nightly toolchain (cargo-fuzz requirement)
- `cargo install cargo-fuzz`

## Targets

- `fuzz_lexer`
- `fuzz_parser`
- `fuzz_fmt_roundtrip`
- `fuzz_bytecode_decode`
- `fuzz_vm_step`

## Run Locally

```bash
cargo +nightly fuzz run fuzz_lexer -- -max_total_time=30
cargo +nightly fuzz run fuzz_parser -- -max_total_time=30
cargo +nightly fuzz run fuzz_fmt_roundtrip -- -max_total_time=30
cargo +nightly fuzz run fuzz_bytecode_decode -- -max_total_time=30
cargo +nightly fuzz run fuzz_vm_step -- -max_total_time=30
```

Notes:

- `fuzz_vm_step` uses VM fuel limits to prevent hangs.
- Fuzz targets use `vm::FuzzHost`, which is deterministic and disables filesystem/network/process side effects.
