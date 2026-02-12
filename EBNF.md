µScript v0.1 — EBNF Grammar
Notes / conventions
This is syntactic EBNF; semantic rules (type checking, effect ordering, exhaustiveness, canonical formatting) are specified elsewhere.
Whitespace and comments may appear between any tokens unless explicitly forbidden.
All keywords/forms are case-sensitive and must appear exactly as shown.
Terminals are in quotes.
1. Lexical
letter      = "A"…"Z" | "a"…"z" | "_" ;
digit       = "0"…"9" ;
ident       = letter , { letter | digit } ;

int_lit     = "0" | ( "1"…"9" , { digit } ) ;
bool_lit    = "t" | "f" ;

escape      = "\" , ( "\"" | "\" | "n" | "r" | "t" ) ;
str_char    = ? any char except " and \ and newline ? | escape ;
str_lit     = "\"" , { str_char } , "\"" ;

modid       = ident , { "." , ident } ;
Comments (lexical, optional):
Line comment: // … end of line
Block comment: /* … */
2. Program structure
program     = module ;

module      = "@" , modid , "{" , { decl } , "}" ;
3. Declarations
decl        = import_decl
            | export_decl
            | type_decl
            | val_decl
            | fun_decl
            ;

import_decl = ":" , ident , "=" , modid , ";" ;

export_decl = "E" , "[" , [ ident_list ] , "]" , ";" ;

type_decl   = "T" , ident , [ type_params ] , "=" , ctor , { "|" , ctor } , ";" ;

ctor        = ident , [ "(" , [ type_list ] , ")" ] ;

val_decl    = "V" , ident , ":" , type , "=" , expr , ";" ;

fun_decl    = "F" , ident , [ type_params ] , ":" , fun_type , "=" , expr , ";" ;
Lists:
ident_list  = ident , { "," , ident } ;
type_list   = type  , { "," , type  } ;
Type parameters:
type_params = "[" , ident_list , "]" ;
4. Types
type        = prim_type
            | opt_type
            | array_type
            | map_type
            | tuple_type
            | named_type
            | result_sugar
            ;

prim_type   = "b" | "s"
            | "i32" | "i64" | "u32" | "u64"
            | "f32" | "f64"
            | "unit"
            ;

named_type  = ident , [ type_args ] ;

type_args   = "[" , type_list , "]" ;

opt_type    = "?" , type ;

array_type  = type_atom , "[]" ;

map_type    = "{" , type , ":" , type , "}" ;

tuple_type  = "(" , type , "," , type , { "," , type } , ")" ;

fun_type    = "(" , [ type_list ] , ")" , "->" , type , [ effect_set ] ;

result_sugar = type_atom , "!" , type_atom ;
Where type_atom is a non-recursive base used to avoid ambiguity for postfix forms:
type_atom   = prim_type
            | named_type
            | opt_type
            | map_type
            | tuple_type
            | "(" , type , ")"
            ;
4.1 Effect sets
effect_set  = "!{" , effect_atom , { "," , effect_atom } , "}" ;

effect_atom = "io" | "fs" | "net" | "proc" | "rand" | "time" | "st" ;
Semantic constraint (not EBNF): effect atoms must be unique and sorted in canonical order: io, fs, net, proc, rand, time, st.
5. Expressions
expr        = block
            | unit_expr
            | let_expr
            | if_expr
            | match_expr
            | call_expr
            | lambda_expr
            | assert_expr
            | require_expr
            | ensure_expr
            | literal
            | ident
            | ctor_expr
            | paren_expr
            ;
5.1 Core forms
block       = "{" , { expr , ";" } , expr , "}" ;

unit_expr   = "(" , ")" ;

let_expr    = "v" , "(" , ident , [ ":" , type ] , "=" , expr , "," , expr , ")" ;

if_expr     = "i" , "(" , expr , "," , expr , "," , expr , ")" ;

match_expr  = "m" , "(" , expr , ")" , "{" , { match_arm } , "}" ;

match_arm   = pattern , "=>" , expr , ";" ;

call_expr   = "c" , "(" , expr , { "," , expr } , ")" ;

lambda_expr = "l" , "(" , params , ")" , ":" , type , [ effect_set ] , "=" , expr ;

params      = param , { "," , param } ;
param       = ident , ":" , type ;

assert_expr = "a" , "(" , expr , [ "," , expr ] , ")" ;

require_expr = "^" , expr ;
ensure_expr  = "_" , expr ;
5.2 Literals and parentheses
literal     = int_lit | bool_lit | str_lit ;

