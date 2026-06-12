# R0 Review — md-codec k≤n encoder gate (PLAN_encode_k_le_n_gate.md) — Round 1
Reviewer: Fable 5, 2026-06-12. Verified against descriptor-mnemonic origin/main 31e5895 (working tree confirmed identical: `git status -sb` shows only the untracked plan-doc). Companion toolkit citations verified against mnemonic-toolkit origin/master 6899670.

## Verdict: GREEN (0C/0I)

The plan is sound, correctly grounded, and complete. Every load-bearing claim verified true against live source. The proposed gate is the exact symmetric mirror of the decode-side rejects, the ordering decision is not merely desirable but **already test-pinned** (see Minor M5 — a strength the plan undersells), and the test plan's Variable-arm cell was traced end-to-end and confirmed to reach the gate (not vacuous). Four minor citation/ritual nits should be folded; none blocks implementation.

## Critical (must fix before any code)
none

## Important (must fix before GREEN)
none

Key adversarial checks that came back clean:

1. **Variable-arm test reachability (the requested CRITICAL CHECK) — VERIFIED NON-VACUOUS.** Traced `descriptor_from_tree(wrap(Tag::Wsh, thresh_node(3, [keyarg(PkK,0), keyarg(PkK,1)])), true)` → `encode_payload` through every pre-`write_node` step at `encode.rs:65-88`: (a) `canonicalize_placeholder_indices` only permutes indices — @0 before @1 is already canonical, no-op; (b) `validate_placeholder_usage` (`validate.rs:40-80`) checks ONLY index-range and usage (indices 0,1 vs n=2 from `renumbered` — passes), never k; (c) `validate_multipath_consistency` skipped (`TlvSection::new_empty()` → no overrides); (d) Tr-taptree check skipped (root tag is Wsh). `write_node` then recurses Wsh→`Body::Children`→Thresh→`Body::Variable` arm, where k=3 passes `1..=32` and children.len()=2 passes `1..=32`, reaching exactly the proposed insertion point. There is no miniscript typecheck anywhere on the encode path (the existing P8 cell proves the analogous MultiKeys path encodes k>n today). `thresh_node` confirmed to build `Tag::Thresh`/`Body::Variable` (`common/mod.rs:61-66`). All helper imports already present (`proptest_to_miniscript.rs:19-23` imports `wrap`, `keyarg`, `multikeys`, `thresh_node`, `descriptor_from_tree`).
2. **No existing test regresses.** The ONLY k>n encode in the entire repo is the P8 characterization cell (`grep KGreaterThanN` across both crates: only `tree.rs`, `error.rs`, and the P8 cell). All proptest strategies constrain k ≤ count: `descriptor_strategy`'s four multisig arms `prop_filter("k<=n")` (`common/mod.rs:283-297`); `tr_multi_a` filters `k < n` against `(1..n)` = n−1 indices (`:301-304` — correctly k ≤ len); `w_multikeys` and all four W/T thresh sites draw `k in 1..=len` (`:510, :540, :575, :815, :852, :871, :912, :962`); P7 draws `k in 1..=20` vs `n in 21..=32` (`proptest_to_miniscript.rs:518-526`). All md-cli fixtures are k≤n (max is `multi(3,@0,@1,@2)` = 3-of-3 at `parse/template.rs:143`).
3. **Type lineup.** `Error::KGreaterThanN { k: u8, n: usize }` (`error.rs:108-114`) — plan's `k: *k` (u8 deref) and `n: children.len()/indices.len()` (usize) match exactly; decode uses the same `k as usize > count` comparison with `n: count` (usize). Symmetric.
4. **One gate covers all three doors.** `encode_payload` → `write_node` at `encode.rs:88`; `encode_md1_string` → `encode_payload` at `encode.rs:115` (fn at :114); `chunk::split` → `encode_payload` at `chunk.rs:240` (inside `pub fn split`, :236). No other wire-emit path exists.
5. **Edge ordering.** Gate after the range checks ⇒ k=0 → `ThresholdOutOfRange` first; empty children/indices → `ChildCountOutOfRange` first (existing cells at `proptest_to_miniscript.rs:583, :603` pin the empty case); k=n accepted (`>` not `>=`), pinned by the plan's cell 3. Decode-side k,count are 5-bit+1 reads (always 1..=32), so decode's sole check being k>count is structurally forced — plan's Q2 framing is accurate.
6. **SemVer PATCH defensible.** Every newly-rejected payload was already undecodable (decode rejects k>count unconditionally and 5-bit fields cap both at 32 — no encodable k>n payload can decode). No downstream relies on k>n encode succeeding: toolkit gates k≤n before reaching md-codec (spot-verified live: `pre_check_threshold` at `bundle_unified.rs:67`, `synthesize.rs:790-794` threshold range check), md-cli has no k>n fixture. Matches the 0.35.1 PATCH precedent.
7. **Toolkit tail.** `crates/mnemonic-toolkit/Cargo.toml:36` = `md-codec = "0.35"` ✓ semver-compatible → lockfile-only, as the plan says. Both FOLLOWUP entries exist (`descriptor-mnemonic design/FOLLOWUPS.md:1915` primary, `mnemonic-toolkit design/FOLLOWUPS.md:4013` companion); the primary's flip protocol matches the plan's cell-1 invert. No clap surface change ⇒ no manual/schema_mirror/quickstart lockstep — confirmed (pure library validation).

