use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_file(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("muc_diag_{name}_{nanos}.mu"))
}

#[test]
fn parse_errors_include_file_line_col_and_code() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let path = unique_temp_file("bad_parse");
    fs::write(&path, "@m{V x:i32=;}").expect("should write fixture");

    let output = Command::new(exe)
        .args(["check", path.to_str().expect("utf8 path")])
        .output()
        .expect("binary should run");

    assert!(!output.status.success(), "check should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E2005"),
        "should include stable parse code, got: {stderr}"
    );
    assert!(
        stderr.contains(":1:11:") || stderr.contains(":1:12:"),
        "should include file:line:col, got: {stderr}"
    );

    let _ = fs::remove_file(path);
}

#[test]
fn type_errors_include_file_line_col_and_code() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let path = unique_temp_file("bad_type");
    fs::write(&path, "@m{F main:()->i32={c(print,\"x\");0};}").expect("should write fixture");

    let output = Command::new(exe)
        .args(["check", path.to_str().expect("utf8 path")])
        .output()
        .expect("binary should run");

    assert!(!output.status.success(), "check should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E3007"),
        "should include stable type code, got: {stderr}"
    );
    assert!(
        stderr.contains(":1:"),
        "should include file:line:col, got: {stderr}"
    );

    let _ = fs::remove_file(path);
}

#[test]
fn runtime_errors_include_stable_code() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let path = unique_temp_file("bad_runtime");
    fs::write(&path, "@m{F main:()->i32=c(/,1,0);}").expect("should write fixture");

    let output = Command::new(exe)
        .args(["run", path.to_str().expect("utf8 path")])
        .output()
        .expect("binary should run");

    assert!(!output.status.success(), "run should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E4003"),
        "should include stable runtime code, got: {stderr}"
    );

    let _ = fs::remove_file(path);
}
