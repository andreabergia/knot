use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn check_existing_file_succeeds_without_diagnostics() {
    let mut command = Command::cargo_bin("knot").expect("binary should build");

    let output = command
        .args(["check", "tests/fixtures/sample.py"])
        .assert()
        .success()
        .get_output()
        .clone();

    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout), @"");
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stderr), @"");
}

#[test]
fn check_existing_directory_succeeds_without_diagnostics() {
    let mut command = Command::cargo_bin("knot").expect("binary should build");

    let output = command
        .args(["check", "tests/fixtures"])
        .assert()
        .success()
        .get_output()
        .clone();

    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout), @"");
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stderr), @"");
}

#[test]
fn check_multiple_existing_paths_succeeds_without_diagnostics() {
    let temp = TempDir::new().expect("temp dir should be created");
    let python = temp.child("sample.py");
    let typescript = temp.child("sample.ts");
    python.write_str("print('hello')\n").expect("write fixture");
    typescript
        .write_str("console.log('hello');\n")
        .expect("write fixture");

    let mut command = Command::cargo_bin("knot").expect("binary should build");

    let output = command
        .arg("check")
        .arg(python.path())
        .arg(typescript.path())
        .assert()
        .success()
        .get_output()
        .clone();

    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout), @"");
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stderr), @"");
}

#[test]
fn check_syntax_error_prints_location() {
    let mut command = Command::cargo_bin("knot").expect("binary should build");

    let output = command
        .args(["check", "tests/error-fixtures/broken.py"])
        .assert()
        .success()
        .get_output()
        .clone();

    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout), @"tests/error-fixtures/broken.py:1:12: error[knot/syntax]: syntax error: missing )");
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stderr), @"");
}

#[test]
fn check_missing_path_fails() {
    let mut command = Command::cargo_bin("knot").expect("binary should build");

    command
        .args(["check", "missing-knot-path"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: path does not exist: missing-knot-path",
        ));
}
