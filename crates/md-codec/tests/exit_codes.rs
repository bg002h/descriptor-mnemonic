use assert_cmd::Command;

#[test]
fn no_args_returns_2() {
    Command::cargo_bin("md").unwrap().assert().code(2);
}

#[test]
fn unknown_subcommand_returns_2() {
    Command::cargo_bin("md").unwrap().arg("bogus").assert().code(2);
}

#[test]
fn encode_bad_template_returns_1() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "this is not a template"])
        .assert().code(1);
}

#[test]
fn decode_bad_string_returns_1() {
    Command::cargo_bin("md").unwrap()
        .args(["decode", "not-a-valid-md-string"])
        .assert().code(1);
}
