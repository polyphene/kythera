use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn cli_tmp_command_callable() {
    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.arg("tmp").assert().success();
}

#[test]
fn cli_tmp_print_config_command_callable() {
    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.arg("tmp").arg("print-config").assert().success();
}

#[test]
fn default_context_when_no_config_file() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let assert = cmd
        .current_dir(temp_dir.path())
        .arg("tmp")
        .arg("print-config")
        .assert();
    assert.success().stdout(
        predicate::str::starts_with("actors_bin_dir").and(predicate::str::ends_with("artifacts\n")),
    );
}

#[test]
fn default_context_value_when_empty_field() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .child("kythera.config.yml")
        .write_str("other_key: other_value")
        .unwrap();
    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let assert = cmd
        .current_dir(temp_dir.path())
        .arg("tmp")
        .arg("print-config")
        .assert();
    assert.success().stdout(
        predicate::str::starts_with("actors_bin_dir").and(predicate::str::ends_with("artifacts\n")),
    );
}

#[test]
fn valid_context_value_from_config_file() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .child("kythera.config.yml")
        .write_str("actors_bin_dir: ./custom-artifacts/")
        .unwrap();
    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let assert = cmd
        .current_dir(temp_dir.path())
        .arg("tmp")
        .arg("print-config")
        .assert();
    assert.success().stdout(
        predicate::str::starts_with("actors_bin_dir")
            .and(predicate::str::ends_with("custom-artifacts\n")),
    );
}

#[test]
fn context_error_on_security_issue() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .child("kythera.config.yml")
        .write_str("actors_bin_dir: ../unsecure_dir/")
        .unwrap();
    let mut cmd = Command::cargo_bin("kythera").unwrap();
    let assert = cmd
        .current_dir(temp_dir.path())
        .arg("tmp")
        .arg("print-config")
        .assert();
    assert.failure().stderr(predicate::str::contains(
        "file path outside the project directory",
    ));
}