## Minor (optional / note)
- **M1 — MultiKeys insertion-point off-by-one.** Plan §2 says "insert after `:114`, before `:116`" and cites the range pair as `:108-114`. The closing brace of the `ChildCountOutOfRange` if-block is at **`tree.rs:115`**; the range pair spans `:108-115`. Correct insertion is "after `:115`, before `:116`". Following the citation literally places the gate inside the if-block (instant compile error — self-catching), and the prose intent ("after the existing 1..=32 range checks") is unambiguous, hence Minor not Important. Fold the line numbers. (The Variable-arm twin, "after `:99`, before `:100`", is correct.)
- **M2 — fmt gate: now verified, drop the conditional.** Plan §3 says "`cargo fmt --check` (if the repo gates fmt — verify)". Verified: `.github/workflows/ci.yml:49-57` runs a dedicated `cargo fmt --all --check` job. Make the fmt check unconditional in the GREEN gate.
- **M3 — tag convention: 0.35.1 is confirmed untagged; recommend tagging 0.35.2 anyway.** `git tag` has `md-codec-v0.35.0` and every prior release back to v0.3.0, but no `md-codec-v0.35.1` (0.35.1 shipped at `762a4f8`, crates.io-published, untagged). "Match whatever 0.35.1 did" would mean NOT tagging — but the 0.35.1 gap reads as an oversight against a ~30-tag convention, not a new policy. Recommend: tag `md-codec-v0.35.2` per the dominant convention, and optionally back-tag `md-codec-v0.35.1` at `762a4f8` in the same push. Surface the choice to the user at release time.
- **M4 — md-cli inherits a user-visible behavior change; note it in the CHANGELOG entry.** md-cli's template path (`parse/template.rs` lexes `multi(k,@0,…)` itself, not via rust-miniscript) will now refuse k>n input with `KGreaterThanN` instead of emitting an unrestorable card — that is the fix working, and no md-cli test/fixture relies on k>n (verified, answers plan Q4: clean). The ms-cli `=0.4.x` no-version-bump precedent covers the pin mechanics, but the CHANGELOG `[0.35.2]` entry should add one line noting md-cli's encode surface inherits the refusal.
- **M5 — strengthen §7 Q2's answer: the ordering is already test-pinned, not just preferred.** `p8_encode_rejects_out_of_range_multi_k` (k∈{0, 33..}, len 1..=8) and `p8_encode_rejects_out_of_range_thresh_k` (k≥33, 2 children) at `proptest_to_miniscript.rs:531-569` both assert `ThresholdOutOfRange` for inputs that are ALSO k>n. If the gate were inserted before the range checks, both existing cells would go red. So gate-after-range is enforced by the existing suite — worth stating in the plan as the anti-regression backstop.
- **M6 — hairline header nit.** "two docs-only commits since" dbdacfb: log shows ONE commit since (`6899670`, docs-only). The load-bearing part (no code drift; toolkit gate citations live at 6899670 — re-verified) holds.

