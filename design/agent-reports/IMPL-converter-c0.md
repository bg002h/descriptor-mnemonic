# IMPL report — converter C0 (wiring)

**Phase**: C0 of `design/IMPLEMENTATION_PLAN_wallet_form_converter.md` §3.
**Worktree**: `/scratch/code/worktrees/converter-c0`, branch `impl/converter-c0`,
off `main` at `7a3f7a68` (matches the dispatch brief's stated HEAD).
**Implementer**: single agent, TDD, no sub-dispatch (implementation-tight rule).
**Final commit**: `1967d17c` (three commits total, one per task; tree clean).

## What landed

1. **`mk-codec = "0.5"` as a registry dependency** (D1) —
   `crates/md-cli/Cargo.toml`, with a comment mirroring the existing
   `mnemonic-io-lib` registry-not-path rationale at the same file. Resolved to
   `mk-codec 0.5.0` (confirmed published via `cargo search mk-codec` before
   adding). `cargo update -p mk-codec` failed (`did not match any packages` —
   expected: it's not an *existing* lock entry to bump, it's brand new), so
   `Cargo.lock` was regenerated via a plain `cargo build` and then verified
   with `cargo build --locked -p md-cli` (exit 0). Commit `a029fb29`.

2. **`vendor/` refresh, same commit as the manifest change.** Determination:
   **vendor/ IS load-bearing in CI.** Evidence —
   `.github/workflows/vendor-freshness.yml` is a *leading* (PR-time) gate,
   triggered on any change to `Cargo.lock`, `Cargo.toml`, or
   `crates/**/Cargo.toml`, running `ci/repro/vendor-freshness.sh`, which
   resolves `Cargo.lock` against the committed `vendor/` tree under
   `--offline --locked`. Measured directly: the script passed before this
   change (baseline, `git stash` + run), failed immediately after adding the
   `mk-codec` line (`error: no matching package named 'mk-codec' found ...
   directory source .../vendor`), then passed again after `cargo vendor
   vendor` in the same commit. The delta is minimal: `git status --short
   vendor/` showed exactly one new untracked entry, `vendor/mk-codec/` (33
   files); every other vendored crate — including `bech32`, already a
   transitive dep — was untouched.

3. **Origin-notated `--key` value parser** (P1) —
   `crates/md-cli/src/parse/keys.rs` gains `OriginNotatedKey` and
   `parse_key_with_origin`, parsing `@i=[fp/path]xpub` (BIP-380 origin
   notation) or falling through to the existing bare `@i=XPUB` form via
   `parse_key`. **Parser only** — nothing outside the module calls it;
   today's `descriptor`/`address` `--key` handling is unchanged, per the
   brief. A new `CliError::BadOrigin { i, why }` variant carries a NAMED
   refusal ("`--key @{i}: origin notation {why}`"), verified to never contain
   "base58check decode" — the bare error the spec's motivation section
   (refusal 3) measured against this exact input class. Commit `d0152dac`.

   - Three malformed-origin classes, each a `v_keyorig_bad_*` test (row
     V-KEYORIG-BAD's unit half): bad fingerprint hex, unclosed bracket, empty
     path (a bare trailing slash right after the fingerprint — distinct from
     no slash at all, which BIP-380 permits as a fingerprint-only origin).
   - Accepted-form tests: fingerprint+path, fingerprint-only (no path),
     bare xpub (no bracket at all, same syntax `parse_key` already accepts),
     and **both hardened spellings** — `'` and `h`.
   - **Hardening choice, stated explicitly per the brief's request**: path
     parsing reuses `bitcoin::bip32::DerivationPath::from_str`, the same
     mechanism `parse::path::parse_path`'s literal-path fallback already
     relies on. Verified directly in the vendored `bitcoin-0.32.9` source
     (`ChildNumber::from_str`, `bip32.rs:227`):
     `let is_hardened = inp.chars().last().map_or(false, |l| l == '\'' || l == 'h');`
     — both spellings are accepted, and the file's own `template.rs` uses the
     identical `'`/`h` pair for multipath wildcard hardening. No new grammar
     was invented; this parser follows that precedent.
   - `OriginNotatedKey`, `parse_key_with_origin`, and `CliError::BadOrigin`
     carry `#[allow(dead_code)]`, the same pattern already on
     `CliError::Compile` in this file: under default features the plain `md`
     binary build never constructs them outside `#[cfg(test)]` until C1
     wires them in.
   - **Mutation-checked inline** (not part of the formal gate, done for
     confidence): temporarily narrowed the fingerprint check to only
     `fp_str.len() != 8` (dropping the hexdigit half). Re-ran
     `v_keyorig_bad_fingerprint_hex` — it went red, panicking at the now-
     unreachable `.expect("hexdigit-checked byte")` on the `"zzzzzzzz"`
     input. Reverted; full suite re-confirmed green afterward. The test is
     not vacuous.

4. **`seat/` module skeleton** (D2) — `crates/md-cli/src/seat/mod.rs`, wired
   via `mod seat;` in `main.rs`. Doc comment only, no engine code. Embeds the
   plan §1 matrix (the operator's MATRIX-TRAVELS directive's fourth copy).
   **Verified byte-identical** to the plan source:
   ```
   diff <(sed -n '27,55p' design/IMPLEMENTATION_PLAN_wallet_form_converter.md) \
        <(sed -n '12,40p' crates/md-cli/src/seat/mod.rs | sed 's|^//! ||; s|^//!$||')
   ```
   → empty diff (exit 0, no output). No `#[allow(dead_code)]` needed — an
   empty module with only doc comments triggers no lint. Commit `1967d17c`.

## Deviations from the brief

None. The brief's D1 pointer to "the in-file comment at lines 39-41" was
checked against `crates/md-cli/Cargo.toml` at HEAD `7a3f7a68`
(`git show 7a3f7a68:crates/md-cli/Cargo.toml | sed -n '39,41p'`) and is
accurate: line 39 is `# P3 — the shared constellation IO crate, taken from
the REGISTRY.` and line 41 begins the `NOT \`path =\`` rationale, exactly as
cited. The full comment block runs to line 53, `mnemonic-io-lib = "0.1.0"`
landing at line 54.

## Exit gate (all commands run against the final tree, commit `1967d17c`)

**`cargo nextest run --locked`** (full suite):
```
     Summary [   0.760s] 887 tests run: 887 passed, 2 skipped
```
(the 2 skipped are the pre-existing `cli-compiler`-feature-gated tests,
unaffected by this phase; default features unchanged)

**`cargo clippy --locked --all-targets -- -D warnings`**:
```
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.06s
```
(clean exit, zero warnings)

**`cargo fmt --check`**: clean (no diff output).

**Row-scoped gate — `cargo nextest run --locked -E 'test(v_keyorig_bad)'`**:
```
    Starting 3 tests across 72 binaries (886 tests skipped)
        PASS [   0.003s] (1/3) md-cli::bin/md parse::keys::origin_tests::v_keyorig_bad_unclosed_bracket
        PASS [   0.003s] (2/3) md-cli::bin/md parse::keys::origin_tests::v_keyorig_bad_empty_path
        PASS [   0.003s] (3/3) md-cli::bin/md parse::keys::origin_tests::v_keyorig_bad_fingerprint_hex
     Summary [   0.004s] 3 tests run: 3 passed, 886 skipped
```
**Matched count: 3, against expected count: 3** (the three malformed-origin
classes the brief named: bad fingerprint hex, unclosed bracket, empty path).
Non-empty, non-short — PASS per the gate's own rule (r2 M1: "nonzero" alone
is not sufficient; this is the count, quoted, against the expectation).

**`vendor-freshness` (`ci/repro/vendor-freshness.sh`)** — not part of the
brief's named exit gate list, but load-bearing per the D1 entry-gate
instruction and run to completion regardless:
```
vendor-freshness: resolving Cargo.lock against committed vendor/ (offline, locked; miniscript rev ff4732e5f75aa555682343cb180fa72ee3e8e9d5) ...
vendor-freshness: OK — vendor/ satisfies Cargo.lock.
```

## Commits

- `a029fb29` — `c0: add mk-codec 0.5 as a registry dependency, refresh vendor/`
- `d0152dac` — `c0: origin-notated --key value parser (@i=[fp/path]xpub), parser only`
- `1967d17c` — `c0: seating-engine module skeleton, doc-only, matrix embedded`
- this report, committed separately (repo convention: report lands as its
  own commit).

Working tree is clean at `1967d17c`; nothing left uncommitted or unstaged.
