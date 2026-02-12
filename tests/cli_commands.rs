use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_file(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("muc_{name}_{nanos}"))
}

fn unique_temp_mub_file(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("muc_{name}_{nanos}.mub"))
}

#[test]
fn fmt_check_examples_succeeds() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let output = Command::new(exe)
        .args(["fmt", "--check", "examples"])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "fmt --check should succeed for examples: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_examples_succeeds() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let output = Command::new(exe)
        .args(["check", "examples"])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "check should succeed for examples: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn build_outputs_mub_magic() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let out = unique_temp_file("hello.mub");

    let output = Command::new(exe)
        .args([
            "build",
            "examples/hello.mu",
            "-o",
            out.to_str().expect("temp path should be valid utf8"),
        ])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "build should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let bytes = fs::read(&out).expect("build output should be readable");
    assert!(bytes.starts_with(b"MUB1"), "bytecode should start with MUB1");
    let _ = fs::remove_file(out);
}

#[test]
fn run_command_executes_program() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let output = Command::new(exe)
        .args(["run", "examples/hello.mu"])
        .output()
        .expect("binary should run");

    assert!(output.status.success(), "run should succeed with VM implemented");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "run should not emit errors, got: {stderr}"
    );
}

#[test]
fn run_command_executes_built_mub() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let out = unique_temp_mub_file("hello_run");

    let build = Command::new(exe)
        .args([
            "build",
            "examples/hello.mu",
            "-o",
            out.to_str().expect("temp path should be valid utf8"),
        ])
        .output()
        .expect("binary should run");
    assert!(build.status.success(), "build should succeed");

    let run = Command::new(exe)
        .args(["run", out.to_str().expect("temp path should be valid utf8")])
        .output()
        .expect("binary should run");
    assert!(run.status.success(), "run on .mub should succeed");

    let _ = fs::remove_file(out);
}
