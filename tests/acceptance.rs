use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_mu(name: &str, src: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("muc_accept_{name}_{nanos}.mu"));
    fs::write(&path, src).expect("fixture should be writable");
    path
}

fn temp_mub(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("muc_accept_{name}_{nanos}.mub"))
}

#[test]
fn acceptance_end_to_end_run_and_build() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let src = "@acc.ok{F main:()->i32=v(x:i32=0,v(fn1=l(y:i32):i32=c(+,x,y),c(fn1,0)));}";
    let file = temp_mu("ok", src);
    let out = temp_mub("ok");

    let check = Command::new(exe)
        .args(["check", file.to_str().expect("utf8 path")])
        .output()
        .expect("check should run");
    assert!(check.status.success(), "check should pass");

    let run_src = Command::new(exe)
        .args(["run", file.to_str().expect("utf8 path")])
        .output()
        .expect("run should execute source");
    assert!(run_src.status.success(), "run source should pass");

    let build = Command::new(exe)
        .args([
            "build",
            file.to_str().expect("utf8 path"),
            "-o",
            out.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("build should run");
    assert!(build.status.success(), "build should pass");

    let run_mub = Command::new(exe)
        .args(["run", out.to_str().expect("utf8 path")])
        .output()
        .expect("run should execute bytecode");
    assert!(run_mub.status.success(), "run .mub should pass");

    let _ = fs::remove_file(file);
    let _ = fs::remove_file(out);
}

#[test]
fn acceptance_rejects_noncanonical_effects_and_main_signature() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let bad_effect = temp_mu("bad_effect", "@acc.bad1{F main:()->i32!{fs,io}=0;}");
    let bad_main = temp_mu("bad_main", "@acc.bad2{F main:(i32)->i32=arg0;}");

    let out1 = Command::new(exe)
        .args(["check", bad_effect.to_str().expect("utf8 path")])
        .output()
        .expect("check should run");
    assert!(!out1.status.success(), "noncanonical effect should fail");
    assert!(
        String::from_utf8_lossy(&out1.stderr).contains("E3012"),
        "should show E3012"
    );

    let out2 = Command::new(exe)
        .args(["check", bad_main.to_str().expect("utf8 path")])
        .output()
        .expect("check should run");
    assert!(!out2.status.success(), "bad main signature should fail");
    assert!(
        String::from_utf8_lossy(&out2.stderr).contains("E3014"),
        "should show E3014"
    );

    let _ = fs::remove_file(bad_effect);
    let _ = fs::remove_file(bad_main);
}

#[test]
fn acceptance_examples_run_and_build_matrix() {
    let exe = env!("CARGO_BIN_EXE_muc");
    for example in ["examples/hello.mu", "examples/json.mu", "examples/http.mu"] {
        let check = Command::new(exe)
            .args(["check", example])
            .output()
            .expect("check should run");
        assert!(
            check.status.success(),
            "check should pass for {example}: {}",
            String::from_utf8_lossy(&check.stderr)
        );

        let run_src = Command::new(exe)
            .args(["run", example])
            .output()
            .expect("run should execute source");
        assert!(
            run_src.status.success(),
            "run should pass for {example}: {}",
            String::from_utf8_lossy(&run_src.stderr)
        );

        let out = temp_mub("example_matrix");
        let build = Command::new(exe)
            .args(["build", example, "-o", out.to_str().expect("utf8 path")])
            .output()
            .expect("build should run");
        assert!(
            build.status.success(),
            "build should pass for {example}: {}",
            String::from_utf8_lossy(&build.stderr)
        );

        let run_mub = Command::new(exe)
            .args(["run", out.to_str().expect("utf8 path")])
            .output()
            .expect("run should execute bytecode");
        assert!(
            run_mub.status.success(),
            "run .mub should pass for {example}: {}",
            String::from_utf8_lossy(&run_mub.stderr)
        );

        let _ = fs::remove_file(out);
    }
}
