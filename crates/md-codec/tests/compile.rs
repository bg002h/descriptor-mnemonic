#![cfg(feature = "cli-compiler")]
use assert_cmd::Command;

/// Goldens are deliberately conservative: rust-miniscript's compiler is
/// heuristic for non-trivial policies (e.g. `thresh(...)` may emit
/// `c:pk_k`/`c:pk_h` wrappers rather than a literal `multi`/`multi_a`).
/// We pin only the trivial single-key cases here. To extend the table,
/// run `md compile <expr> --context <ctx>` once, paste the literal
/// output as the expected prefix, and document the miniscript revision
/// in a comment alongside.
const GOLDEN: &[(&str, &str, &str)] = &[
    // (policy, context, expected_template_starts_with)
    ("pk(@0)", "segwitv0", "wsh(pk(@0))"),
    ("pk(@0)", "tap",      "tr(@0)"),
];

#[test]
fn compiler_golden_table() {
    for (expr, ctx, expected_prefix) in GOLDEN {
        let out = Command::cargo_bin("md").unwrap()
            .args(["compile", expr, "--context", ctx])
            .output().unwrap();
        let actual = String::from_utf8(out.stdout).unwrap();
        let actual_first = actual.lines().next().unwrap();
        assert!(
            actual_first.starts_with(expected_prefix),
            "compile({expr}, {ctx}) → {actual_first}, expected prefix {expected_prefix}"
        );
    }
}

#[test]
fn threshold_compiles_to_some_template() {
    // Sanity check only: thresh(2,pk,pk,pk) compiles to *some* well-formed
    // template starting with `wsh(...)`. The exact AST shape depends on
    // miniscript's heuristics; we don't pin it.
    let out = Command::cargo_bin("md").unwrap()
        .args(["compile", "thresh(2,pk(@0),pk(@1),pk(@2))", "--context", "segwitv0"])
        .output().unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    let first = s.lines().next().unwrap();
    assert!(first.starts_with("wsh("), "got: {first}");
    assert!(first.contains("@0") && first.contains("@1") && first.contains("@2"),
        "expected all three placeholders in {first}");
}
