//! `md gen-man --out <DIR>` — emit roff man pages for the whole `md` CLI tree.
//!
//! Generates one `*.1` page per (sub)command directly from the compiled clap
//! `Command` tree via `clap_mangen::generate_to`. The pages are
//! binary-faithful by construction: clap_mangen renders from the same
//! `clap::Command` the binary parses, so a page cannot drift from the actual
//! flag surface (no content-fidelity gate is needed — SPEC §1 non-goals).
//!
//! **Naive call — NO pre-`.build()` (SPEC §2, C-1).** `clap_mangen::generate_to`
//! builds the tree internally *after* `disable_help_subcommand(true)`. An
//! external `root.build()` would run first and materialize the `help`
//! pseudo-subcommand shadow tree as real entries, poisoning the output with
//! spurious `*-help*.1` pages. The bare call below is clean.

use crate::error::CliError;
use clap::CommandFactory;
use std::fs;
use std::path::PathBuf;

use crate::Cli;

pub fn run(out: PathBuf) -> Result<u8, CliError> {
    fs::create_dir_all(&out).map_err(|e| CliError::BadArg(format!("mkdir {out:?}: {e}")))?;
    // Naive call: Cli::command() is UNBUILT; generate_to builds internally and
    // suppresses the auto `help` subcommand (C-1). Do NOT pre-`.build()`.
    clap_mangen::generate_to(Cli::command(), &out)
        .map_err(|e| CliError::BadArg(format!("gen-man write to {out:?}: {e}")))?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Command;
    use std::collections::BTreeSet;

    /// Walk the UNBUILT command tree exactly the way `generate_to` does:
    /// recurse into every visible subcommand, skip `is_hide_set()` and the
    /// auto `help`, and collect the hyphen-joined display filenames. This is
    /// the golden expected-set the produced `*.1` files must match exactly
    /// (SPEC §8 P1 exact-page-set walk, I-2).
    fn expected_pages(cmd: &Command, prefix: &str, acc: &mut BTreeSet<String>) {
        // generate_to names files by hyphen-joining the parent chain onto the
        // child name (e.g. `md-encode.1`); the root uses its bare name.
        let stem = if prefix.is_empty() {
            cmd.get_name().to_string()
        } else {
            format!("{prefix}-{}", cmd.get_name())
        };
        acc.insert(format!("{stem}.1"));
        for sub in cmd.get_subcommands() {
            if sub.is_hide_set() || sub.get_name() == "help" {
                continue;
            }
            expected_pages(sub, &stem, acc);
        }
    }

    #[test]
    fn gen_man_exact_page_set_walk() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path().to_path_buf()).unwrap();

        let produced: BTreeSet<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.ends_with(".1"))
            .collect();

        // Build the expected set from the UNBUILT tree (mirrors the naive
        // generate_to call — no pre-`.build()`, C-1).
        let root = Cli::command();
        let mut expected = BTreeSet::new();
        expected_pages(&root, "", &mut expected);

        assert_eq!(
            produced, expected,
            "produced man-page set must exactly equal the walked unbuilt tree"
        );
        // Root page present.
        assert!(produced.contains("md.1"), "root md.1 missing: {produced:?}");
        // The new gen-man page present.
        assert!(
            produced.contains("md-gen-man.1"),
            "md-gen-man.1 missing: {produced:?}"
        );
        // No magic integer — the set is derived, not baked.
        assert!(!produced.is_empty(), "produced set is empty");
    }

    #[test]
    fn gen_man_negative_canary_no_help_pages() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path().to_path_buf()).unwrap();
        for entry in std::fs::read_dir(dir.path()).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            assert!(
                !(name == "md-help.1" || name.contains("-help-") || name.ends_with("-help.1")),
                "spurious help shadow page produced (accidental pre-build?): {name}"
            );
        }
    }

    #[test]
    fn gen_man_root_page_has_th_header_and_distinct_filenames() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path().to_path_buf()).unwrap();

        let root = std::fs::read_to_string(dir.path().join("md.1")).unwrap();
        assert!(
            root.contains(".TH"),
            "root md.1 missing roff .TH header:\n{}",
            &root[..root.len().min(200)]
        );

        // Every produced page is non-empty, and filenames are distinct (a
        // BTreeSet of read_dir entries de-dupes — assert the count matches the
        // raw read_dir count so no two display-names collided).
        let raw: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.ends_with(".1"))
            .collect();
        let distinct: BTreeSet<&String> = raw.iter().collect();
        assert_eq!(raw.len(), distinct.len(), "duplicate page filenames");
        for n in &raw {
            let bytes = std::fs::read(dir.path().join(n)).unwrap();
            assert!(!bytes.is_empty(), "page {n} is empty");
        }
    }
}
