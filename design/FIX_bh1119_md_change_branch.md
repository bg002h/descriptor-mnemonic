# FIX_bh1119_md_change_branch: `md descriptor --change` / `--chain N` refuses a branch no key has (bh-md-I6 Important)

Small-fix track (bughunt-1119 FIX table row #30; grade I; class B, funds-adjacent: a "change" descriptor equal to
the receive descriptor makes a coordinator reuse receive addresses for change). Rule decided by the grader
(coordinator `agent-reports/bughunt-1119/grade-md.md:51`): the descriptor route refuses a branch no slot has, using
the same max-alt rule (baseline plus every per-slot override) as `md address` (`crates/md-codec/src/derive.rs:108-132`),
not the triage scratch's all-nonmultipath check. Repo descriptor-mnemonic, branch `fix/md-change-branch` from main
`6aa7e5e32638e963014d43fcce251720090820b2` (md-codec 0.48.6, md-cli 0.21.0 at that head). Crates md-codec and md-cli; one md-codec function changes behaviour (S1-S3) and one md-cli block changes which verdict form prints (S9); no wire change, no new flag, type, error variant or command.
Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22.

**No STOP is open for this row.** No new printed string: the refusal is the existing `md address` text, and S9 only selects among existing verdict lines. The CHANGELOG and manual sentences in section 3 are PROPOSED (architect confirms). Section 8 lists one related defect (F2) outside this row; F1 (the multipath verdict on a wallet with no multipath) was folded into this row by the architect's ruling (C/agent-reports/architect-md30-q1.md) as S9.

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (`rust-toolchain.toml`); `--locked`; nextest
`--build-jobs 6 -j 6`; never `--release`. `X` below is
`xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf`
and `Y` is
`xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk`
(public test keys already in the repo's tests and `md decompose --help`; no seed is involved).

## 1. The defect (RED evidence, run by the author on the base)

`to_miniscript_descriptor_with_network` (`crates/md-codec/src/to_miniscript.rs:90-111`) resolves `chain` per key in
`use_site_to_derivation_path` (`:381-396`), which bounds `chain` only inside `if let Some(alts) = &u.multipath`. A key
with no multipath ignores `chain`, so on a wallet where NO key has a multipath every chain returns the chain-0
descriptor. `Descriptor::derive_address` (`derive.rs:108-132`) has a descriptor-wide pre-flight (max alt-count across
the baseline and every override, `None` counting as 1) and refuses; `md descriptor` (`crates/md-cli/src/cmd/descriptor.rs:204-208`)
calls `to_miniscript_descriptor_with_network` directly and does not.

| # | Command (base build, `target/debug/md`) | Base result |
|---|---|---|
| E1 | `md descriptor --template 'wpkh(@0/*)' --key @0=X --change` | exit 0; stdout `wpkh(xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/*)#fn4fyxff` (the receive descriptor), stderr carries the "chain1 form" coordinator note |
| E2 | same with `--chain 2` instead of `--change` | exit 0; the same stdout line |
| E3 | same with `--chain 0` (control) | exit 0; the same stdout line (correct) |
| E4 | `md address --template 'wpkh(@0/*)' --key @0=X --change` (control) | exit 1; stderr `md: codec error: chain index 1 out of range; use-site multipath alt-count is 1` |
| E5 | `md descriptor --template 'wpkh(@0/<0;1>/*)' --key @0=X --chain 2` (control) | exit 1; stderr `md: codec error: chain index 2 out of range; use-site multipath alt-count is 2` |
| E6 | `md descriptor --template 'wpkh(@0/*h)' --key @0=X --chain 1` | exit 0; stdout `wpkh(xpub661My...Fit1WpVZTmqZLzpvZG2s/*h)#twcwr4x7` |

## 2. Spec delta (behaviour)

S1. md-codec gains one private-to-crate helper in `crates/md-codec/src/to_miniscript.rs`:
`pub(crate) fn check_chain_in_range(d: &Descriptor, chain: u32) -> Result<(), Error>`. Its body is the existing
pre-flight of `derive.rs:108-132` moved verbatim: `baseline_alts` = the baseline multipath length or 1 when `None`;
`max_alts` = the fold of `baseline_alts` with every `use_site_path_overrides` entry's length (or 1); if
`chain as usize >= max_alts` return `Error::ChainIndexOutOfRange { chain, alt_count: max_alts }`; else `Ok(())`.
No new error variant, no new public item.

S2. `to_miniscript_descriptor_with_network` calls `check_chain_in_range(d, chain)?` as its FIRST statement, before
`expand_per_at_n`. Hence `to_miniscript_descriptor` (which delegates, `:72`) gets it too. The per-key bound in
`use_site_to_derivation_path` stays unchanged and still fails closed per key (E5's message keeps coming from
there when a key's own multipath is narrower than the widest).

S3. `Descriptor::derive_address` replaces its inline block (`derive.rs:108-132`) with
`crate::to_miniscript::check_chain_in_range(self, chain)?;` at the same position (after the hardened pre-flight).
`derive_address`'s error order and texts are unchanged.

S4. Mixed wallets keep the `md address` rule (decided by grade-md.md:51): when at least one key has a branch `N`, `--chain N`
renders, and a key without a multipath contributes its single path (`/*`) to that descriptor, exactly as
`md address --chain N` derives from it today. A wallet where NO key has branch `N` is refused.

S5. Error precedence on the descriptor route: the chain check runs before key expansion and before any hardened
check. So `wpkh(@0/*h) --chain 1` (E6) now refuses with the chain text, not the hardened text that `md address` prints
for the same input (address checks hardened first, S3). Both exit 1. `--chain 0` on `/*h` is unchanged (exit 0).

S6. md-cli, refusal path: no source change (S9 changes the verdict form only). `md descriptor` with `--chain N` (or `--change`, N = 1) where N is at least W, the
widest alt-count across all keys (1 when no key has a multipath), now prints, on stderr, exactly
`md: codec error: chain index N out of range; use-site multipath alt-count is W`
and exits 1 with nothing on stdout. The text and exit code are the ones `md address` already prints (E4). The
refusal happens at `descriptor.rs:204-208`. That is after the `--verify-against` verdict (`:167-183`) and the
`--emit md1` dispatch (`:188-197`), and before any warning, coordinator note or watch-only advisory. So `--json`
prints no envelope either. A run that passes `--verify-against` too still prints that verdict line before the
refusal (code read, not run); this FIX keeps that order.

S7. Doc comments (no behaviour): `error.rs:454-463` `ChainIndexOutOfRange` reads "...called with a `chain` index
at or beyond the widest use-site multipath alt-count across all keys (the baseline and every per-`@N` override; a
key with no multipath counts as 1)" and its `alt_count` field doc reads "The widest alt-count across all keys (`1`
when no key has a multipath component)" (today it says `0`, which no code path produces). `to_miniscript.rs:45-58`
(Errors list of `to_miniscript_descriptor`, inherited by `_with_network`) gains the bullet
"- [`Error::ChainIndexOutOfRange`] when `chain` is at or beyond the widest use-site multipath alt-count across all keys
(a key with no multipath counts as 1); checked before key expansion." `derive.rs:84-85` bullet reads "...out of
range for the widest use-site multipath across all keys (shared check with [`crate::to_miniscript_descriptor`])".

S8. Out of scope, unchanged: `--emit md1` still ignores `--chain`/`--change` (section 8, F2); `to_miniscript_descriptor_multipath*`
(takes no chain); `md vectors` (`vectors.rs:198` already `continue`s on Err for "single-path vectors have no chain
1". No committed vector is single-path: all 48 `*.conformance.json` carry a chain "1" entry with addresses, so
the corpus does not move).

S9. md-cli `crates/md-cli/src/cmd/descriptor.rs` (the coordinator-verdict block, the block ending at the `notice` call): the verdict form
names the spelling md printed. Before the `match args.chain` that picks the form, compute
`has_branches` = true when the baseline `use_site_path.multipath` or any `tlv.use_site_path_overrides` entry's
`multipath` has more than one alternative (the quantity `check_chain_in_range` calls `max_alts`, tested `> 1`).
When `has_branches` is false the form is `None` for every chain the command accepts (no `--chain`, `--chain 0`);
otherwise the existing mapping (None -> Multipath, 0 -> Chain0, 1 -> Chain1, other -> None) is unchanged.
Effect: for `wpkh(@0/*)` (and `wpkh(@0/<5>/*)`, a one-alternative multipath that renders `/5/*`) the notice header
reads `note: coordinators for this descriptor (verified versions only; newer: unmeasured):`, Core's line reads
`Bitcoin Core 24.2-31.1: unproven (not measured for this policy)`, and no line prints
`(use --chain 0 and --chain 1)`, whose `--chain 1` this FIX refuses on such a wallet (and base already refuses on
`<5>`). The no-chain and `--chain 0` runs, which print one identical descriptor, print one identical notice.
No new printed string, no registry, evidence or table change; stdout, `--json` and exit codes unchanged.
Mixed and override-only multipath wallets keep the multipath form.

## 3. Help, manual, CHANGELOG (exact texts, PROPOSED (architect confirms))

- Help (`md descriptor --help`, `main.rs:538-544`): unchanged. Man pages and the mnemonic-gui md schema mirror: unchanged (no
  flag or help change).
- CHANGELOG.md, under `## md-cli [Unreleased]` / `### Fixed`, append:
  "- **`md descriptor --change` refuses a wallet with no change branch.** On a wallet where no key has a multipath
  (for example `wpkh(@0/*)`), `--change` and `--chain N` (N > 0) printed the receive descriptor at exit 0. They now
  print `md: codec error: chain index 1 out of range; use-site multipath alt-count is 1` (with N and the widest
  alt-count) and exit 1, as `md address` already did. A wallet where at least one key has the branch still renders."
- Q1 CHANGELOG (architect item 3): CHANGELOG.md, under `## md-cli [Unreleased]` / `### Fixed`, after the entry above, append:

```text
- **`md descriptor` no longer gives a wallet without a change branch the multipath verdict.** For a wallet where no
  key has two or more branches (for example `wpkh(@0/*)`), the coordinator note named the "multipath form" and said
  Bitcoin Core 24.2-28.4 refuses "the <0;1> multipath spelling (use --chain 0 and --chain 1)", although the printed
  descriptor has no `<0;1>` and `--chain 1` is refused there. The note now names no form, as it does for a
  template, and Core reads `unproven`; `--chain 0` prints the same note, since it prints the same descriptor.
```

- CHANGELOG.md, under `## md-codec [Unreleased]` / `### Fixed`, append:
  "- **`to_miniscript_descriptor` / `to_miniscript_descriptor_with_network` refuse a chain no key has.** They now return
  `Error::ChainIndexOutOfRange` when `chain` is at or beyond the widest use-site multipath alt-count across all keys
  (the check `Descriptor::derive_address` already made, now shared); before, a key with no multipath ignored `chain`
  and every chain returned the chain-0 descriptor."
- Toolkit manual lockstep (mnemonic-toolkit master `b0b99a160f119fac20a0b398d275ef787d0e2140`,
  `docs/manual/src/40-cli-reference/42-md.md`): after line 199 (the paragraph ending "`--chain` collapses it."), add
  a new paragraph:
  "`--chain N` (and `--change`, which is `--chain 1`) is refused when no key in the wallet has an N-th branch — a
  wallet written `wpkh(@0/*)` has only chain 0. The refusal is the one `md address` prints,
  `md: codec error: chain index 1 out of range; use-site multipath alt-count is 1`, exit 1, with nothing on stdout."
  No other manual line changes (line 177's `--chain` row stays accurate).
- Q1 manual (architect item 4): toolkit manual lockstep, mnemonic-toolkit master `b0b99a160f119fac20a0b398d275ef787d0e2140`, `docs/manual/src/40-cli-reference/42-md.md`: replace lines 235-237 (from "the spelling it judged" to "refuse the other:"; line 234 keeps its last words "The header names") with:

```text
the spelling it judged — `multipath` by default, `chain0` / `chain1`
under `--chain` — because a coordinator can accept one spelling and
refuse the other. (A wallet where no key has two or more branches,
written `wpkh(@0/*)`, has none of these spellings: its header names no
form, as for a template, and every line is a rule refusal or
`unproven`.) For a multipath wallet:
```

## 4. TRACE rows (RED on the base; GREEN after the build)

New md-cli test file `crates/md-cli/tests/bh1119_md_change_branch.rs` (assert_cmd; each row asserts the exact
`(exit code, stdout, stderr)` triple unless noted). Two md-codec unit rows go into `derive.rs`'s `mod tests` (they
reuse `bip84_origin`, `one_test_xpub_bytes`, `none_baseline_override_change_descriptor`, and add one fixture
`single_chain_wpkh_descriptor`: n 1, Shared bip84 origin, baseline `multipath: None`, `Tag::Wpkh` on `@0`, pubkey 0 =
`one_test_xpub_bytes()`, no overrides). Let `OOR(n,w)` = `md: codec error: chain index n out of range; use-site multipath alt-count is w` + newline.

| Row | Input | Expected after build | Base (seen by the author) |
|---|---|---|---|
| r1_descriptor_change_on_single_chain_wallet_refused | `descriptor --template 'wpkh(@0/*)' --key @0=X --change` | (1, "", OOR(1,1)) | FAIL: (0, receive descriptor `#fn4fyxff`, warning + chain1 note + advisory) |
| r2_descriptor_chain2_on_single_chain_wallet_refused | same, `--chain 2` | (1, "", OOR(2,1)) | FAIL: exit 0, `#fn4fyxff` |
| r3_descriptor_change_json_on_single_chain_wallet_refused | same as r1 plus `--json` | (1, "", OOR(1,1)) | FAIL: exit 0, JSON envelope `"chain": 1` with the receive descriptor |
| `derive::tests::r4_to_miniscript_chain1_without_multipath_refused` (md-codec `derive::tests`) | `single_chain_wpkh_descriptor()`; `to_miniscript_descriptor_with_network(&d, 1, Bitcoin)` and `to_miniscript_descriptor(&d, 1)`; chain 0 control | both `Err(ChainIndexOutOfRange { chain: 1, alt_count: 1 })`; chain 0 `Ok` | FAIL at the first assert (Ok) |
| r5_descriptor_chain1_hardened_wildcard_single_chain_refused | `descriptor --template 'wpkh(@0/*h)' --key @0=X --chain 1` | (1, "", OOR(1,1)) | FAIL: exit 0, `#twcwr4x7` |
| c1_descriptor_chain0_on_single_chain_wallet_unchanged | r1 with `--chain 0` | exit 0, stdout exactly the `#fn4fyxff` line (stderr not pinned) | PASS |
| c2_descriptor_change_on_mixed_wallet_follows_address_rule | `descriptor --template 'wsh(multi(2,@0/<0;1>/*,@1/*))' --key @0=X --key @1=Y --change`; then `address` with the same args | descriptor exit 0, stdout exactly `wsh(multi(2,xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/1/*,xpub661MyMwAqRbcG5161axqKvt7Kx7XBe4pWwNMvgbwdffMtvzPnXA85ToGs3EtpEVAAYf9PggopL6xt7ySJw5Kc7ELWVcwopEjYXVHaHy6tFz/*))#4a8tvuad`; address exit 0 | PASS |
| c3_descriptor_chain2_counts_per_slot_override | `descriptor --template 'wsh(multi(2,@0/*,@1/<0;1;2>/*))' --key @0=X --key @1=Y --chain 2` | exit 0, stdout exactly `wsh(multi(2,xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/*,xpub661MyMwAqRbcG5161axqKvt7Kx7XBe4pWwNMvgbwdffMtvzPnXA85ToGs3EtpEVAAYf9PggopL6xt7ySJw5Kc7ELWVcwopEjYXVHaHy6tFz/2/*))#250az8uk` (fails a baseline-only rule) | PASS |
| c4_address_change_on_single_chain_wallet_unchanged | `address --template 'wpkh(@0/*)' --key @0=X --change` | (1, "", OOR(1,1)) | PASS |
| c5_descriptor_chain2_on_two_branch_wallet_unchanged | `descriptor --template 'wpkh(@0/<0;1>/*)' --key @0=X --chain 2` | (1, "", OOR(2,2)) | PASS |
| `derive::tests::c6_to_miniscript_override_change_chain_still_renders` (md-codec `derive::tests`) | `none_baseline_override_change_descriptor()`, `to_miniscript_descriptor_with_network(&d, 1, Bitcoin)` | `Ok` | PASS |

The author ran all 11 rows on the base (5 FAIL: r1-r5; 6 PASS: c1-c6). With a scratch helper per S1-S2 the same
11 rows all PASS. The full workspace suite (`--all-features`) under the scratch fix gave 1762 passed, 6 skipped,
0 failed. The scratch code is not on this branch. With S9 and the six Q1 rows added, the architect's full workspace run gave 1768 passed, 6 skipped, 0 failed.

Coverage map: S1/S2 -> r1, r2, r4; S2 network-less entry -> r4; S3 (address unchanged) -> c4, c5; S4 -> c2, c3, c6; S5 -> r5, c1; S6 (exact stderr, empty stdout, `--json`) -> r1, r2, r3; S8 corpus -> gate step 3; S9 -> q1_r1, q1_r2, q1_r3 (no form), q1_r4 (every printed `--chain N` runs), q1_c1, q1_c2 (multipath, mixed and override-only keep the multipath form).

Q1 rows (architect ruling, md-cli): the builder creates `crates/md-cli/tests/bh1119_md_single_path_verdict.rs` with exactly the Rust text below (rustfmt output; clippy `-D warnings` and `cargo fmt --check` clean) and implements S9 in `crates/md-cli/src/cmd/descriptor.rs`. Base 18528b50 (run by this review): q1_r1, q1_r2, q1_r3, q1_r4 FAIL; q1_c1, q1_c2 PASS; with S1-S2 and S9 all six PASS.

```rust
//! Bughunt-1119 #30 Q1: the coordinator verdict `md descriptor` prints for a
//! wallet where no key has a multipath names no spelling, so it never claims
//! the `<0;1>` refusal or prints its `--chain 1` remedy (refused there).

#![allow(missing_docs)]

use std::process::Command;

const X: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const Y: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";

const NO_FORM_BLOCK: &str =
    "note: coordinators for this descriptor (verified versions only; newer: unmeasured):
  Liana 8.0-15.0: refuses (legacy wrapper)
  Nunchuk 2.1.1: unproven (not measured for this policy)
  Bitcoin Core 24.2-31.1: unproven (not measured for this policy)
";

const WPKH_X_STAR: &str = "wpkh(xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/*)#fn4fyxff\n";

fn md(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        o.status.code().expect("md exited normally"),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn key(slot: &str, xpub: &str) -> String {
    format!("{slot}={xpub}")
}

#[test]
fn q1_r1_descriptor_single_path_no_chain_names_no_form() {
    let kx = key("@0", X);
    let (code, out, err) = md(&["descriptor", "--template", "wpkh(@0/*)", "--key", &kx]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, WPKH_X_STAR);
    assert!(err.contains(NO_FORM_BLOCK), "{err}");
    assert!(!err.contains("--chain"), "{err}");
    assert!(!err.contains(" form"), "{err}");
}

#[test]
fn q1_r2_descriptor_single_path_chain0_same_verdict_as_no_chain() {
    let kx = key("@0", X);
    let (code, out, err) = md(&[
        "descriptor",
        "--template",
        "wpkh(@0/*)",
        "--key",
        &kx,
        "--chain",
        "0",
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, WPKH_X_STAR);
    assert!(err.contains(NO_FORM_BLOCK), "{err}");
    assert!(!err.contains(" form"), "{err}");
}

#[test]
fn q1_r3_descriptor_single_alt_multipath_names_no_form() {
    let kx = key("@0", X);
    let (code, out, err) = md(&["descriptor", "--template", "wpkh(@0/<5>/*)", "--key", &kx]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "wpkh(xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/5/*)#q45wdwf5\n"
    );
    assert!(err.contains(NO_FORM_BLOCK), "{err}");
    assert!(!err.contains("--chain"), "{err}");
}

/// Every `--chain N` a no-chain run prints is accepted as printed.
#[test]
fn q1_r4_every_printed_chain_remedy_runs() {
    let kx = key("@0", X);
    let ky = key("@1", Y);
    let cases: &[&[&str]] = &[
        &["--template", "wpkh(@0/*)", "--key", &kx],
        &["--template", "wpkh(@0/<5>/*)", "--key", &kx],
        &["--template", "wpkh(@0/<0;1>/*)", "--key", &kx],
        &[
            "--template",
            "wsh(multi(2,@0/<0;1>/*,@1/*))",
            "--key",
            &kx,
            "--key",
            &ky,
        ],
        &[
            "--template",
            "wsh(multi(2,@0/*,@1/<0;1;2>/*))",
            "--key",
            &kx,
            "--key",
            &ky,
        ],
    ];
    for case in cases {
        let mut args = vec!["descriptor"];
        args.extend_from_slice(case);
        let (code, _, err) = md(&args);
        assert_eq!(code, 0, "{args:?}: {err}");
        for n in err.split("--chain ").skip(1) {
            let n: String = n.chars().take_while(char::is_ascii_digit).collect();
            let mut again = args.clone();
            again.extend(["--chain", n.as_str()]);
            let (code, _, err2) = md(&again);
            assert_eq!(code, 0, "printed remedy {again:?} failed: {err2}");
        }
    }
}

#[test]
fn q1_c1_descriptor_multipath_no_chain_unchanged() {
    let kx = key("@0", X);
    let (code, _, err) = md(&["descriptor", "--template", "wpkh(@0/<0;1>/*)", "--key", &kx]);
    assert_eq!(code, 0, "{err}");
    assert!(
        err.contains(
            "note: coordinators for this descriptor, multipath form (verified versions only; newer: unmeasured):
  Liana 8.0-15.0: refuses (legacy wrapper)
  Nunchuk 2.1.1: unproven (not measured for this policy)
  Bitcoin Core 24.2-28.4: refuses (the <0;1> multipath spelling (use --chain 0 and --chain 1))
  Bitcoin Core 29.4-31.1: unproven (not measured for this policy)
"
        ),
        "{err}"
    );
}

#[test]
fn q1_c2_descriptor_mixed_and_override_multipath_keep_multipath_form() {
    let kx = key("@0", X);
    let ky = key("@1", Y);
    for t in [
        "wsh(multi(2,@0/<0;1>/*,@1/*))",
        "wsh(multi(2,@0/*,@1/<0;1;2>/*))",
    ] {
        let (code, _, err) = md(&["descriptor", "--template", t, "--key", &kx, "--key", &ky]);
        assert_eq!(code, 0, "{t}: {err}");
        assert!(
            err.contains("note: coordinators for this descriptor, multipath form ("),
            "{t}: {err}"
        );
        assert!(
            err.contains(
                "  Bitcoin Core 24.2-28.4: refuses (the <0;1> multipath spelling (use --chain 0 and --chain 1))\n"
            ),
            "{t}: {err}"
        );
    }
}
```

## 5. Tests that change

None. The full suite passed unchanged under the scratch fix: `derive_address_chain_out_of_range`,
`derive_address_override_change_chain_derivable`, `derive_address_override_chain_over_max_still_rejects`,
`vector_corpus::vectors_output_matches_committed_corpus` and `proptest_to_miniscript` all pass. New: the 11 rows above and the six Q1 rows (17 in all).

## 6. FOLLOWUPS

None appended in the PR. The report lists the closing line for bh-md-I6 and the new line for F2 from section 8 (F1 closed inside this row; the architect dropped its proposed line).
They are pushed directly after the merge.

## 7. Gate line

The builder commits `scripts/fix-bh1119-md-change-branch-gate.sh`, on the pattern of
`scripts/fix-bh1119-md-placeholder-order-gate.sh`. Every step runs, and the script exits 1 naming the failed steps.
Its steps:
1. Every TRACE row
above by its exact nextest name, with match count 1 each: `cargo nextest list --locked -p md-codec --message-format oneline -E 'test(=derive::tests::<row>)'`
for r4 and c6, and `cargo nextest list --locked -p md-cli --message-format oneline -E 'test(=<row>)'` for the nine rows in
`crates/md-cli/tests/bh1119_md_change_branch.rs` and the six rows in `crates/md-cli/tests/bh1119_md_single_path_verdict.rs`; then run each.
2. The ci.yml set: nextest `--workspace --all-targets --all-features`, doctests, clippy `-D warnings`, fmt, the
   `--no-default-features` build, `cargo doc` with `-D warnings`, the grouping sha, the shared-ci sha,
   `ci/docs-only.test.sh`, `ci/code-docs-check.test.sh` and `ci/code-docs-check.sh`.
3. No golden moved: `git diff --exit-code $(git merge-base HEAD origin/main) -- crates/md-codec/tests/vectors design/display-grouping-vectors.tsv`.
It does NOT cover the bitcoind-differential, fuzz-smoke, freebsd and musl/aarch64 jobs, the toolkit-manual
PR, or the release.

## 8. Related defects found (not in this row; for the architect)

F1 (grade I, kind ux; RULED and folded into this row as S9, architect md30-q1): `md descriptor` with no `--chain` on a wallet where no key has two or more branches (`wpkh(@0/*)`) printed the "multipath form" note, including `Bitcoin Core 24.2-28.4: refuses (the <0;1> multipath spelling (use --chain 0 and --chain 1))` (registry.rs:330), although the printed descriptor has no `<0;1>` and its `--chain 1` is refused (after this FIX; `wpkh(@0/<5>/*)` already on base). S9 gives such a wallet no form; rows Q1 pin it.

F2 (proposed grade M, kind ux; code read, not run): `md descriptor --emit md1` returns at `descriptor.rs:188-197`
before `args.chain` is read, so `--chain`/`--change` are silently ignored on that form for any wallet. clap does not
refuse the combination (`main.rs:538-563`).
