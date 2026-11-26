# descriptor-mnemonic final review (fable), 2026-11-26

Branch `claude/project-thread-utm1aa` at `1dac0e10`, against merge-base `c8cde234` (25 commits, 56 files).
A docs-only commit (`9912ffde`, FOLLOWUPS + the r6 report) landed on top while I worked; ignored.
Read-only: no commits, no pushes. Scratch files are in `scratchpad/fable-probes2/`, `scratchpad/fable-base/`
(a `git archive` of the base, for one build check) and `scratchpad/fable-*.log`.

**Counts: 0 Critical, 2 Important, 3 Minor, 1 Nit.**

**Verdict:** not ready to open as is. Neither Important finding touches decode, ids or addresses, and neither
puts funds at risk, but one makes CI red and breaks a documented command, and the other breaks a documented
build configuration of md-codec. Both are small to fix. Everything else I probed held.

## Important

### I-1. The bare-`@N` mint refusal breaks `md compile` → `md encode` and `md encode --from-policy`; 11 tests fail under CI's `--all-features`

**Where:** `crates/md-cli/src/parse/template.rs:196-221` (`refuse_placeholder_without_wildcard`, commit
`6604c88c`, r1 A2) against `crates/md-cli/src/compile.rs` (unchanged), `crates/md-cli/tests/cmd_encode.rs`
(unchanged), `crates/md-cli/README.md:58-85`.

**What happens.** `md compile` keeps the placeholders exactly as typed, and every documented example types
them bare. Its output is then not mintable:

```
$ md compile 'thresh(2,pk(@0),pk(@1),older(100))' --context segwitv0
wsh(thresh(2,pk(@0),s:pk(@1),snl:older(100)))
$ md encode 'wsh(thresh(2,pk(@0),s:pk(@1),snl:older(100)))'
md: unsupported: @0 has no use-site tail, which an md1 card cannot hold ...   (rc 1)
$ md encode --from-policy 'pk(@0)' --context tap
md: unsupported: @0 has no use-site tail ...                                  (rc 1)
```

Both worked at the base (a bare `@N` minted as `/*`). The tails do survive compilation, so
`--from-policy 'pk(@0/<0;1>/*)'` works today; only the bare spelling is broken.

**Why nobody saw it.** The `cli-compiler` tests are feature-gated. Every run quoted in the PR body is
`cargo test --locked --workspace` with default features. CI runs
`cargo nextest run --workspace --all-targets --all-features` (`.github/workflows/ci.yml:65,106`), and the
release builds ship `--features cli-compiler` (`release.yml:207,213`).

**Measured** (`cargo test --locked -j 2 -p md-cli --features cli-compiler`, logs
`scratchpad/fable-allfeat.log`, `fable-allfeat-unit.log`):
- `tests/cmd_encode.rs`: 8 failed of 37 — `encode_from_policy_segwitv0`, `encode_from_policy_inheritance_tap`,
  `encode_from_policy_thresh_2_of_3_tap`, `encode_with_explicit_path_populates_path_decl`,
  `encode_with_named_path_bip48`, `encode_path_overrides_canonical_default`,
  `encode_decode_roundtrip_thresh_2_of_3_tap_with_explicit_path`,
  `encode_decode_roundtrip_inheritance_pattern_with_explicit_path`.
