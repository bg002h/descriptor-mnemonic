use assert_cmd::Command;

fn long_help(sub: &str) -> String {
    let out = Command::cargo_bin("md").unwrap().args([sub, "--help"]).output().unwrap();
    String::from_utf8(out.stdout).unwrap()
}

/// Parse the EXAMPLES block from `<sub> --help`. Returns (cmdline, expected_stdout).
fn parse_example(help: &str) -> Option<(String, String)> {
    let block = help.split("EXAMPLES:").nth(1)?;
    let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
    let cmd_line = lines.first()?.trim().strip_prefix("$ ")?.to_string();
    let expected = lines.iter().skip(1).map(|s| s.trim_start()).collect::<Vec<_>>().join("\n");
    Some((cmd_line, expected))
}

fn check_example(sub: &str) {
    let help = long_help(sub);
    let (cmdline, expected) = parse_example(&help)
        .unwrap_or_else(|| panic!("{sub} --help has no EXAMPLES block"));
    let parts: Vec<&str> = cmdline.split_whitespace().collect();
    assert_eq!(parts[0], "md", "EXAMPLES cmdline must start with `md`");
    let out = Command::cargo_bin("md").unwrap().args(&parts[1..]).output().unwrap();
    let actual = String::from_utf8(out.stdout).unwrap();
    assert_eq!(actual.trim_end(), expected.trim_end(),
        "drift between {sub} --help EXAMPLE and actual stdout");
}

#[test]
fn encode_example_matches_actual_output() { check_example("encode"); }

#[test]
fn decode_example_matches_actual_output() { check_example("decode"); }
