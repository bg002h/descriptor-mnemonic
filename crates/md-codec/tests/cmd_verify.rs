use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn verify_match_returns_0() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    Command::cargo_bin("md").unwrap()
        .args(["verify", &phrase, "--template", template])
        .assert().code(0).stdout(predicates::str::contains("OK"));
}

#[test]
fn verify_mismatch_returns_1() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let wrong = "wpkh(@0/<0;1>/*)";
    Command::cargo_bin("md").unwrap()
        .args(["verify", &phrase, "--template", wrong])
        .assert().code(1).stderr(predicates::str::contains("MISMATCH"));
}