- unit tests in `src/compile.rs`: 3 failed — `compile_caterpillar_shapes_still_reparse`,
  `compile_or_chain_four_keys_tap_reparses`, `compile_thresh_1_of_5_tap_reparses` ("compile emitted an
  unparseable template").
- All 11 fail with the A2 refusal text. The other six feature-gated test files pass. Clippy with
  `--all-features` is clean.

**Also affected:** the README compile examples (`crates/md-cli/README.md:61-84`) and the toolkit manual
(`42-md.md:455` `md encode --from-policy 'thresh(2,pk(@0),pk(@1),pk(@2))'`, and `:479`), which now fail as
written.

**Fix (the ruling stands; this is about the compiler path following it).** Either write the tails in the
tests, README and manual examples (`pk(@0/<0;1>/*)`), which already works, or have `md compile` /
`--from-policy` refuse a bare placeholder up front in its own words. Then run the suite once with
`--all-features` and quote that count in the PR body. The PR body's "0 failed" lines are true only for the
default feature set.

### I-2. md-codec no longer compiles with `--no-default-features`

**Where:** `crates/md-codec/src/policy_shape.rs:471` calls `crate::to_miniscript::tree_type_checks`
unconditionally; `to_miniscript` is `#[cfg(feature = "derive")]` (`crates/md-codec/src/lib.rs:45-46`), and
`miniscript` is an optional dependency behind that feature (`Cargo.toml`: `derive = ["dep:miniscript"]`).

**Measured.**
- HEAD: `cargo check --locked -p md-codec --no-default-features` → `error[E0433]: could not find
  to_miniscript in the crate root`.
- Base `c8cde234` (from `git archive`, same command): builds.

**Why it matters.** `crates/md-codec/README.md:117-126` documents this configuration for "pure-codec
consumers" (`default-features = false`). md-codec 0.49.0 would ship unable to build that way. No sibling
repo uses it today (I checked every `md-codec =` line in the constellation), and CI does not cover it
(`ci.yml:146` builds only md-cli without default features, which leaves md-codec's `derive` on), so it would
surface at a consumer.

**Fix needs one decision, not just a `cfg`.** Without miniscript the type rule cannot run. A no-`derive`
`policy_shape` must not return `complete == true` on the walk alone (that is the r2 I-1 defect again). So
either gate `policy_shape`, `skeleton` and `coordinator` behind `derive`, or make `policy_type_checks` return
`false` without it (no summary, no key). Add `cargo check -p md-codec --no-default-features` to CI either
way.

## Minor

### m-1. `md compile` can print, at exit 0, a template that md#4 now refuses

`md compile 'or(pk(@0/<0;1>/*),or(pk(@1/<0;1>/*),pk(@2/<0;1>/*)))' --context segwitv0` prints
`wsh(t:or_c(pk(@0/..),vc:or_i(pk_h(@1/..),pk_h(@2/..))))`, and `md encode` refuses it ("wraps a fragment
other than a single key in `c:`", `encode.rs:596-609`). This is not a regression in what can be backed up:
before md#4 the card minted and could never render. But a plain "any one of three keys" policy in `wsh` now
dead-ends between two md verbs with no hint at the compile step, and the refusal's advice
(`or_i(pk(..),pk(..))`) is not what the user should write (`thresh(1,…)`, which compiles to `multi(1,…)`).
Suggest a compile-side note, or naming `thresh(1,…)` in the refusal.

### m-2. Out of the diff, but the BIP text in this PR now contradicts it: seating does not resolve an elided keyless origin

The amended BIP says "On a keyless Template Card, a decoder MUST resolve a depth-0 origin … through the
corresponding canonical origin", and `md encode` still elides by default for a keyless canonical wrapper.
`md descriptor <that card> --from-mk1 …` does not resolve it: with
`md encode 'wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))'` and the two mk1 cards of
`fixtures/seating/v-ap-row1-e2e.txt` (declared at `m/48'/0'/0'/2'`), seating refuses with "Unfilled slots —
@0 [] (no fingerprint declared)"; the same template minted with `--path bip48` seats. With fingerprints,
`md inspect` shows the slot as `[73c5da0a/m]`. `crates/md-cli/src/seat/` has no reference to the canonical
origin. It fails closed (a refusal), and the seat code is untouched by this branch, so it is not a blocker,
but the default keyless mint is a card md's own seating cannot fill. Not in FOLLOWUPS; file it (it sits
beside the logged `bh1125-r1-n2-keyless-decode-shows-m`).

### m-3. The PR body's test section never ran what CI runs

Every quoted run is default-features. Given I-1, add the `--all-features` run (and, after I-2, the
`--no-default-features` check) and quote those counts. "fmt, clippy and doc are clean" does hold with
`--all-features` for clippy (I ran it).

## Nit

### n-1. PR body and FOLLOWUPS disagree on the legacy-render item

PR body, "Not fixed": md#1 legacy keyed card in `md descriptor` "Needs Brian's decision" with recommendation
(b). FOLLOWUPS `bh1125-r1-m8-descriptor-renders-legacy-empty-origin` records an owner direction (keep the
render plus the loud warning). Say the same thing in both.

## What I checked and found sound

- **Decode, ids, addresses of existing valid cards.** The md-codec diff touches only `policy_shape.rs`,
  `skeleton.rs`, `coordinator/{mod,registry}.rs` and one added function in `to_miniscript.rs`. Nothing in
  decode, identity, canonicalize or derive changed. `policy_type_checks` is consulted by `policy_shape`,
  `skeleton` and md-cli's `gate_invalid_miniscript` only.
- **No false "not valid miniscript".** 40 templates minted and passed through `md shape-key`
  (`scratchpad/fable-probes2/p.py`): single-sig wrappers, `sh(multi)` and `sh(sortedmulti)` at 15-of-15,
  `sh(wsh)`, `wsh(multi(20,…))`, `multi_a` with 32 keys, `sortedmulti_a`, Liana-style decaying trees, a
  four-leaf NUMS tree, hash locks, `andor`, every in-scope `thresh` spelling. None was refused as untyped.
  The only refusals were the designed out-of-scope `thresh` shapes (`IncompleteShape`).
- **The summary never overstates.** I re-derived the allowlist against BIP-379 (each listed child leaves
  exactly 0 or 1; `s:pkh` excluded because `pkh` is not "o"), walked the `any_of_root` / `collect` paths for
  `or_b`, `or_c`, `or_d`, `or_i`, `andor` X/Y/Z, `k == n`, duplicate slots, and checked the 19 any-of cases in
  `cases.json` by hand (`min_keys = k - min(k, locks)`). The `min(k+1, n)` statement in rule 6 matches
  `vendor/miniscript/src/miniscript/types/extra_props.rs:892-907`. Timelock ranges in rule 3 match
  `AbsLockTime::from_consensus` / `RelLockTime::from_consensus`.
- **Coordinator clauses and the verdict table.** Read every clause that touches `branches`; the unchanged
  ones (Nunchuk, Core, "no locked path", lock classes) stay true or silent for an any-of entry.
  `cargo xtask verdicts --check`: fresh (548 rows), and `table.rs` is not in the diff.
- **md#1 fill.** No-path keyed mint equals the `--path bip84` / `--path bip48` card byte for byte; a mixed
  template (one explicit origin, one filled) equals the fully explicit card and verifies; `sh(wsh)` with a
  `2'` key and non-canonical wrappers refuse and ask for `--path`; a tpub needs `--network` and then gets
  coin `1'`.
- **Read verbs.** `verify` uses the Warn parse, so neither the wildcard rule nor the dead-leaf rule runs;
  `needs_no_key` uses the probe; `--emit md1` requires mk1 seating input, so it never re-emits an existing
  card. All 36 seating fixtures give the same exit code with and without `--emit md1` except
  `v-spendeq-keyed` (pre-existing).
- **Feature builds.** `cargo check -p md-cli --no-default-features` builds; clippy `--workspace
  --all-targets --all-features -D warnings` is clean.
- **Tests run:** md-codec `policy_shape` 21, `policy_shape_cases` 2, `policy_shape_thresh` 7,
  `policy_shape_thresh_props` 2, `policy_shape_typecheck` 5, `coordinator` 11, `coordinator_thresh` 6,
  `skeleton_key` 14, all passing. The new test files I read (`bh1126_*`, the changed fixtures in
  `baseline_gapb`, `cli_keyed_excess_origin_note`, `cmd_verify`) assert what they say.

Not re-reported: every Minor and Nit of r1–r6 (gate order on keyless cards, manual and GUI mirror for the
new flag, the `policy_type_checks` "addresses never consult it" line, fill unannounced, and the rest).
