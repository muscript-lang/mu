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
    assert!(err.to_string().contains("E4001"));
}

#[test]
fn bytecode_runs_require_and_ensure_true() {
    let src = "@x.req{F main:()->i32={^t;_ t;0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("contracts should run");
}

#[test]
fn bytecode_traps_on_contract_failure() {
    let src = "@x.reqbad{F main:()->i32={^f;0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    let err = run_bytecode(&bc, &[]).expect_err("contract false should trap");
    assert!(err.to_string().contains("E4002"));
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

#[test]
fn bytecode_runs_user_defined_function_call() {
    let src = "@x.fn{F id:(i32)->i32=arg0;F main:()->i32=c(id,0);}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("user function call should run");
}

#[test]
fn bytecode_runs_non_capturing_lambda_call() {
    let src = "@x.l1{F main:()->i32=v(fn1=l(x:i32):i32=x,c(fn1,0));}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("lambda call should run");
}

#[test]
fn bytecode_runs_capturing_lambda_call() {
    let src = "@x.l2{F main:()->i32=v(y:i32=0,v(fn1=l(x:i32):i32=y,c(fn1,1)));}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("capturing lambda call should run");
}

#[test]
fn bytecode_runs_name_pattern_as_nullary_ctor() {
    let src = "@x.p1{T Opt[A]=None|Some(A);F main:()->i32=m(None()){None=>0;_=>1;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("nullary ctor name pattern should run");
}

#[test]
fn bytecode_runs_name_pattern_as_binding() {
    let src = "@x.p2{F main:()->i32=m(0){x=>x;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("binding name pattern should run");
}

#[test]
fn bytecode_runs_top_level_value_reference() {
    let src = "@x.v1{V x:i32=0;F main:()->i32=x;}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("top-level V should be loadable");
}

#[test]
fn bytecode_runs_numeric_operator_calls() {
    let src = "@x.op1{F main:()->i32=c(-,c(+,1,2),3);}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("numeric operators should run");
}

#[test]
fn bytecode_runs_boolean_operator_calls() {
    let src = "@x.op2{F main:()->i32={a(c(and,t,c(not,f)),\"bool\");0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("boolean operators should run");
}

#[test]
fn bytecode_traps_on_invalid_match() {
    let src = "@x.badm{F main:()->i32=m(t){f=>0;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    let err = run_bytecode(&bc, &[]).expect_err("invalid match should trap");
    assert!(err.to_string().contains("E4005"));
}

#[test]
fn bytecode_runs_neg_builtin() {
    let src = "@x.neg{F main:()->i32={c(neg,1);0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("neg builtin should run");
}

#[test]
fn bytecode_runs_string_helpers() {
    let src = "@x.str{F main:()->i32={c(str_cat,\"a\",\"b\");c(len,\"abc\");0};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("str_cat and len builtins should run");
}

#[test]
fn bytecode_json_parse_returns_json_adt_for_matching() {
    let src = "@x.json2{T Res[A,B]=Ok(A)|Er(B);T Json=Null|Bool(b)|Num(f64)|Str(s)|Arr(Json[])|Obj({s:Json});F main:()->i32=m(c(parse,\"{\\\"a\\\":1}\")){Ok(j)=>m(j){Obj(_)=>0;_=>1;};Er(_)=>1;};}";
    let program = parse_str(src).expect("program should parse");
    let bc = compile(&program).expect("program should lower to bytecode");
    run_bytecode(&bc, &[]).expect("parsed JSON object should match Obj constructor");
}
