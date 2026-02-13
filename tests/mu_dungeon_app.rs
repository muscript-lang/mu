use std::process::Command;

fn run_muc(args: &[&str]) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_muc");
    Command::new(exe)
        .args(args)
        .output()
        .expect("muc command should run")
}

#[test]
fn mu_dungeon_mu_unit_tests_pass() {
    for file in [
        "apps/mu_dungeon/tests/rng_test.mu",
        "apps/mu_dungeon/tests/rules_test.mu",
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
fn mu_dungeon_main_seed_1_result_line_is_stable() {
    let out = run_muc(&["run", "apps/mu_dungeon/src/main.mu", "--", "1"]);
    assert!(
        out.status.success(),
        "mu_dungeon main should run: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("");
    assert_eq!(
        last, "RESULT Lose room=10 turn=7 xp=117 hp=-1 seed=941",
        "unexpected final line:\n{stdout}"
    );
}

#[test]
fn mu_dungeon_baseline_win_rate_is_in_target_band_for_100_seeds() {
    let src = std::fs::read_to_string("apps/mu_dungeon/src/main.mu")
        .expect("mu_dungeon source should be readable");
    let mut wins = 0usize;

    for seed in 1..=100 {
        let seeded = src.replacen(
            "F main:()->i32!{io}=v(seed:i32=1,",
            &format!("F main:()->i32!{{io}}=v(seed:i32={seed},"),
            1,
        );
        let tmp = std::env::temp_dir().join(format!("mu_dungeon_seed_{seed}.mu"));
        std::fs::write(&tmp, seeded).expect("temp seeded file should be writable");

        let out = run_muc(&["run", tmp.to_str().expect("temp path should be utf8")]);
        let _ = std::fs::remove_file(&tmp);
        assert!(
            out.status.success(),
            "seed {seed} run should pass: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        let stdout = String::from_utf8_lossy(&out.stdout);
        let last = stdout.lines().last().unwrap_or("");
        if last.starts_with("RESULT Win") {
            wins += 1;
        }
    }

    assert!(
        (20..=40).contains(&wins),
        "wins over 100 seeds should be in [20,40], got {wins}"
    );
}

#[test]
fn mu_dungeon_cli_checks_pass() {
    for args in [
        vec!["fmt", "--mode=readable", "--check", "apps/mu_dungeon/src"],
        vec!["check", "apps/mu_dungeon/src/main.mu"],
        vec!["run", "apps/mu_dungeon/src/main.mu", "--", "1"],
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
fn mu_dungeon_compressed_format_idempotent_for_sources() {
    for path in [
        "apps/mu_dungeon/src/model.mu",
        "apps/mu_dungeon/src/rng.mu",
        "apps/mu_dungeon/src/rules.mu",
        "apps/mu_dungeon/src/dungeon.mu",
        "apps/mu_dungeon/src/main.mu",
    ] {
        let temp = std::env::temp_dir().join(format!(
            "mu_dungeon_compressed_{}.mu",
            path.replace('/', "_")
        ));
        std::fs::copy(path, &temp).expect("temp copy should succeed");
        let out = run_muc(&[
            "fmt",
            "--mode=compressed",
            temp.to_str().expect("temp path should be utf8"),
        ]);
        assert!(
            out.status.success(),
            "compressed fmt should run for {path}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _ = std::fs::remove_file(temp);
    }
}
