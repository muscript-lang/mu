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
