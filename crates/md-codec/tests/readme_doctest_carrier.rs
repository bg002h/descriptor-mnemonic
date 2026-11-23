//! The README doctest carrier cannot be dropped, nor a README rust block muted, without a red row.
#![allow(missing_docs)]

#[test]
fn i11_readme_doctest_carrier_is_wired_and_blocks_run() {
    let lib = include_str!("../src/lib.rs");
    assert!(
        lib.contains("#[cfg(any(test, doctest))]\nmod readme_doctests;"),
        "lib.rs must declare readme_doctests"
    );
    let carrier = include_str!("../src/readme_doctests.rs");
    assert!(
        carrier.contains("#[doc = include_str!(\"../README.md\")]"),
        "readme_doctests.rs must include the README"
    );
    let readme = include_str!("../README.md");
    for line in readme.lines().filter(|l| l.starts_with("```")) {
        assert!(
            ["```", "```rust", "```toml", "```sh"].contains(&line),
            "README fence `{line}` would mute or skip a doctest"
        );
    }
    assert_eq!(
        readme.matches("\n```rust\n").count(),
        2,
        "README must hold exactly 2 rust blocks"
    );
}
