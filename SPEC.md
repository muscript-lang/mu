# µScript v0.1 --- Specification

µScript (mu-script) v0.1 — Canonical, token-min, LLM-first scripting language
0. Purpose
µScript is a small, deterministic, hermetic scripting language designed for machine generation (LLMs/agents) and reliable compilation:
Canonical surface syntax: one formatting, one AST serialization.
Small vocabulary: most structure uses single-character forms.
Explicit effects: no hidden IO/state; every effect is declared and checked.
Strong static types with inference (optional at call sites).
Scripting: fast interpret/compile-run, simple module system, stdlib for files/process/json/http (all gated by effects).
Target implementations:
Reference interpreter (bytecode VM).
Optional ahead-of-time compiler to a small VM bytecode format.
1. Source files and modules
File extension: .mu
A file contains exactly one module:
@<modid>{ <decl>* }
modid is a dotted identifier: foo.bar.baz
Each module compiles to a unit with:
exported decls = names listed in module export table (see §1.3)
private decls otherwise
1.1 Imports (hermetic)
Import binds a module id to an alias:
:<alias>=<modid>;
Example:
:io=core.io;
:js=core.json;
1.2 Exports
Exports are declared once per module:
E[ name1,name2,... ];
If no E[...] is present, export nothing by default (strict).
1.3 Entry point convention
A script module may define:
F main:()->i32!io= ...
The runner executes main() if present. Exit code is returned i32.
2. Lexical structure
2.1 Characters / encoding
UTF-8
Identifiers restricted to ASCII for v0.1 to reduce ambiguity and tokenizer variance:
Ident: [A-Za-z_][A-Za-z0-9_]*
2.2 Whitespace
Whitespace is allowed between tokens but never significant.
Canonical form: formatter removes all unnecessary whitespace.
Newlines are treated as whitespace.
2.3 Comments
Line comment: // ... to end of line
Block comment: /* ... */ (nesting not required)
2.4 Literals
Integers: 0|[1-9][0-9]* (decimal only v0.1)
Booleans: t and f
Strings: "..." with escapes:
\" \\ \n \r \t
Unicode escapes not required v0.1
3. Types
3.1 Primitive type names
b (bool), s (string)
i32 i64 u32 u64
f32 f64
unit (only value is ())
3.2 Type constructors
Optional: ?T
Array: T[]
Map: {K:V}
Tuple: (T,T,...) (1-tuple not allowed; use bare T)
Function: (A,B,...)->R!effset
Result: T!E (sum type sugar, see §3.4)
User ADTs: T Name = ...;
3.3 Effects
An effect set is a ! followed by a sorted concatenation of effect atoms:
!io console
!fs filesystem
!net network
!proc processes
!st mutable state / refs
!rand randomness
!time wall clock
!none is implicit (no suffix means no effects)
Canonical ordering (must be enforced by formatter and checker):
io,fs,net,proc,rand,time,st
Example:
()->i32!iofs is invalid; must be ()->i32!io!fs? No: v0.1 uses a single ! + concatenation:
()->i32!iofsnet with canonical atom boundaries is ambiguous.
So v0.1 uses !{...} to keep token-min but unambiguous:
Effect set syntax:
none: omitted
non-empty: !{io,fs}
So:
()->i32 = pure
()->i32!{io} = may print/read stdin
()->i32!{fs,io} is invalid (must be !{io,fs})
3.4 Result type
T!E is type sugar for the builtin ADT:
T Res[T,E]=Ok(T)|Er(E);
(Internally, all Results are that ADT.)
4. Declarations
All decls end with ;.
4.1 Type declaration
Sum/variant type (ADT):
T <Name><TypeParams?> = <Ctor> ('|' <Ctor>)* ;
<Ctor> := <CtorName> | <CtorName>(<TypeList?>)
Type params:
<Name>[A,B,...]
Example:
T Opt[A]=None|Some(A);
T Pair[A,B]=Pair(A,B);
4.2 Value declaration
V <name>:<type>=<expr>;
4.3 Function declaration
F <name><TypeParams?>:<ftype>=<expr>;
Where <ftype> is a function type (args)->ret with optional effect set on return:
(i32,s)->i32!{io}
Note: Functions are values; F is just a top-level convenience that binds a name.
5. Expressions (core forms)
Everything is an expression; semicolons only separate expressions inside blocks.
5.1 Block
{ <expr;>* <expr> }
Block value is last expression.
5.2 Unit literal
() is a value of type unit.
5.3 Let binding
v(<name>:<type?>=<expr>,<body>)
<type?> may be omitted for inference:
v(x=1, ...)
Scope is <body> only.
5.4 If
i(<cond>,<then>,<else>)
5.5 Match
m(<expr>){ <pat>=> <expr>; ... }
Match must be exhaustive for ADTs and booleans; compiler must reject non-exhaustive matches unless a wildcard _ is present.
5.6 Call
c(<fn>,<arg1>,<arg2>,...)
5.7 Lambda
l(<params>):<retType><eff?>=<expr>
<params> is comma-separated name:type pairs, types required in v0.1 for lambdas
<eff?> is optional !{...} after <retType>
Example:
l(x:i32):i32= c(+,x,1)
5.8 Field access and indexing (stdlib-defined, but canonical)
To keep core tiny, field/index are just calls to prelude functions:
get(a,i) for arrays
put(a,i,v) for arrays (requires !{st})
map_get(m,k) etc.
(There is no . or [] syntax in v0.1.)
5.9 Operators
No operator syntax. All arithmetic/comparison are prelude functions:
+ - * / % == != < <= > >= and or not
So c(+,a,b) etc.
6. Patterns
Patterns in m(...):
Wildcard: _
Literal: 0, t, f, "str"
Bind: <name>
Constructor:
Ctor
Ctor(p1,p2,...)
Tuple: (p1,p2,...)
No guards in v0.1 (keep core minimal).
7. Type inference and checking
7.1 Inference
Hindley–Milner-style inference for local v(x=...) and for call sites.
Top-level V and F require explicit types in v0.1 (simplifies compiler).
7.2 Unification rules (key points)
No implicit numeric widening. Prelude provides explicit conversions:
i32_to_i64, etc.
?T is distinct from T. No null.
Result T!E is the builtin Res[T,E].
7.3 Effect checking
Each function type includes an effect set (possibly empty).
The checker computes the effect of an expression as union of:
effects of called functions
intrinsic effect of certain stdlib calls (see §10)
Calling an effectful function from a less-effectful context is a type error:
Pure function cannot call !{io}.
main may be effectful.
8. Contracts (built-in, token-min)
Contracts are expressions that type-checker treats specially.
8.1 Require / Ensure
Inside any block:
Require: ^<bool-expr> (precondition)
Ensure: _<bool-expr> (postcondition)
_r is a magic identifier bound to the returned value of the enclosing function body (only valid in ensures)
Contracts:
Are checked at runtime in debug mode.
May be compiled out in release mode (flag).
8.2 Assert
a(<bool>,<msg?>)
If msg omitted, use "assert".
9. Runtime model (scripting-first)
9.1 Values
Immediate: ints, bool, unit
Heap: strings, arrays, maps, ADT instances, closures
9.2 Equality
== for primitives is structural.
For strings/arrays/maps/ADTs: structural deep equality in v0.1 (may be expensive, acceptable for scripts).
9.3 Errors
No exceptions.
Runtime traps only for:
contract/assert failure
out-of-bounds in get
division by zero
Everything else modeled as Res[T,E].
10. Standard library (minimal, effect-gated)
All stdlib lives under core.*.
10.1 Prelude core.prelude (pure)
bool ops: and,or,not
compare: ==,!=,<,<=,>,>=
numeric: +,-,*,/,%,neg
string: str_cat, len
constructors for Result:
Ok(x) / Er(e) are ctors of Res[T,E]
10.2 IO core.io (!{io})
print(s):unit!{io}
println(s):unit!{io}
readln():s!{io}
10.3 FS core.fs (!{fs})
read(path:s):Res[s,s]!{fs}
write(path:s, data:s):Res[unit,s]!{fs}
10.4 JSON core.json (pure)
parse(s):Res[Json,s]
stringify(j):s
T Json = Null|Bool(b)|Num(f64)|Str(s)|Arr(Json[])|Obj({s:Json});
10.5 Process core.proc (!{proc})
run(cmd:s, args:s[]):Res[i32,s]!{proc}
10.6 Net core.http (!{net})
get(url:s):Res[s,s]!{net}
(Enough for scripting; keep small.)
11. Canonical formatting (part of the language)
A program is considered well-formed only if it equals its canonical pretty-print (mufmt) output. (Implement mufmt in repo and use it in CI.)
Rules:
No unnecessary whitespace
Commas have no surrounding spaces
Keywords/forms are exactly as specified: @ : E T F V v i m c l a ^ _
Effect sets always printed as:
omitted for empty
!{io,fs,...} with canonical ordering
Type param lists: Name[A,B] no spaces
Blocks: {...} no spaces
12. Bytecode VM (recommended implementation)
12.1 Compilation pipeline
Parse → AST
Canonicalize (rename locals to stable de Bruijn-like IDs OPTIONAL; not required)
Type/effect check
Lower to bytecode
Execute in VM
12.2 VM instructions (minimal set)
Stack-based:
PUSH_INT, PUSH_BOOL, PUSH_STR, PUSH_UNIT
LOAD_LOCAL, STORE_LOCAL
CALL <fnid> <argc>
RET
JMP, JMP_IF_FALSE
MATCH_TAG <nCases> ... (or lower match to chained tests)
MK_CLOSURE <fnid> <nfree>
LOAD_FREE
MK_ADT <tag> <arity>
GET_ADT_FIELD <idx>
Host calls for stdlib, tagged by effect.
Define a stable .mub bytecode format:
header MUB1
constant pool (strings, ints optional)
function table (name, arity, locals, bytecode)
export table
13. CLI tools (deliverables)
13.1 muc (compiler / runner)
Commands:
muc fmt <file|dir>: rewrite to canonical form; fail if changes in --check mode
muc check <file|dir>: parse + type/effect check
muc run <file.mu> [--] [args...]: run module main
muc build <file.mu> -o out.mub: compile to bytecode
13.2 muvm (optional)
muvm run out.mub [args...]
14. Repository plan (what Codex should create)
Suggested repo:
mu-script/
  README.md
  SPEC.md                # this spec
  AGENTS.md              # coding guidelines for Codex
  crates/ (if Rust) or src/ (if TS/Go)
  examples/
    hello.mu
    json.mu
    http.mu
  tests/
    parser/
    typecheck/
    runtime/
  .github/workflows/ci.yml
Implementation language recommendation: Rust (fast, good for compiler/VM, easy CLI).
CI:
run muc fmt --check
run unit tests
run examples
Codex can work directly in your repo via CLI/IDE and can be triggered in GitHub workflows/reviews if you enable it.
15. Acceptance tests (must pass)
Provide golden tests:
Parser round-trip: parse(fmt(ast)) stable
Type/effect:
pure fn calling core.io.print must fail
Runtime:
examples/hello.mu prints “hi”
examples/json.mu parses + stringifies
Match exhaustiveness errors detected
Canonical formatting is enforced
