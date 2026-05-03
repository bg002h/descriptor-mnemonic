use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn bytecode_prints_hex_and_lengths() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md").unwrap()
        .args(["bytecode", &phrase])
        .assert().success()
        .stdout(predicates::str::contains("payload-bits:"))
        .stdout(predicates::str::contains("payload-bytes:"))
        .stdout(predicates::str::contains("hex:"));
}