## Answers to the plan's R0 questions
1. **write_node-only: CONFIRMED.** `validate_placeholder_usage` checks a different invariant and is itself invoked BY `encode_payload` (`encode.rs:69`) — a second k≤n copy there would be a drift-prone duplicate on the same path. The residual gap (an API user constructing a k>n `Node` and never encoding) is acceptable: the hazard is wire-emit ("engrave-but-can't-restore"), and every wire emit transits `write_node`. No public `validate()` contract promises k≤n today.
2. **Ordering: CONFIRMED** — and already pinned by existing tests (M5).
3. **Test count: the 3 deterministic cells suffice.** All existing strategies generate k≤n (verified exhaustively, see clean-check 2), so a dedicated k>n property adds little; a chunk::split-door cell would be redundant (transits `encode_payload`). Agree with the plan's lean.
4. **md-cli: verified clean** — no fixture/golden/test encodes k>n; the new refusal is user-visible but desired (M4: one CHANGELOG line).

## Citation audit
| Plan citation | Verdict | Live |
|---|---|---|
| `tree.rs:79` `write_node` | ACCURATE | :79 |
| Variable arm `:90-104`; k range `:92-94`; count range `:95-99`; insert after `:99` before `:100` | ACCURATE | all match |
| MultiKeys arm `:106-121` | ACCURATE | :106-121 |
| MultiKeys range pair "`:108-114`"; insert "after `:114`, before `:116`" | **DRIFTED** (off-by-one) | pair is :108-115; closing `}` at :115; insert after :115 (M1) |
| Decode rejects `:229-231` (multi-family), `:241-243` (thresh) | ACCURATE | exact, incl. `Error::KGreaterThanN { k, n: count }` |
| `error.rs:108` `KGreaterThanN { k: u8, n: usize }` | ACCURATE | :108-114, message string matches |
| `encode.rs:88` (encode_payload→write_node), `:115` (encode_md1_string→encode_payload), `chunk.rs:240` (split→encode_payload) | ACCURATE | :88, :115 (fn at :114), :240 |
| P8 cell `proptest_to_miniscript.rs:654`; doc-comment `:642-652` | ACCURATE | fn at :654, `///` block :642-652 |
| Helpers `common/mod.rs`: `wrap` :31, `keyarg` :37, `multikeys` :43, `thresh_node` :61, `descriptor_from_tree` :234 | ACCURATE | all five match; thresh_node = `Tag::Thresh`/`Body::Variable` |
| `validate.rs` `:62`/`:67` Variable/MultiKeys arms | ACCURATE | :62, :67 (placeholder-walk only, no k check) |
| `md-cli/Cargo.toml:28` `version = "=0.35.1"` | ACCURATE | :28 exact |
| md-codec `Cargo.toml` version 0.35.1 | ACCURATE | :3 |
| `git tag` shows only `md-codec-v0.35.0` (0.35.1 untagged) | ACCURATE | confirmed; 0.35.1 = `762a4f8`, no tag (M3) |
| toolkit `Cargo.toml:36` `md-codec = "0.35"` | ACCURATE | :36 |
| toolkit gates `bundle_unified.rs:67-90`, `synthesize.rs:790-794` | ACCURATE | live at 6899670 |
| toolkit origin/master `6899670`, "two docs-only commits since dbdacfb" | DRIFTED (trivial) | SHA correct; ONE docs-only commit since (M6) |
| FOLLOWUP entries (md-codec primary :1915, toolkit companion :4013) | ACCURATE | both present, flip protocol matches |
| Root `CHANGELOG.md` md-codec-sectioned PATCH convention | ACCURATE | `[0.35.1] — 2026-06-11` entry at :7 matches the proposed format |

**Gate disposition: GREEN — implementation may begin.** Fold M1-M4 (mechanical, no re-review required for these specific folds since none alters the design; if any fold goes beyond the stated corrections, re-dispatch per convention).
