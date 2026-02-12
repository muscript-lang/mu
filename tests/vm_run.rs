use muc::bytecode::compile;
use muc::parser::parse_str;
use muc::vm::run_bytecode;

#[test]
fn bytecode_runs_main_and_returns_zero() {
    let src = "@x.run{F main:()->i32!{io}={c(print,\"ok\");0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("bytecode should run");
}

#[test]
fn bytecode_requires_main_function() {
    let src = "@x.nom{V n:i32=1;}";
    let program = parse_str(src).expect("program should parse");
    let err = compile(&program).expect_err("missing main should fail");
    assert!(err.to_string().contains("missing `main` function"));
}

#[test]
fn bytecode_runs_bool_match() {
    let src = "@x.match{F main:()->i32=m(t){t=>0;f=>1;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("bytecode should run");
}

#[test]
fn bytecode_runs_bool_match_with_wildcard() {
    let src = "@x.match2{F main:()->i32=m(f){t=>1;_=>0;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("bytecode should run");
}

#[test]
fn bytecode_runs_nullary_ctor_match() {
    let src = "@x.adt{T Opt[A]=None|Some(A);F main:()->i32=m(None()){None()=>0;_=>1;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("bytecode should run");
}

#[test]
fn bytecode_runs_fs_write_and_read_calls() {
    let src = "@x.fs{F main:()->i32!{fs}={c(write,\"/tmp/mu_vm_fs.txt\",\"hello\");c(read,\"/tmp/mu_vm_fs.txt\");0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("bytecode should run");
}

#[test]
fn bytecode_runs_json_parse_and_stringify_calls() {
    let src = "@x.json{F main:()->i32={c(parse,\"{\\\"a\\\":1}\");c(stringify,\"{\\\"a\\\":1}\");0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("bytecode should run");
}

#[test]
fn bytecode_runs_assert_true() {
    let src = "@x.aok{F main:()->i32={a(t,\"ok\");0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("assert true should run");
}

#[test]
fn bytecode_traps_on_assert_false() {
    let src = "@x.abad{F main:()->i32={a(f,\"boom\");0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    let err = run_bytecode(&bc, &[]).expect_err("assert false should trap");
    assert!(err.to_string().contains("assert failure"));
}

#[test]
fn bytecode_runs_require_and_ensure_true() {
    let src = "@x.req{F main:()->i32={^t;_ t;0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("contracts should run");
}

#[test]
fn bytecode_runs_ctor_match_with_field_binding() {
    let src = "@x.adt2{T Opt[A]=None|Some(A);F main:()->i32=m(Some(0)){Some(x)=>x;None()=>1;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("ADT field binding should run");
}

#[test]
fn bytecode_runs_ctor_match_with_wildcard_field() {
    let src = "@x.adt3{T Opt[A]=None|Some(A);F main:()->i32=m(Some(1)){Some(_)=>0;None()=>1;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("ADT wildcard field pattern should run");
}
