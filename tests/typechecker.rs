use muc::parser::parse_str;
use muc::typecheck::{TypeErrorCode, check_program, check_programs};

#[test]
fn pure_function_cannot_call_io() {
    let src = "@m.p{F main:()->i32={c(print,\"x\");0};}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("effect violation should fail");
    assert_eq!(err.code, TypeErrorCode::EffectViolation);
    assert_eq!(err.code.as_str(), "E3007");
}

#[test]
fn effectful_function_can_call_io() {
    let src = "@m.ok{F main:()->i32!{io}={c(print,\"x\");0};}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("effectful function should pass");
}

#[test]
fn non_exhaustive_bool_match_fails() {
    let src = "@m.ne{V x:i32=m(t){t=>1;};}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("non exhaustive match should fail");
    assert_eq!(err.code, TypeErrorCode::NonExhaustiveMatch);
    assert_eq!(err.code.as_str(), "E3008");
}

#[test]
fn exhaustive_bool_match_passes() {
    let src = "@m.ex{V x:i32=m(t){t=>1;f=>0;};}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("exhaustive match should pass");
}

#[test]
fn import_of_unknown_module_fails() {
    let src = "@a.main{:x=missing.mod;V n:i32=1;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("unknown import should fail");
    assert_eq!(err.code, TypeErrorCode::UnknownModule);
    assert_eq!(err.code.as_str(), "E3002");
}

#[test]
fn imports_validate_against_loaded_modules() {
    let main_src = "@main.app{:x=dep.mod;V n:i32=1;}";
    let dep_src = "@dep.mod{E[v];V v:i32=1;}";
    let main = parse_str(main_src).expect("main parses");
    let dep = parse_str(dep_src).expect("dep parses");
    check_programs(&[main, dep]).expect("known import should pass");
}

#[test]
fn unsorted_effect_set_is_rejected() {
    let src = "@m.fx{F main:()->i32!{fs,io}=0;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("unsorted effect set should fail");
    assert_eq!(err.code.as_str(), "E3012");
}

#[test]
fn duplicate_effect_atom_is_rejected() {
    let src = "@m.fx2{F main:()->i32!{io,io}=0;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("duplicate effect atom should fail");
    assert_eq!(err.code.as_str(), "E3012");
}
