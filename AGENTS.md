AGENTS.md — µScript v0.1 Implementation Rules
This document defines strict rules for Codex (and contributors) when implementing µScript.
The goal is spec compliance, minimalism, determinism, and zero feature creep.
If a decision is ambiguous, choose the simplest possible implementation that satisfies SPEC.md.
1. Hard Constraints
1.1 DO NOT extend the language
You must NOT:
Add syntax not defined in SPEC.md
Add operator sugar
Add implicit conversions
Add exceptions
Add macros
Add additional effects
Add additional stdlib modules
Add new keywords
Add hidden runtime behavior
If something is not explicitly in SPEC.md, it does not exist.
1.2 Canonical formatting is mandatory
The language is defined by its canonical form.
Rules:
The AST pretty printer (mufmt) must produce exactly one representation.
muc fmt --check must fail if input differs from canonical form.
CI must fail if formatting changes output.
The compiler must parse both formatted and unformatted input,
but only canonical form is accepted in CI.
1.3 Effect system is strict
Pure functions cannot call effectful functions.
Effect sets must be sorted canonically.
Empty effect set must be omitted.
No implicit effect propagation.
No global mutable state unless gated behind !{st}.
If effect inference becomes complex, simplify — never weaken the checker.
2. Implementation Language
Use Rust.
Required crates:
clap (CLI)
thiserror (error handling)
anyhow (CLI surface errors)
serde ONLY for JSON in stdlib
No heavy parser generators unless minimal (e.g., chumsky or hand-written parser preferred)
Avoid:
Macros that obscure logic
Async runtime unless strictly required for HTTP
Complex dependency trees
Keep compile time fast.
3. Architecture
3.1 Compiler pipeline
Must follow this exact structure:
Lexer → Parser → AST
           ↓
     Canonicalizer
           ↓
    Type + Effect Checker
           ↓
        Lowering
           ↓
        Bytecode
           ↓
           VM
Each stage must be separate module.
No monolithic implementation.
3.2 AST Design Principles
AST must directly reflect the spec forms.
Do not desugar into implicit constructs early.
Preserve spans for error reporting.
Match exhaustiveness checking must operate on ADT definitions.
3.3 Type System
Implement:
HM-style inference for local v(...)
Explicit types for top-level F and V
No rank-n polymorphism
No higher-kinded types
No trait system
Unification must be simple and deterministic.
3.4 Effects
Represent effect sets as:
struct EffectSet {
    io: bool,
    fs: bool,
    net: bool,
    proc: bool,
    rand: bool,
    time: bool,
    st: bool,
}
Canonical ordering must be enforced during formatting and checking.
Effect union is boolean OR.
4. VM Requirements
4.1 Design
Stack-based VM.
Deterministic execution.
No JIT.
No hidden host side effects.
4.2 Bytecode format
Stable binary header: MUB1
Versioned format.
No unsafe memory tricks.
All indices bounds-checked.
4.3 Runtime Errors
Allowed traps only:
Assertion failure
Contract failure
Out-of-bounds
Division by zero
Invalid match (should not happen if checker works)
All other failures must use Res[T,E].
5. Standard Library Constraints
Stdlib modules allowed:
core.prelude
core.io
core.fs
core.json
core.proc
core.http
Do not add others.
Each stdlib function must:
Declare correct effect set.
Be implemented in Rust host bindings.
Be small and deterministic.
6. CLI Requirements
Binary: muc
Commands:
fmt
check
run
build
No additional commands.
Help output must be minimal.
7. Testing Requirements
You must implement:
7.1 Parser tests
Golden tests for canonical formatting.
7.2 Type tests
Effect violation cases must fail.
Non-exhaustive match must fail.
Wrong type application must fail.
7.3 Runtime tests
Hello world
JSON parse/stringify
FS read/write (mockable)
HTTP get (optional integration test)
7.4 Property tests (optional but encouraged)
parse(fmt(x)) == fmt(x)
8. CI Requirements
GitHub Actions must:
Build project
Run tests
Run muc fmt --check .
Fail on warnings
9. Error Reporting Rules
Errors must:
Include file
Include line/column
Include stable error code
Example:
E0301: effect violation
  at examples/test.mu:12:5
Do not produce vague panic messages.
10. Forbidden Shortcuts
You must NOT:
Implement as a thin wrapper over an existing scripting language.
Translate µScript to Python/JS and run it.
Skip type checking.
Make effect system advisory only.
Skip exhaustiveness checking.
Ignore canonical formatting.
11. Definition of Done
The implementation is complete when:
All features in SPEC.md are implemented.
No undocumented features exist.
All tests pass.
CI passes.
Example scripts run.
Canonical formatting works.
muc build produces stable .mub.
No additional features are allowed in v0.1.
12. Development Philosophy
µScript is:
Small
Strict
Deterministic
LLM-friendly
Canonical
If a choice increases complexity or expressive power, reject it.
Prefer:
Simplicity over flexibility
Determinism over convenience
Explicitness over magic
Static guarantees over runtime checks
If you want, next we can:
Design a v0.2 roadmap (carefully controlled),
Or design a minimal SSA IR to future-proof the compiler,
Or define a formal grammar (EBNF) for absolute parsing clarity.