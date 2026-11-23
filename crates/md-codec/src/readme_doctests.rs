//! Runs the Rust blocks of `crates/md-codec/README.md` as doctests (bh-md-I11), and keeps the
//! README's dependency pin in step with the crate version.

/// Carrier for the README doctests: `cargo test --doc` compiles and runs each ```` ```rust ````
/// block of the README.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

#[cfg(test)]
mod tests {
    #[test]
    fn i11_readme_dependency_pin_matches_crate_version() {
        let pin = format!(
            "md-codec = \"{}.{}\"",
            env!("CARGO_PKG_VERSION_MAJOR"),
            env!("CARGO_PKG_VERSION_MINOR")
        );
        let readme = include_str!("../README.md");
        assert!(readme.contains(&pin), "README quickstart must pin {pin}");
    }
}
