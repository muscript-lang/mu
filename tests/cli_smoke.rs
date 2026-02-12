use std::process::Command;

#[test]
fn help_prints_usage() {
    let exe = env!("CARGO_BIN_EXE_muc");
    let output = Command::new(exe)
        .arg("--help")
        .output()
        .expect("binary should run");

    assert!(output.status.success(), "--help should exit successfully");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("muc"), "help should mention binary name");
    assert!(stdout.contains("fmt"), "help should list subcommands");
}
