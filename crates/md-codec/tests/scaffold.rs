use assert_cmd::Command;

#[test]
fn md_help_runs() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.arg("--help").assert().success();
}

#[test]
fn md_encode_help_runs() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "--help"]).assert().success();
}

#[test]
fn md_no_args_fails_with_usage_error() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.assert().code(2);
}
