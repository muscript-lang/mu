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

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("muc_dir_{name}_{nanos}"))
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
fn run_command_executes_json_example() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let output = Command::new(exe)
        .args(["run", "examples/json.mu"])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "run should succeed for json example: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("{\"mu\":1}") || stdout.contains("{\"mu\":1.0}"),
        "json example should print stringify(parse(...)) output, got: {stdout}"
    );
}

#[test]
fn run_command_executes_http_example() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let output = Command::new(exe)
        .args(["run", "examples/http.mu"])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "run should succeed for http example: {}",
        String::from_utf8_lossy(&output.stderr)
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

#[test]
fn run_command_executes_built_http_mub() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let out = unique_temp_mub_file("http_run");

    let build = Command::new(exe)
        .args([
            "build",
            "examples/http.mu",
            "-o",
            out.to_str().expect("temp path should be valid utf8"),
        ])
        .output()
        .expect("binary should run");
    assert!(build.status.success(), "http build should succeed");

    let run = Command::new(exe)
        .args(["run", out.to_str().expect("temp path should be valid utf8")])
        .output()
        .expect("binary should run");
    assert!(run.status.success(), "run on http .mub should succeed");

    let _ = fs::remove_file(out);
}

#[test]
fn run_command_executes_built_json_mub() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let out = unique_temp_mub_file("json_run");

    let build = Command::new(exe)
        .args([
            "build",
            "examples/json.mu",
            "-o",
            out.to_str().expect("temp path should be valid utf8"),
        ])
        .output()
        .expect("binary should run");
    assert!(build.status.success(), "json build should succeed");

    let run = Command::new(exe)
        .args(["run", out.to_str().expect("temp path should be valid utf8")])
        .output()
        .expect("binary should run");
    assert!(run.status.success(), "run on json .mub should succeed");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("{\"mu\":1}") || stdout.contains("{\"mu\":1.0}"),
        "json .mub run should print roundtrip output, got: {stdout}"
    );

    let _ = fs::remove_file(out);
}

#[test]
fn run_loads_local_modules_for_import_validation() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let dir = unique_temp_dir("run_imports");
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let dep = dir.join("dep.mu");
    let main = dir.join("main.mu");
    fs::write(&dep, "@dep.mod{E[v];V v:i32=1;}").expect("dep source should be written");
    fs::write(&main, "@main.app{:d=dep.mod;F main:()->i32=0;}").expect("main source should be written");

    let output = Command::new(exe)
        .args(["run", main.to_str().expect("temp path should be valid utf8")])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "run should load sibling modules for import validation: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_file(dep);
    let _ = fs::remove_file(main);
    let _ = fs::remove_dir(dir);
}

#[test]
fn build_loads_local_modules_for_import_validation() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let dir = unique_temp_dir("build_imports");
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let dep = dir.join("dep.mu");
    let main = dir.join("main.mu");
    let out = dir.join("main.mub");
    fs::write(&dep, "@dep.mod{E[v];V v:i32=1;}").expect("dep source should be written");
    fs::write(&main, "@main.app{:d=dep.mod;F main:()->i32=0;}").expect("main source should be written");

    let output = Command::new(exe)
        .args([
            "build",
            main.to_str().expect("temp path should be valid utf8"),
            "-o",
            out.to_str().expect("temp path should be valid utf8"),
        ])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "build should load sibling modules for import validation: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_file(dep);
    let _ = fs::remove_file(main);
    let _ = fs::remove_file(out);
    let _ = fs::remove_dir(dir);
}

#[test]
fn run_loads_nested_local_modules_for_import_validation() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let dir = unique_temp_dir("run_nested_imports");
    let nested = dir.join("nested");
    fs::create_dir_all(&nested).expect("nested temp dir should be created");
    let dep = nested.join("dep.mu");
    let main = dir.join("main.mu");
    fs::write(&dep, "@dep.mod{E[v];V v:i32=1;}").expect("dep source should be written");
    fs::write(&main, "@main.app{:d=dep.mod;F main:()->i32=0;}").expect("main source should be written");

    let output = Command::new(exe)
        .args(["run", main.to_str().expect("temp path should be valid utf8")])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "run should load nested local modules for import validation: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_file(dep);
    let _ = fs::remove_file(main);
    let _ = fs::remove_dir(nested);
    let _ = fs::remove_dir(dir);
}

#[test]
fn build_loads_nested_local_modules_for_import_validation() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let dir = unique_temp_dir("build_nested_imports");
    let nested = dir.join("nested");
    fs::create_dir_all(&nested).expect("nested temp dir should be created");
    let dep = nested.join("dep.mu");
    let main = dir.join("main.mu");
    let out = dir.join("main.mub");
    fs::write(&dep, "@dep.mod{E[v];V v:i32=1;}").expect("dep source should be written");
    fs::write(&main, "@main.app{:d=dep.mod;F main:()->i32=0;}").expect("main source should be written");

    let output = Command::new(exe)
        .args([
            "build",
            main.to_str().expect("temp path should be valid utf8"),
            "-o",
            out.to_str().expect("temp path should be valid utf8"),
        ])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "build should load nested local modules for import validation: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_file(dep);
    let _ = fs::remove_file(main);
    let _ = fs::remove_file(out);
    let _ = fs::remove_dir(nested);
    let _ = fs::remove_dir(dir);
}

#[test]
fn check_file_loads_local_modules_for_import_validation() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let dir = unique_temp_dir("check_imports");
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let dep = dir.join("dep.mu");
    let main = dir.join("main.mu");
    fs::write(&dep, "@dep.mod{E[v];V v:i32=1;}").expect("dep source should be written");
    fs::write(&main, "@main.app{:d=dep.mod;F main:()->i32=0;}").expect("main source should be written");

    let output = Command::new(exe)
        .args(["check", main.to_str().expect("temp path should be valid utf8")])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "check should load sibling modules for import validation: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_file(dep);
    let _ = fs::remove_file(main);
    let _ = fs::remove_dir(dir);
}

#[test]
fn check_file_loads_nested_local_modules_for_import_validation() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let dir = unique_temp_dir("check_nested_imports");
    let nested = dir.join("nested");
    fs::create_dir_all(&nested).expect("nested temp dir should be created");
    let dep = nested.join("dep.mu");
    let main = dir.join("main.mu");
    fs::write(&dep, "@dep.mod{E[v];V v:i32=1;}").expect("dep source should be written");
    fs::write(&main, "@main.app{:d=dep.mod;F main:()->i32=0;}").expect("main source should be written");

    let output = Command::new(exe)
        .args(["check", main.to_str().expect("temp path should be valid utf8")])
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "check should load nested modules for import validation: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_file(dep);
    let _ = fs::remove_file(main);
    let _ = fs::remove_dir(nested);
    let _ = fs::remove_dir(dir);
}
