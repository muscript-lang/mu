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
