use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn run_muc(args: &[&str]) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_muc");
    Command::new(exe)
        .args(args)
        .output()
        .expect("muc command should run")
}

fn unique_temp_mu(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("signal_reactor_{name}_{nanos}.mu"))
}

#[test]
fn signal_reactor_rules_spec_passes() {
    let out = run_muc(&["run", "apps/signal_reactor/src/rules.mu"]);
    assert!(
        out.status.success(),
        "rules spec should pass: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn signal_reactor_integration_default_fixture_outputs_expected_lines() {
    let out = run_muc(&["run", "apps/signal_reactor/src/signal_reactor.mu"]);
    assert!(
        out.status.success(),
        "signal reactor run should pass: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines = stdout.lines().collect::<Vec<_>>();
    let expected = vec!["EL", "H", "XL", "ES", "XS", "H", "ERR", "ES", "H", "ERR"];

    assert_eq!(
        lines.len(),
        expected.len(),
        "unexpected line count: {stdout}"
    );
    assert_eq!(lines[0], expected[0]);
    assert_eq!(lines[3], expected[3]);
    assert_eq!(lines[6], expected[6]);
    assert_eq!(lines[9], expected[9]);
    assert_eq!(lines, expected);
}

#[test]
fn signal_reactor_cli_checks_pass() {
    for args in [
        vec![
            "fmt",
            "--mode=readable",
            "--check",
            "apps/signal_reactor/src",
        ],
        vec!["check", "apps/signal_reactor/src/signal_reactor.mu"],
        vec!["run", "apps/signal_reactor/src/signal_reactor.mu"],
    ] {
        let out = run_muc(&args);
        assert!(
            out.status.success(),
            "muc {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn signal_reactor_compressed_format_runs() {
    let src = fs::read_to_string("apps/signal_reactor/src/signal_reactor.mu")
        .expect("signal reactor source should exist");
    let temp = unique_temp_mu("compressed_check");
    fs::write(&temp, src).expect("temp source should be writable");

    let out1 = run_muc(&[
        "fmt",
        "--mode=compressed",
        temp.to_str().expect("temp path should be valid utf8"),
    ]);
    assert!(
        out1.status.success(),
        "first compressed fmt should pass: {}",
        String::from_utf8_lossy(&out1.stderr)
    );

    let _ = fs::remove_file(temp);
}
