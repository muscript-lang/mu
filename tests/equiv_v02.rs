use std::fs;
use std::path::Path;
use std::process::Command;

fn run_muc(args: &[&str]) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_muc");
    Command::new(exe)
        .args(args)
        .output()
        .expect("muc should run")
}

fn pair_paths(name: &str) -> (String, String) {
    (
        format!("tests/equiv/readable/{name}.mu"),
        format!("tests/equiv/compressed/{name}.mu"),
    )
}

#[test]
fn readable_and_compressed_pairs_run_with_same_output_and_exit_code() {
    for name in ["hello", "json_transform", "match_adt"] {
        let (readable, compressed) = pair_paths(name);
        assert!(Path::new(&readable).exists());
        assert!(Path::new(&compressed).exists());

        let check_r = run_muc(&["check", &readable]);
        let check_c = run_muc(&["check", &compressed]);
        assert!(check_r.status.success(), "readable check failed for {name}");
        assert!(
            check_c.status.success(),
            "compressed check failed for {name}"
        );

        let run_r = run_muc(&["run", &readable]);
        let run_c = run_muc(&["run", &compressed]);

        assert_eq!(
            run_r.status.code(),
            run_c.status.code(),
            "exit mismatch for {name}"
        );
        assert_eq!(run_r.stdout, run_c.stdout, "stdout mismatch for {name}");
        assert_eq!(run_r.stderr, run_c.stderr, "stderr mismatch for {name}");
    }
}

#[test]
fn readable_and_compressed_negative_pair_match_effect_violation() {
    let (readable, compressed) = pair_paths("effect_violation");

    let out_r = run_muc(&["check", &readable]);
    let out_c = run_muc(&["check", &compressed]);

    assert!(
        !out_r.status.success(),
        "readable negative case should fail"
    );
    assert!(
        !out_c.status.success(),
        "compressed negative case should fail"
    );

    let err_r = String::from_utf8_lossy(&out_r.stderr);
    let err_c = String::from_utf8_lossy(&out_c.stderr);
    assert!(
        err_r.contains("E3007"),
        "expected E3007 in readable stderr: {err_r}"
    );
    assert!(
        err_c.contains("E3007"),
        "expected E3007 in compressed stderr: {err_c}"
    );
}

#[test]
fn compressed_fixtures_are_canonical_in_compressed_mode() {
    for entry in fs::read_dir("tests/fixtures/compressed").expect("fixtures dir should exist") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("mu") {
            continue;
        }
        let out = run_muc(&[
            "fmt",
            "--mode=compressed",
            "--check",
            path.to_str().expect("utf8 path"),
        ]);
        assert!(
            out.status.success(),
            "compressed fixture must be canonical: {}",
            path.display()
        );
    }
}
