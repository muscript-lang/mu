use muc::fmt::format_program;
use muc::parser::parse_str;

#[test]
fn parses_full_module_forms() {
    let src = "@demo.mod{:io=core.io;E[main,valx];T Opt[A]=None|Some(A);V valx:?i32=v(x:i32=1,x);F main:()->i32!{io}=l(msg:s):i32!{io}={^t;_t;c(print,msg);0};}";
    let program = parse_str(src).expect("program should parse");
    assert_eq!(format_program(&program), format!("{src}\n"));
}

#[test]
fn parses_match_patterns() {
    let src = "@p.m{F main:()->i32=m(Some(1)){_=>0;Some(x)=>x;(x,y)=>x;};}";
    let program = parse_str(src).expect("program should parse");
    assert_eq!(format_program(&program), format!("{src}\n"));
}

#[test]
fn parses_type_variants() {
    let src = "@tm.m{T R[A,B]=R(A,B);V a:((i32,s)->i32)!s=foo;V b:{s:?i32[]}=bar;}";
    let program = parse_str(src).expect("program should parse");
    let formatted = format_program(&program);
    assert!(formatted.starts_with("@tm.m{"));
}

#[test]
fn formatter_keeps_ensure_parseable() {
    let src = "@m.e{F main:()->i32={_ t;0};}";
    let program = parse_str(src).expect("program should parse");
    let formatted = format_program(&program);
    assert!(formatted.contains("_ t"), "ensure should include separator");
    parse_str(formatted.trim_end()).expect("formatted ensure should reparse");
}

#[test]
fn parses_symbolic_prelude_operator_names() {
    let src = "@m.op{F main:()->i32=c(+,1,2);}";
    let program = parse_str(src).expect("program should parse");
    let formatted = format_program(&program);
    assert!(formatted.contains("c(+,1,2)"));
}

#[test]
fn allows_t_and_f_as_identifiers_outside_literal_positions() {
    let src = "@t.m{F f:()->i32=0;F main:()->i32=c(f);}";
    let program = parse_str(src).expect("program should parse");
    let formatted = format_program(&program);
    assert!(formatted.starts_with("@t.m{"));
}

#[test]
fn preserves_array_suffix_in_type_arguments() {
    let src = "@m.arr{T Json=Null|Arr(Json[]);}";
    let program = parse_str(src).expect("program should parse");
    let formatted = format_program(&program);
    assert!(
        formatted.contains("Arr(Json[])"),
        "array suffix should be preserved in canonical formatting: {formatted}"
    );
}

#[test]
fn allows_identifiers_named_like_core_forms() {
    let src = "@m.forms{F v:()->i32=0;F i:()->i32=0;F m:()->i32=0;F c:()->i32=0;F l:()->i32=0;F a:()->i32=0;F main:()->i32=c(v);}";
    let program = parse_str(src).expect("program should parse");
    let formatted = format_program(&program);
    assert!(
        formatted.contains("F main:()->i32=c(v);"),
        "formatted output should preserve identifier call through explicit c(...): {formatted}"
    );
}
