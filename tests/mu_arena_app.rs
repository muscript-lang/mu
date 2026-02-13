use std::process::Command;

fn run_muc(args: &[&str]) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_muc");
    Command::new(exe)
        .args(args)
        .output()
        .expect("muc command should run")
}

#[test]
fn mu_arena_mu_unit_tests_pass() {
    for file in [
        "apps/mu_arena/src/arena_model.mu",
        "apps/mu_arena/src/policies.mu",
        "apps/mu_arena/src/runner.mu",
        "apps/mu_arena/tests/runner_test.mu",
    ] {
        let out = run_muc(&["run", file]);
        assert!(
            out.status.success(),
            "{file} should pass: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn mu_arena_main_is_deterministic() {
    let out1 = run_muc(&["run", "apps/mu_arena/src/main.mu"]);
    let out2 = run_muc(&["run", "apps/mu_arena/src/main.mu"]);
    assert!(out1.status.success());
    assert!(out2.status.success());

    let s1 = String::from_utf8_lossy(&out1.stdout);
    let s2 = String::from_utf8_lossy(&out2.stdout);
    assert_eq!(s1, s2, "mu_arena output should be deterministic");

    let best = s1.lines().last().unwrap_or("");
    assert!(best.starts_with("BEST policy="), "unexpected best line: {best}");
}

#[test]
fn mu_arena_cli_checks_pass() {
    for args in [
        vec!["fmt", "--mode=readable", "--check", "apps/mu_arena/src"],
        vec!["check", "apps/mu_arena/src/main.mu"],
        vec!["run", "apps/mu_arena/src/main.mu"],
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
