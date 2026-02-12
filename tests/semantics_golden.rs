use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cases() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir("tests/semantics").expect("semantics fixtures should exist") {
        let entry = entry.expect("fixture entry should be readable");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("mu") {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn read_trimmed(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
        .trim()
        .to_string()
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn semantics_goldens() {
    let exe = env!("CARGO_BIN_EXE_muc");
    for mu in cases() {
        let stem = mu
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("fixture stem should be utf8");
        let base = mu.with_extension("");

        let cmd = read_trimmed(&base.with_extension("cmd"));
        let expected_exit = read_trimmed(&base.with_extension("exitcode"))
            .parse::<i32>()
            .expect("exitcode should be i32");
        let expected_stdout = read_text(&base.with_extension("stdout"));
        let expected_stderr_needles = read_text(&base.with_extension("stderr"));

        let output = Command::new(exe)
            .arg(cmd.as_str())
            .arg(mu.as_path())
            .output()
            .unwrap_or_else(|e| panic!("{stem}: failed to run muc: {e}"));

        let actual_code = output.status.code().unwrap_or(-1);
        assert_eq!(
            actual_code,
            expected_exit,
            "{stem}: unexpected exit code\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.as_ref(),
            expected_stdout,
            "{stem}: stdout mismatch\nexpected:\n{expected_stdout}\nactual:\n{stdout}"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        for needle in expected_stderr_needles
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            assert!(
                stderr.contains(needle),
                "{stem}: stderr should contain `{needle}`\nactual:\n{stderr}"
            );
        }
    }
}
