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
fn import_of_builtin_core_module_passes_without_workspace_sources() {
    let src = "@a.main{:io=core.io;F main:()->i32!{io}={c(println,\"ok\");0};}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("core.io import should be recognized");
}

#[test]
fn import_of_unknown_builtin_core_module_fails() {
    let src = "@a.main{:x=core.missing;F main:()->i32=0;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("unknown core module should fail");
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

#[test]
fn fs_effect_is_required_for_write_call() {
    let src = "@m.fs{F main:()->i32={c(write,\"/tmp/mu_typecheck.txt\",\"x\");0};}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("missing fs effect should fail");
    assert_eq!(err.code, TypeErrorCode::EffectViolation);
}

#[test]
fn fs_effect_allows_write_call() {
    let src = "@m.fsok{F main:()->i32!{fs}={c(write,\"/tmp/mu_typecheck.txt\",\"x\");0};}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("fs effect should allow write");
}

#[test]
fn json_parse_is_pure() {
    let src = "@m.json{F main:()->i32={c(parse,\"{}\" );0};}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("json parse should be pure");
}

#[test]
fn net_effect_is_required_for_http_get() {
    let src = "@m.net{F main:()->i32={c(get,\"https://example.com\");0};}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("missing net effect should fail");
    assert_eq!(err.code, TypeErrorCode::EffectViolation);
}

#[test]
fn net_effect_allows_http_get() {
    let src = "@m.netok{F main:()->i32!{net}={c(get,\"https://example.com\");0};}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("net effect should allow http get");
}

#[test]
fn proc_effect_is_required_for_proc_run() {
    let src = "@m.proc{F helper:(s[])->i32!s=c(run,\"echo\",arg0);F main:()->i32=0;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("missing proc effect should fail");
    assert_eq!(err.code, TypeErrorCode::EffectViolation);
}

#[test]
fn proc_effect_allows_proc_run() {
    let src = "@m.procok{F helper:(s[])->i32!s!{proc}=c(run,\"echo\",arg0);F main:()->i32=0;}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("proc effect should allow run");
}

#[test]
fn proc_run_rejects_non_array_args_type() {
    let src = "@m.procbad{F main:()->i32!{proc}={c(run,\"echo\",\"oops\");0};}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("run expects array of strings");
    assert_eq!(err.code, TypeErrorCode::TypeMismatch);
}

#[test]
fn return_magic_is_allowed_inside_ensure() {
    let src = "@m.r1{F helper:()->b={_ _r;t};F main:()->i32=0;}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("_r should be available in ensure");
}

#[test]
fn return_magic_is_rejected_outside_ensure() {
    let src = "@m.r2{V x:b=_r;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("_r outside ensure should fail");
    assert_eq!(err.code.as_str(), "E3013");
}

#[test]
fn main_must_have_zero_params() {
    let src = "@m.m1{F main:(i32)->i32=arg0;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("invalid main signature should fail");
    assert_eq!(err.code.as_str(), "E3014");
}

#[test]
fn main_must_return_i32() {
    let src = "@m.m2{F main:()->b=t;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("invalid main return type should fail");
    assert_eq!(err.code.as_str(), "E3014");
}

#[test]
fn numeric_operator_call_typechecks() {
    let src = "@m.op1{F main:()->i32=c(+,1,2);}";
    let program = parse_str(src).expect("program should parse");
    check_program(&program).expect("numeric prelude operator should typecheck");
}

#[test]
fn numeric_operator_rejects_wrong_arg_types() {
    let src = "@m.op2{F main:()->i32=c(+,t,2);}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("wrong arg types should fail");
    assert_eq!(err.code, TypeErrorCode::TypeMismatch);
}

#[test]
fn duplicate_export_name_is_rejected() {
    let src = "@m.ex{E[x,x];V x:i32=0;F main:()->i32=0;}";
    let program = parse_str(src).expect("program should parse");
    let err = check_program(&program).expect_err("duplicate export should fail");
    assert_eq!(err.code, TypeErrorCode::DuplicateSymbol);
}

#[test]
fn duplicate_import_alias_is_rejected() {
    let main_src = "@main.app{:x=dep.one;:x=dep.two;F main:()->i32=0;}";
    let dep_one = "@dep.one{F main:()->i32=0;}";
    let dep_two = "@dep.two{F main:()->i32=0;}";
    let p0 = parse_str(main_src).expect("main parses");
    let p1 = parse_str(dep_one).expect("dep one parses");
    let p2 = parse_str(dep_two).expect("dep two parses");
    let err = check_programs(&[p0, p1, p2]).expect_err("duplicate import alias should fail");
    assert_eq!(err.code, TypeErrorCode::DuplicateSymbol);
}
