#![allow(missing_docs)]
//! P3 row 20 — THE DECLINE, ASSERTED.
//!
//! `md` keeps what no §6 ruling changes. These are backstops: they protect the
//! RED-first entries from a later phase "tidying up" by adopting a crate item
//! P3 deliberately declined, or by widening a rule §6 scoped narrowly. Each one
//! names the ruling it rests on, so a future reader cannot delete it as noise.
//!
//! `md` adopts **1 of the crate's 11 items** — `write::write_private` — and
//! declines the other ten.

use assert_cmd::Command;

const CARD: &str = "md1yqpqqxqq8xtwhw4xwn4qh";
const TEMPLATE: &str = "wpkh(@0/<0;1>/*)";

/// Every `.rs` file under `crates/md-cli/src`, read once.
fn md_cli_sources() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push((
                    p.display().to_string(),
                    std::fs::read_to_string(&p).unwrap(),
                ));
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, &mut out);
    out
}

/// **The boundary table, enforced.** `md` adopts `write_private` and nothing
/// else from `mnemonic-io-lib`. The declines are not stylistic:
///
/// * `exit::write_block` / `WriteBlock` — `write_block`'s `Terminal` arm
///   refuses unconditionally, and §6e RETRACTED the generalisation of `me`'s
///   terminal gate in as many words. md1 strings are short printable ASCII a
///   human must read in order to engrave them.
/// * `observation::PayloadKind` — its variants are `Bearer` and
///   `CarriesNoSecret`, and md1 is neither. `CarriesNoSecret` is documented as
///   a destroy-fill image, so filing a template under it would make
///   `exposure_matters()` a constant false AND say something untrue.
/// * `records::split_record_stream` — filters on `trim().is_empty()` and
///   returns the line unchanged, so it does NOT strip display separators per
///   line. `md`'s own reader does, which is the whole mstring-grouping
///   contract.
/// * `records::no_records_guard` — its refusal text names `mt encode --qr`,
///   another binary's flag.
/// * `channel::destination`, `fd::stdout_mode`, `fd::mode_of` — nothing in
///   P3's row for `md` reads or reasons about a mode except through
///   `write_private`, which sets one rather than measuring one.
/// * `remedy::*` — §4 exempts `md` from the argv refusal by name, so there is
///   nothing to print a purge recipe for.
#[test]
fn md_adopts_exactly_one_crate_item_and_declines_the_other_ten() {
    let sources = md_cli_sources();
    assert!(
        sources.len() > 10,
        "the source scan found {} files -- it is not reaching src/, so a clean \
         result here would be a false PASS",
        sources.len()
    );

    // Every path rooted at the crate, collected. A crate item can only be
    // REACHED through `mnemonic_io_lib::…` (there is no `use mnemonic_io_lib::`
    // anywhere either, asserted below), so this finds adoptions and not the
    // prose that explains the declines -- an earlier version of this test
    // matched bare item names and flagged its own doc comment.
    const ROOT: &str = "mnemonic_io_lib::";
    let mut adopted: Vec<(String, String)> = Vec::new();
    for (path, body) in &sources {
        assert!(
            !body.contains("use mnemonic_io_lib"),
            "{path} imports from the crate with `use`; this test reads fully \
             qualified paths, so an import would hide an adoption from it"
        );
        let mut rest = body.as_str();
        while let Some(i) = rest.find(ROOT) {
            rest = &rest[i + ROOT.len()..];
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
                .unwrap_or(rest.len());
            adopted.push((path.clone(), rest[..end].trim_end_matches(':').to_string()));
            rest = &rest[end..];
        }
    }

    // Positive control: the ONE adopted item is genuinely present, so a clean
    // result below is evidence rather than an empty scan.
    assert!(
        !adopted.is_empty(),
        "no crate path found at all -- the scan proves nothing"
    );
    for (path, item) in &adopted {
        assert_eq!(
            item, "write::write_private",
            "{path} reaches `mnemonic_io_lib::{item}`. P3's boundary table gives \
             `md` ONE of the crate's 11 items -- `write::write_private` -- and \
             declines the other ten. If a later phase wants one, that is a \
             ruling, not a tidy-up."
        );
    }
}

/// §6e — the terminal / world-readable write gate stays scoped to `me`'s binary
/// container. `md` writes a world-readable stdout without refusing.
///
/// This is the observable of `exit::write_block`'s refusal arms: `me sysw pack`
/// without `--out` exits 2 on a mode-0644 stdout. An adoption of that gate here
/// would red this test.
///
/// **The terminal arm itself is not asserted here** — it needs a pty, and the
/// suite runs on three OSes. Filed as F-332 rather than half-built; the crate
/// item that would bring the terminal refusal in is pinned by the source scan
/// above, which is the stronger of the two checks anyway.
#[test]
#[cfg(unix)]
fn md_writes_a_world_readable_stdout_without_refusing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("world-readable.md1");
    std::fs::write(&f, "").unwrap();
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
    let handle = std::fs::OpenOptions::new().write(true).open(&f).unwrap();
    let out = std::process::Command::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", TEMPLATE])
        .stdout(std::process::Stdio::from(handle))
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "md must not refuse a world-readable stdout (§6e); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read_to_string(&f).unwrap(), format!("{CARD}\n"));
    assert_eq!(
        std::fs::metadata(&f).unwrap().permissions().mode() & 0o777,
        0o644,
        "and it must not silently tighten a file the SHELL created"
    );
}

/// §6a scopes the stdout rule to `encode` by an explicit table. `md decode`'s
/// stdout is out of scope BY NAME and is unchanged: a bare template, one line.
#[test]
fn md_decode_stdout_shape_is_unchanged() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", CARD])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("{TEMPLATE}\n"),
        "§6a puts decode's stdout out of scope; it must not have moved"
    );
}

/// §4 exempts `md` from the argv refusal by name — *"md and mk DO take their
/// strings as arguments; md1/mk1 are watch-only, so a leak there costs privacy
/// rather than the money."* Adding one is a ruling P3 does not make, and
/// `mnemonic`'s refusal must not be copied here by a later reader looking for
/// uniformity.
#[test]
fn md_still_accepts_its_material_on_argv_with_no_refusal() {
    for args in [vec!["decode", CARD], vec!["encode", TEMPLATE]] {
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "`md {}` must still work on argv; stderr={}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("secret material on argv"),
            "no argv refusal or advisory on md (§4); got {stderr:?}"
        );
    }
}
