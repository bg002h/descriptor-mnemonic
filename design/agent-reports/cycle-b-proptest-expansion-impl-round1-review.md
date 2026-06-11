# Implementation Review — Cycle B proptest expansion (round 1)

Reviewer: Fable 5 architect agent (ad6b868ef8018fbca), 2026-06-11.
Target: uncommitted Cycle B implementation @ 69b7a74 (common/mod.rs +771, proptest_roundtrip.rs +47, proptest_to_miniscript.rs NEW 812, FOLLOWUPS.md +12) against the R4-GREEN spec.
Persisted verbatim per CLAUDE.md convention BEFORE the fold.

## Verdict: YELLOW

0 Critical. 2 Important — both are **registry gaps, not code defects**; the code itself is spec-conformant, all implementer claims verified or exceeded. Both Importants are minutes-of-work FOLLOWUPS.md edits; fold them and this is GREEN without re-running the code review (no test or strategy change is implicated).

## Critical
- none

## Important
- **[I1] The upstream find has NO durable FOLLOWUPS entry — in either repo.** I independently reproduced it in a scratch crate with pure `miniscript = "=13.0.0"` + zero md-codec: `TapTree::combine(combine(a,b),c)` Displays as malformed `{{pk(a),pk(b),pk(c)}}`, miniscript's own `from_str` rejects it with `IncorrectNumberOfChildren { description: "taptree branch", n_children: 1, ... }`, and a correctly-written `{{a,b},c}` string parses Ok but re-Displays broken with the **same checksum** — every detail of the implementer's claim is true, including "Display is the faulty side." But the only durable records are the test comment on `upstream_taptree_depth2_display_asymmetry` (proptest_to_miniscript.rs:295–311) and the `t_tr_tree` doc comment (common/mod.rs:979–990). Test comments are not the registry: the re-enable action ("miniscript bump past the fix → restore the depth-2 T arm + invert the cell") and the report-upstream action (this repo has precedent: `external-pr-1-hash-terminals` → rust-bitcoin/rust-miniscript#935) both need a `design/FOLLOWUPS.md` entry (e.g. `upstream-miniscript-taptree-depth2-display-asymmetry`) citing the pin, the repro, the constrained generator arm, and the flip cell. File before commit.
- **[I2] The new `encode-accepts-k-greater-than-n` entry's Companion line asserts a toolkit-side companion that does not exist.** The entry reads "Companion: `mnemonic-toolkit` `design/FOLLOWUPS.md::encode-accepts-k-greater-than-n` (toolkit-side companion, filed by the Cycle-B orchestrator…)" — I grepped `/scratch/code/shibboleth/mnemonic-toolkit/design/FOLLOWUPS.md`: **no such entry**. The cross-repo convention (CLAUDE.md "Cross-repo follow-ups") requires the mirror in BOTH repos in lockstep, and as written the Companion line is a false citation the moment this commits. Either file the toolkit companion now (it is genuinely toolkit-relevant — bundle emit can engrave a k>n card no decoder will read) or reword the line to "to be filed".

## Minor
- `w_inner`'s doc comment claims "≤ 24 nodes (w2 worst case 21; pairing 23)" but doesn't count root wrappers: `sh(wsh(<23-node inner>))` = 25 tree nodes, one over the spec's ≤24 advisory. Harmless — the spec deliberately moved enforcement to the encoded-bits assert (≤18,000, present at common/mod.rs:733–742) and 25 small nodes is nowhere near it — but the comment should say "≤ 25 incl. root wrappers" so a future reader doesn't trust the stale arithmetic.
- Sole-child `wsh(sortedmulti)` / `sh(sortedmulti)` (the one to_miniscript-SUPPORTED sortedmulti shape) never flows through P6 — the T grammar omits it, conforming to the spec's grammar as gated, and derive coverage exists in address_derivation.rs (`wsh_sortedmulti_2_of_3_address`, `sh_wsh_sortedmulti_2_of_3_address`). Recorded as a future-T-arm observation, not a deviation.
- W anti-vacuity asserts SortedMulti tag presence but not the under-combinator *pairing* specifically; the deterministic P7 cell `self_test_bad_sortedmulti_under_combinator` pins the Cycle-A shape, so coverage is anchored. Observation only.

## Spec-conformance table

| Spec item | Conforms? | Notes |
|---|---|---|
| 349 md-codec / 537 workspace tests; fmt; clippy | YES | Re-ran all three: 349 (sums per-target: 210+21+10+11+1+9+9+1+4+1+10+33+8+21), 537 workspace, `fmt --check` clean, `clippy --workspace --all-targets -D warnings` clean |
| W: full-domain leaves + 11 boundary timelocks | YES | `W_BOUNDARY_TIMELOCKS` matches spec list exactly; `any::<u32>()` arm; full `[u8;32]`/`[u8;20]` hash domains; True/False/RawPkH present |
| W: ≥1-key guarantee in EVERY arm | YES | `w_inner`: all 5 arms pair a `w_keyed_leaf`; `w_tr`: arms 1/2/5 non-NUMS internal key, arms 3/4 designated `tap_keyed` leaf — no NUMS+keyless path exists |
| W: node budget + encode-assert ≤18,000 | YES (Minor) | Assert in final `prop_map` on `canon(d)`; 25-node sh(wsh) edge vs the 24 advisory (comment-only) |
| W: n≤8 with pubkeys/fingerprints TLVs | YES | `(max_idx,max_len)=(7,8)` when `mode.pubkeys \|\| mode.fingerprints` |
| W: TLV randomization, non-empty paths, valid points | YES | `w_origin_override_path` 1..=3 components; pubkeys from the T-tier `test_xpubs()` pool; Shared/Divergent randomized; anti-vacuity asserts all three TLVs + both decl kinds appear |
| W: SortedMulti-under-combinator + RawPkH generated | YES | `w_keyed_leaf` multikeys tags include SortedMulti under `w_level`; RawPkH in `w_keyless_leaf`; both asserted in anti-vacuity |
| W: taptree leaves avoid §6.3.1 forbidden TOP tags | YES | Tap leaf pools are tap_keyed/keyless/combinator-rooted by construction; Multi/SortedMulti only INSIDE combinator leaves (decode-permitted per spec) |
| T: context-split Bdu (tap keys-only + or_d) | YES | tap `bdu0` = pk/pk_h/multi_a only; segwit/legacy Bdu include hashlocks; `or_d(Bdu,Bdu)` recursion in both |
| T: W = s:pk only + a:\<single-node Bdu leaf\> | YES | Every `w0` (segwit/legacy/tap) = `Swap(PkK)` \| `Alt(bdu0)`; matches the [I1″] fold exactly |
| T: tap locks/hashes only under v: in and_v | YES | `vfirst` consumed solely inside `and_v(Verify(·), sig-B)`; productions match round-3's 22-shape proven set; 256-case P6 + 2048-sample anti-vacuity empirically close it |
| T: per-descriptor timelock-class XOR, rel+abs ok | YES | `(rel_time, abs_time)` chosen once via `prop_flat_map`, threaded to all three contexts |
| T: domains incl. 0x10000 / 0x00410000 | YES | In `t_older_value` select lists AND asserted present in `t_generator_covers_all_fragments` |
| T: caps (n≤16; Legacy ≤6 keys; Segwit ≤16-multi; tap multi_a≤16) | YES | `assert!((1..=16).contains(&next))` in the strategy map; per-context worst cases hand-checked (legacy 6, segwit 15+wide_multi 16, tap 13/16) |
| T: @i at-most-once per tr descriptor | YES | `assign_sequential_indices` gives globally fresh indices (stricter: all contexts) |
| T: NO prop_filter | YES | Only prop_filter hits are the pre-existing `descriptor_strategy` (lines 281–302); T/W use `prop_flat_map` for k≤n |
| P6: 4-step oracle, exact order, no filtering | YES | `p6_chain` = converter-must-succeed → payload+string+chunk round-trips → `from_str` fixed-point with PartialEq → derive_address differential |
| P7: all spec'd classes, clean Err + exact wire | YES | SortedMultiA (wsh+tap), RawPkH, SortedMulti-under-combinator, shape-C Check, after{0,≥2³¹}, older{0,bit-31} (cells + full-range properties), 21..=32-key multi in BOTH contexts |
| P8: encode clean-Err + loud k>n pin + FOLLOWUP | YES | 2 properties + 4 deterministic cells + `p8_encode_accepts_k_greater_than_n_decode_rejects` (encode Ok → decode `KGreaterThanN{k:3,n:2}` → string form too); FOLLOWUP entry filed md-codec-side |
| Anti-vacuity real | YES | Explicit full lists: W 34/36 Tag variants (omits only Wpkh/Pkh — not in W grammar by spec), T = exact spec grammar; `TestRunner::deterministic()`; 1024/2048 samples; boundary constants asserted; golden literals are hard-coded |
| Golden cells pin literals; older-leniency cites v0.53.9 | YES | **Independently re-derived all 4 verifiable goldens with pure bitcoin+miniscript from the published seed — byte-exact matches** (wsh/and_v, wsh/andor, tr/NUMS+sha256, older(0x10000)); both leniency cells cite toolkit v0.53.9 |
| Upstream find: pure, correct flip, minimal deviation | YES code / NO registry | Repro verified upstream-pure (scratch, deleted); flip direction correct (`is_err` + "UPSTREAM FIXED?" message); depth≤1 is minimal — only ≥3-leaf taptrees lose P6-step-3 (upstream-impossible), and the characterization cell keeps converter+derive+wire coverage for depth-2; **durable FOLLOWUP missing → [I1]** |
| FOLLOWUPS format + live citations | PARTIAL | Format matches neighbors; tree.rs:90–121/229–231/241–243 all verified live (KGreaterThanN at :230/:242); **Companion line cites a nonexistent toolkit entry → [I2]** |
| Hygiene: vacuous tests, copy-paste, regressions files, prints, #[ignore], wall-clock | YES | P1/P2/P4/P5(W) faithful clones; no println/dbg/#[ignore]; no proptest-regressions dirs; default 256 cases (no config override, per spec); wall-clock: proptest_roundtrip 0.12s, proptest_to_miniscript 1.64s — negligible CI cost |

## Evidence log
- Tree state at review start/end: identical (3 modified + 5 untracked as handed off); HEAD 69b7a74. Scratch crates `/tmp/upstream-repro` and `/tmp/golden-verify` created and deleted; nothing committed.
- `cargo test -p md-codec`: 349 passed, 0 failed, 0 ignored (per-target sums). `cargo test --workspace`: 537 passed. `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- Upstream repro (pure `miniscript =13.0.0`, generator-multiple keys): `combine(combine(a,b),c)` → Display `tr(K,{{pk(a),pk(b),pk(c)}})#x5ap4nu5` → `from_str` Err(`IncorrectNumberOfChildren{"taptree branch", n_children:1, min:2, max:2}`); correct string `{{a,b},c}` parses Ok, re-Displays the same broken form with the same checksum, which again fails reparse. Matches the test comment verbatim.
- Golden independence: derived m/86'/0'/{0,1}'/0/0 pubkeys from the published abandon-mnemonic seed, built the four descriptor strings with pure miniscript, `at_derivation_index(0).address(Bitcoin)` → `bc1qjrek53x…6xs44a`, `bc1qg0snqky…r406t2`, `bc1psldl66p…qp5l03l`, `bc1qcj2atyh…spfzaze` — all four equal the hard-coded literals.
- Registry greps: no taptree-depth2/display-asymmetry entry in either repo's FOLLOWUPS.md; no `encode-accepts-k-greater-than-n` in mnemonic-toolkit's FOLLOWUPS.md.
- Tag enum enumerated (36 variants, tag.rs:15–89): W list omits exactly {Wpkh, Pkh}; T list = spec grammar.
- Round-3 R0 evidence log cross-checked: every t_tap_leaf production maps to a proven shape in the 22-shape adversarial walk (or to a golden self-test cell: `self_test_tr_thresh_pkh_swap_pk_leaf`, `self_test_tr_and_b_pk_alt_pkh_leaf`).

**Path to GREEN:** file the two FOLLOWUPS entries — (1) `upstream-miniscript-taptree-depth2-display-asymmetry` in this repo (+ consider the toolkit mirror and an upstream report per the #935/#936 precedent), (2) the toolkit-side `encode-accepts-k-greater-than-n` companion (or reword the Companion line). No code changes required; a documentation-only re-check suffices for round 2.
