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
