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
        .write_str("const answer = 42;\n")
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
fn check_typescript_debugger_prints_bundled_rule_diagnostic() {
    let mut command = Command::cargo_bin("knot").expect("binary should build");

    let output = command
        .args(["check", "tests/error-fixtures/debugger.ts"])
        .assert()
        .success()
        .get_output()
        .clone();

    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout), @"tests/error-fixtures/debugger.ts:2:3: warning[knot/ts-debugger]: Unexpected debugger statement.");
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stderr), @"");
}

#[test]
fn check_typescript_console_prints_bundled_rule_diagnostics() {
    let mut command = Command::cargo_bin("knot").expect("binary should build");

    let output = command
        .args(["check", "tests/error-fixtures/console.ts"])
        .assert()
        .success()
        .get_output()
        .clone();

    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout), @r#"
tests/error-fixtures/console.ts:2:3: warning[knot/ts-console]: Unexpected console statement.
tests/error-fixtures/console.ts:3:3: warning[knot/ts-console]: Unexpected console statement.
"#);
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stderr), @"");
}

#[test]
fn check_python_mutable_default_arg_prints_bundled_rule_diagnostics() {
    let mut command = Command::cargo_bin("knot").expect("binary should build");

    let output = command
        .args(["check", "tests/error-fixtures/mutable_default.py"])
        .assert()
        .success()
        .get_output()
        .clone();

    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout), @r#"
tests/error-fixtures/mutable_default.py:1:17: warning[knot/py-mutable-default-arg]: Mutable default argument.
tests/error-fixtures/mutable_default.py:4:15: warning[knot/py-mutable-default-arg]: Mutable default argument.
tests/error-fixtures/mutable_default.py:7:10: warning[knot/py-mutable-default-arg]: Mutable default argument.
"#);
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stderr), @"");
}

#[test]
fn check_json_format_prints_json_diagnostics() {
    let mut command = Command::cargo_bin("knot").expect("binary should build");

    let output = command
        .args([
            "check",
            "--format",
            "json",
            "tests/error-fixtures/debugger.ts",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout), @r#"
[
  {
    "rule_id": "knot/ts-debugger",
    "severity": "warning",
    "message": "Unexpected debugger statement.",
    "span": {
      "file": "tests/error-fixtures/debugger.ts",
      "start_byte": 23,
      "end_byte": 32,
      "start_line": 2,
      "start_column": 3,
      "end_line": 2,
      "end_column": 12
    }
  }
]
"#);
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
