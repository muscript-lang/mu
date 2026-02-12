use muc::parser::{ParseErrorCode, parse_str};

#[test]
fn parse_missing_semicolon_reports_stable_code() {
    let err = parse_str("@m{V x:i32=1}").expect_err("missing semicolon should fail");
    assert_eq!(err.code, ParseErrorCode::ExpectedToken);
    assert_eq!(err.code.as_str(), "E2002");
}

#[test]
fn parse_missing_module_header_reports_stable_code() {
    let err = parse_str("V x:i32=1;").expect_err("missing module should fail");
    assert_eq!(err.code, ParseErrorCode::ExpectedToken);
    assert_eq!(err.code.as_str(), "E2002");
}

#[test]
fn parse_invalid_expression_reports_stable_code() {
    let err = parse_str("@m{V x:i32=;}")
        .expect_err("missing expression in value declaration should fail");
    assert_eq!(err.code, ParseErrorCode::ExpectedExpr);
    assert_eq!(err.code.as_str(), "E2005");
}

#[test]
fn parse_rejects_empty_type_param_list_in_type_decl() {
    let err = parse_str("@m{T Box[]=Box(i32);}").expect_err("empty type param list should fail");
    assert_eq!(err.code, ParseErrorCode::ExpectedIdent);
    assert_eq!(err.code.as_str(), "E2003");
}

#[test]
fn parse_rejects_empty_type_param_list_in_function_decl() {
    let err = parse_str("@m{F id[]:(i32)->i32=arg0;}")
        .expect_err("empty function type param list should fail");
    assert_eq!(err.code, ParseErrorCode::ExpectedIdent);
    assert_eq!(err.code.as_str(), "E2003");
}

#[test]
fn parse_rejects_symref_without_symtab() {
    let err = parse_str("@m{F #0:()->i32=0;}").expect_err("symref without symtab should fail");
    assert_eq!(err.code, ParseErrorCode::MissingSymbolTable);
    assert_eq!(err.code.as_str(), "E2006");
}

#[test]
fn parse_rejects_out_of_range_symref() {
    let err = parse_str("@m{$[x];F #1:()->i32=0;}").expect_err("out of range symref should fail");
    assert_eq!(err.code, ParseErrorCode::SymbolRefOutOfRange);
    assert_eq!(err.code.as_str(), "E2007");
}
