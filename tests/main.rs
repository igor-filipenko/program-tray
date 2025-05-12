use assert_cmd::Command;

#[test]
fn test_no_args() {
    let mut cmd = Command::cargo_bin("program-tray").unwrap();
    cmd.assert()
        .failure()
        .code(1)
        .stderr("Usage: program-tray <config-toml-file-path>\n");
}

#[test]
fn test_file_not_found() {
    let mut cmd = Command::cargo_bin("program-tray").unwrap();
    cmd.arg("not-exists-file").assert().failure().stderr(
        "Failed to read not-exists-file with error: No such file or directory (os error 2)\n",
    );
}
