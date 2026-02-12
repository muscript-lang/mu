# muScript v0.1

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

You can also run the JSON roundtrip example:

```bash
cargo run -- run examples/json.mu
```

5. Build bytecode:

```bash
cargo run -- build examples/hello.mu -o hello.mub
```

6. Run bytecode directly:

```bash
cargo run -- run hello.mub
```

7. Run tests:

```bash
cargo test
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
