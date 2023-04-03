mod tmp;

use assert_cmd::Command;

#[test]
fn cli_is_callable() {
    let mut cmd = Command::cargo_bin("kythera").unwrap();
    cmd.arg("--version").assert().success();
}
