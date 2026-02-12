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