paren_expr  = "(" , expr , ")" ;
5.3 Constructor expressions
Constructors are names declared in ADTs. Syntax is identical to patterns/calls but is parsed distinctly as an expression form:
ctor_expr   = ident , [ "(" , [ expr_list ] , ")" ] ;

expr_list   = expr , { "," , expr } ;
Disambiguation rule (parser + checker):
Syntactically, ident could be a variable, function, or constructor.
Parse ident("(" ... ")" ) as ctor_expr initially; during name resolution/type checking, determine whether it is:
a constructor application (ADT ctor), or
a normal function call (if ident is a value of function type)
c(...) is always an explicit call and does not rely on this disambiguation.
(Implementers: easiest is to treat ctor_expr as NameApp(name, args) in the AST and resolve later.)
6. Patterns
pattern     = "_" 
            | literal
            | ident_pat
            | ctor_pat
            | tuple_pat
            | paren_pat
            ;

ident_pat   = ident ;

ctor_pat    = ident , [ "(" , [ pat_list ] , ")" ] ;

tuple_pat   = "(" , pattern , "," , pattern , { "," , pattern } , ")" ;

paren_pat   = "(" , pattern , ")" ;

pat_list    = pattern , { "," , pattern } ;
Semantic constraint (not EBNF):
ident_pat binds a name unless it resolves to a nullary constructor; resolution happens during type checking with ADT info.
Implementation notes for Codex (practical)
Prefer a two-phase parser:
Parse into an AST that preserves ambiguous forms (Name, NameApp).
Resolve names using the module’s symbol tables (types/ctors/values) during type checking.
Enforce canonicalization separately:
Build an AST printer that prints tokens exactly according to the canonical rules.

---

v0.2 additions (backward-compatible)

7. Symbol names and module symbol table

symref      = "#" , int_lit ;
symname     = ident | symref ;

symtab_decl = "$" , "[" , [ ident_list ] , "]" , ";" ;

module      = "@" , modid , "{" , [ symtab_decl ] , { decl } , "}" ;

Notes:
- `symref` is valid in positions that accept identifiers in v0.1 syntax.
- If any `symref` is used, a `symtab_decl` must be present in the same module.

8. Declaration updates for symbol names

import_decl = ":" , symname , "=" , modid , ";" ;

export_decl = "E" , "[" , [ symname_list ] , "]" , ";" ;
symname_list = symname , { "," , symname } ;

type_decl   = "T" , symname , [ symname_type_params ] , "=" , ctor , { "|" , ctor } , ";" ;
symname_type_params = "[" , symname_list , "]" ;
ctor        = symname , [ "(" , [ type_list ] , ")" ] ;

val_decl    = "V" , symname , ":" , type , "=" , expr , ";" ;

fun_decl    = "F" , symname , [ symname_type_params ] , ":" , fun_type , "=" , expr , ";" ;

param       = symname , ":" , type ;

named_type  = symname , [ type_args ] ;

9. Effect atoms (long + compressed aliases)

effect_set  = "!{" , effect_atom , { "," , effect_atom } , "}" ;

effect_atom = "io" | "fs" | "net" | "proc" | "rand" | "time" | "st"
            | "I"  | "F"  | "N"   | "P"    | "R"    | "T"    | "S" ;

10. New expression forms

expr        = block
            | unit_expr
            | let_expr
            | if_expr
            | match_expr
            | call_expr
            | sexpr_call
            | lambda_expr
            | bracket_let_expr
            | bracket_if_expr
            | bracket_match_expr
            | bracket_lambda_expr
            | assert_expr
            | require_expr
            | ensure_expr
            | literal
            | symname
            | ctor_expr
            | paren_expr
            ;

sexpr_call  = "(" , expr , expr , { expr } , ")" ;
Disambiguation:
- `()` is `unit_expr`
- `(e)` is `paren_expr`
- `(e e2 ...)` is `sexpr_call`

bracket_let_expr
            = "[" , "v" , symname , expr , expr , "]" ;

bracket_if_expr
            = "[" , "i" , expr , expr , expr , "]" ;

bracket_match_expr
            = "[" , "m" , expr , bracket_match_arm , { bracket_match_arm } , "]" ;

bracket_match_arm
            = "{" , pattern , expr , "}" ;

bracket_lambda_expr
            = "[" , "l" , "(" , params , ")" , ":" , type , [ effect_set ] , expr , "]" ;

11. Pattern updates

ident_pat   = symname ;
ctor_pat    = symname , [ "(" , [ pat_list ] , ")" ] ;
