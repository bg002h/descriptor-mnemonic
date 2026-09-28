Verdict: 0 Critical, 0 Important, 1 Minor, 3 Nit

# Whole-diff adversarial review: 3e898ae "md-codec 0.48.4: chunk::split_unadmitted"

Reviewer: independent post-implementation review (subagent), 2026-09-28.
Plan: `design/IMPLEMENTATION_PLAN_split_unadmitted.md` (R0 GREEN r2).
Scope: `git show 3e898ae` in full (9 files, +312/-9).

## Gates run (offline, against the committed `vendor/`)

Source replacement was passed as `--config` flags, copied from the
three-block `SRC_CONFIG` in `ci/repro/vendor-freshness.sh` (miniscript rev
`ff4732e5…` taken from Cargo.lock). The target dir was
`<repo>/target/review`, not `/tmp`. `cargo-nextest` is not installed on this
box, so the tests ran under `cargo test`, which gives the same pass/fail
coverage but runs serially.

| Gate | Result |
|---|---|
| `cargo metadata --locked --offline` + SRC_CONFIG (vendor-freshness) | OK |
| `cargo test --locked -p md-codec --all-features --no-fail-fast` | 0, 659 passed / 0 failed / 3 ignored |
| `cargo test --locked -p md-cli --all-features --no-fail-fast` | 0, 918 passed / 0 failed / 1 ignored |
| `cargo test --locked --workspace --all-targets --all-features` + `--doc` | 0, 1578 passed / 0 failed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0, clean |
| `cargo fmt --all --check` | 0 |
| `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --document-private-items --all-features` | 0, clean (stricter than CI) |
| `tests/split_unadmitted.rs` | 5/5 rows pass |

Every gate passed. Nothing in the diff would fail CI.

## Mutation check (does the test file actually test what it claims?)

I applied each mutant to `chunk.rs`, ran `--test split_unadmitted`, and
reverted with `git checkout`. The working tree is clean afterwards.

| Mutant | Rows that failed |
|---|---|
| `split_unadmitted` = `split_with(d, true)` | 4/5 (all but corpus) |
| `split` = `split_with(d, false)` | 2/5 (`accepts_f217_shape`, `accepts_sortedmulti_a_unspendable`: `expect_err` on split) |
| inverted `if admitted` branch | 4/5 |

Every mutant is killed. Rows 2 and 4 are real behavioural REDs in both
directions: they catch `split` losing admission, and they catch
`split_unadmitted` gaining it.

## `split` behaviour, bytes and error order

- The diff to `split`'s body is exactly the move into `split_with`, plus the
  `if admitted { encode_payload } else { encode_payload_unadmitted }` swap.
  The rest of the function (id derivation, sizing, 64-cap, header, loop) is
  byte-for-byte unchanged, as `git show` confirms. `split(d)` = `split_with(d, true)`
  goes through `encode_payload` → `encode_payload_inner(.., Enforce, None)`,
  the same call as before. Bytes, errors and error order are identical.
- `encode_payload_inner` runs the admission checks before any
  `BitWriter` write and gates them only on `admission`. The bytes do not
  depend on admission, so the claim "same strings whenever split succeeds"
  holds by construction. The corpus row checks it across 68 vectors, and
  most of them are multi-chunk (chunk counts 1 to 24).
- The chunk-set id still comes from `compute_md1_encoding_id`, which is
  non-admitting and unchanged, so both functions get the same id.
- Error-order doc claim: `split` refuses on admission before the cap
  check, and `split_unadmitted` reaches the cap. Correct. One nuance: the
  structural checks on placeholders, multipath and the tap tree run before
  admission, and writer-time structural errors run after it. The doc says
  only that "structural errors still surface", which is accurate.

## Doc comments, CHANGELOG, FOLLOWUPS, version, lock and pin

- The `split`, `split_unadmitted`, `split_with` and `encode_payload_unadmitted`
  docs are accurate. `encode_md1_string` does admit (`encode.rs:337`), so
  pointing mint callers there is correct. All intra-doc links resolve, and
  the doc build under `-D warnings` is clean.
- Version 0.48.4 appears in `crates/md-codec/Cargo.toml`, in `Cargo.lock`
  (only the md-codec stanza changed) and in md-cli's `=0.48.4` pin. No stray
  `0.48.3` pin is left anywhere outside the CHANGELOG history and design docs.
  xtask uses an unpinned path dependency. vendor-freshness is unaffected
  (path dependency) and was confirmed OK.
- The CHANGELOG entry sits above md-cli 0.20.3 and md-codec 0.48.3, is
  accurate, and says md-cli is not released.
- Both FOLLOWUPS entries are present. The companion entry names
  `mr-md-codec-unadmitted-split` as specified.
- The diff matches plan Change §1–§5 and Tests rows 1–5 with no scope creep.

## Findings

**M-1 (Minor): the refused shapes are only exercised as single-chunk sets.**
`contradictory_card()` and the kind-1 `sortedmulti_a` card each split to one
chunk. The multi-chunk path under `admitted = false` is never run on a
descriptor that `split` refuses. The risk is low: the loop is shared code,
the corpus row covers multi-chunk sets on the admitted path, and payload
bytes are provably independent of admission. Still, the motivating Refugium
case is a *chunked* re-emit, and no row pins "refused shape, N > 1 chunks,
round-trips". A cheap fix is a refused F-217 card with enough keys, or a
large TLV, to exceed 320 payload bits, asserting `chunks.len() > 1` and a
round-trip.

**N-1 (Nit): row 5 would pass vacuously on an empty `chunks`.**
`split_unadmitted_chunk_set_id_of_refused_shape` loops over `chunks` with no
non-empty assertion. It cannot happen today, because `count >= 1` and rows
2 and 3 assert non-empty on the same card, but an
`assert!(!chunks.is_empty())` would make the row self-contained.

**N-2 (Nit): `refugium-liana-key-per-member` has tier `cross-repo` and no `Companion:` line.**
The repo's cross-repo convention (CLAUDE.md) pairs entries with
`Companion:` lines. This one is a note/wont-fix, so it is harmless, but it
either needs a companion line or should use a non-cross-repo tier.

**N-3 (Nit): the two new FOLLOWUPS entries use a different format from their neighbours.**
They use the older bullet template (Surfaced/Where/What/Status/Tier), while
the entries just above them use the newer `**Filed …**` prose form. The file
already mixes both, so this is cosmetic only.

Pre-existing, not in the diff: `tests/common/vendored.rs` doc says the
corpus is 65 vectors, but it now holds 68. Only the test's own message says
68.

## Conclusion

No regressions found. `split` is unchanged in behaviour, bytes and error
order. `split_unadmitted` is correct, and its tests are non-vacuous (mutants
killed) with an enforced corpus floor (`compared >= 60`, and every iteration
panics on failure). Every CI-equivalent gate passes offline against
`vendor/`. The commit is fine to ship. M-1 is worth folding before the mr
work depends on multi-chunk re-emits.
