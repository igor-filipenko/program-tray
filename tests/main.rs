use assert_cmd::Command;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_no_args() {
    let mut cmd = Command::cargo_bin("program-tray").unwrap();
    cmd.assert().failure().code(2);
}

#[test]
fn test_file_not_found() {
    let mut cmd = Command::cargo_bin("program-tray").unwrap();
    cmd.arg("not-exists-file").assert().failure().code(1);
}

#[test]
fn test_check_only() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap();
    temp_file
        .as_file()
        .write_all(
            br#"
          id = "id1"
          command = "command1"
        "#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("program-tray").unwrap();
    cmd.arg("--check-only").arg(path);
    cmd.assert().success().code(0);
}
