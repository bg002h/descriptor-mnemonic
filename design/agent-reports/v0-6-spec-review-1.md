# v0.6 spec review (round 1) — strip Layer 3 (signer-compatibility curation)

**Status:** DONE_WITH_CONCERNS
**Commit:** N/A — review of unstaged spec at `design/SPEC_v0_6_strip_layer_3.md` (commit `8e652b1`)
**File(s):**
- `design/SPEC_v0_6_strip_layer_3.md`
- `design/MD_SCOPE_DECISION_2026-04-28.md`
- `design/FOLLOWUPS.md`
- `design/agent-reports/README.md`
- `crates/md-codec/src/bytecode/tag.rs`
- `crates/md-codec/src/bytecode/encode.rs`
- `crates/md-codec/src/bytecode/decode.rs`
- `crates/md-codec/Cargo.toml`
- `bip/bip-mnemonic-descriptor.mediawiki`
**Role:** reviewer (spec)

## What was reviewed

The v0.6 design spec proposing to strip MD's Layer 3 signer-curation: encoder/decoder default-validator removal; Tag enum reorganization with a new `Tag::SortedMultiA` (0x0B) and removal of the 14 `Reserved*` variants 0x24–0x31; BIP draft MUST→MAY rewrite at line 547 and a new §"Signer compatibility (informational)"; +10 positive corpus vectors; family-stable SHA reset at v0.5.x → v0.6.0. Reviewed against the eight numbered items in the dispatch prompt plus the five §12 open questions. Cross-checks performed against the current Tag enum, encoder, and decoder in tree.

## Critical / important findings

### CRIT-1 (cross-check #2 / Q2): §4.3 audit claim is inaccurate

§4.3 says "Most of these decoder arms ALREADY EXIST in `decode_tap_terminal` from Phase D". Reading `decode_tap_terminal` at `crates/md-codec/src/bytecode/decode.rs:626-730`, the **existing** Tap arms are:

`Tag::PkK`, `Tag::PkH`, `Tag::MultiA`, `Tag::Older`, `Tag::AndV`, `Tag::OrD`, `Tag::Check`, `Tag::Verify`, plus a defensive `Tag::TapTree` rejection arm and a catch-all returning `TapLeafSubsetViolation`.

That's 8 in-Tag-set arms. Every other Tag in §4.3's table is **absent** from `decode_tap_terminal` and must be added: `SortedMultiA` (NEW), `Sha256`, `Hash256`, `Ripemd160`, `Hash160`, `After`, `AndB`, `AndOr`, `OrB`, `OrC`, `OrI`, `Thresh`, `Alt`, `Swap`, `DupIf`, `NonZero`, `ZeroNotEqual`, plus `RawPkH`, `True`, `False`. That is ~18-20 new arms — not "most already exist".

The Segwitv0 dispatcher `decode_terminal` (lines 324-583) HAS all of these arms; the spec author may have conflated the two functions. The implementations from `decode_terminal` can be largely copy-adapted (read, recurse via `decode_tap_miniscript` instead of `decode_miniscript`, return `Terminal<_, Tap>`), so the work is mechanical, but it is real new code, not "remove the catch-all". The spec should be updated to make this clear before the implementation plan is written, otherwise the plan will under-scope the decoder phase.

**Fix:** Replace §4.3's "most decoder arms ALREADY EXIST" sentence with a more accurate statement, e.g.:

> The current `decode_tap_terminal` covers only the Phase D Coldcard subset (PkK/PkH/MultiA/Older/AndV/OrD/Check/Verify) and falls through to a TapLeafSubsetViolation catch-all for everything else. v0.6 adds explicit arms for every Tag listed in the table below (~20 new arms), modeled on the Segwitv0 dispatcher in `decode_terminal` but recursing via `decode_tap_miniscript` and producing `Terminal<_, Tap>`.

### CRIT-2 (cross-check #3 / Q2): Encoder catch-all — exhaustiveness IS achievable, but the spec under-specifies tap-illegal Terminal variants

The pinned miniscript revision is `apoelstra/rust-miniscript` rev `f7f1689b...` (see `Cargo.toml:38`). Per the existing in-tree encoder commentary at `encode.rs:590-594` and the `#[allow(unreachable_patterns)]` guards at lines 378, 590, and 637, `Terminal` is **NOT** `#[non_exhaustive]` in the pinned revision — the compiler proves the fallback wildcards unreachable. So the spec's §3.2 claim "exhaustive match" is **technically correct** for now. (Note: this property is fragile across miniscript upgrades; the CI rustdoc-deny-warnings check would catch a future regression but the FOLLOWUPS already track the apoelstra PR.)

However: an exhaustive match on `Terminal<DescriptorPublicKey, Tap>` requires arms for **every** Terminal variant — including ones that are tap-illegal by miniscript typing (`Terminal::Multi`, `Terminal::SortedMulti`) and Segwitv0-only constants/wrappers that simply never reach a tap-context dispatcher in practice (`Terminal::True`, `Terminal::False`). The full variant set is the 30-entry list in `terminal_to_tag` (encode.rs:604-639). The spec doesn't say what those arms should DO. Three options:

(a) Emit the wire byte unconditionally for any in-Tag-set Terminal (relying on miniscript's parser to refuse to construct tap-illegal ASTs upstream — coherent with the new "format is neutral" framing).
(b) Reject Multi/SortedMulti with a precise diagnostic (defensive — these CAN'T reach the dispatcher today but a future hand-built AST could).
(c) Return a generic `UnsupportedOperator` for Multi/SortedMulti.

§12 Q2 leans toward (a) "exhaustive over all in-Tag-set Terminals (preferred)". Recommend the spec land on option (a) explicitly and note in §3.2 that even tap-illegal Terminal variants get arms (Multi, SortedMulti) producing the wire byte, since miniscript's parser is the upstream gate. If the user prefers defense in depth, option (b) for Multi/SortedMulti only is also reasonable — but pick one and write it down.

**Side-effect note for the implementation plan:** removing the catch-all at `encode.rs:460-465` also removes the only *current* user of `tap_terminal_name` in error messages. If the function survives only for `validate_tap_leaf_subset`'s explicit-call path, its rustdoc should note that it is no longer the universal naming hook for tap-context errors.

### CRIT-3 (cross-check #1, structural): §2.3 "swap" wording is misleading; §2.3 column omissions

§2.3's narrative paragraph below the table calls `0x07 ↔ 0x08` a "swap (Bare ↔ TapTree)". That isn't a swap: §2.2 places `Bare` at `0x32`, not at `0x08`. The table itself shows `Bare` v0.5 0x07 → v0.6 0x32 (correct) and `TapTree` v0.5 0x08 → v0.6 0x07 (correct), so the narrative paragraph is internally inconsistent with the table. Reword as:

> `TapTree` moves down to 0x07 (adjacent to Tr=0x06); `Bare` moves up to 0x32 (next to the MD-specific framing block); the multisig family expands into 0x08–0x0B (Multi, SortedMulti, MultiA, SortedMultiA); wrappers and logical operators shift by 2 from their v0.5 positions.

Also: §2.3's table omits `True` (0x01 unchanged) — listed as 0x00→0x00 only the False row. Add a `True` row for completeness, or add a single sentence "operators not listed here are byte-identical from v0.5 to v0.6". Without that, an auditor double-checking the table will not see at a glance that `True` is fixed.

Cross-check of §2.2 ↔ §2.3 ↔ existing `tag.rs` Terminal coverage: every byte in 0x00–0x35 is accounted for (with the explicit gap at 0x24–0x31 for dropped `Reserved*` and the unallocated 0x34 and the byte 0x36+ tail) and every Terminal variant in `terminal_to_tag` (encode.rs:604-639) is covered after `Tag::SortedMultiA` is added. **No byte conflicts, no double-allocations, no missing Terminals.** The layout is internally consistent; only the narrative paragraph wording and the missing `True` row need fixing.

### IMP-4 (cross-check #4 / Q5): BIP MAY clause is OK; §"Signer compatibility" §"§7.2" is good; §7.3 is too vague

§7.1's MAY rewrite reads cleanly: "Implementations MAY enforce a per-leaf miniscript subset matching their target hardware signer's documented admit list. The MD encoding format itself does not require this — see §"Signer compatibility (informational)" below..." This does NOT imply MD is wrong if it doesn't enforce — the second sentence explicitly absolves the format. Fine.

§7.2's responsibility chain is clear: wallet → MD → user → recovery. The "If an MD-encoded backup contains operators a recovery-time signer will not sign..." paragraph correctly delegates upward. Fine.

**Concern:** §7.3 ("Tag table updates ... location TBD by audit during implementation") defers the BIP draft Tag table edit to implementation. The current BIP draft Tag table at `bip-mnemonic-descriptor.mediawiki:371-453` plus the prose "Tags 0x24–0x31 are reserved..." paragraph at line 455 are the affected sections. The spec should pin those line numbers explicitly so the implementer doesn't re-discover them. Same for any §7.3 wording about dropping the `Reserved*` paragraph at line 455 — see Q5 below.

### IMP-5 (cross-check #5): Corpus expansion adequacy — wrappers and andor are under-covered

§6.1's 10 vectors do exercise:
- `s:` wrapper (in `tr_thresh_in_tap_leaf_md_v0_6`, `tr_or_b_in_tap_leaf_md_v0_6`)
- `or_b` (`tr_or_b_in_tap_leaf_md_v0_6`)
- `sha256` (`tr_sha256_htlc_md_v0_6`)
- `after` absolute height + absolute time (Ledger compound shapes)
- `older` (in `tr_older_relative_time_md_v0_6` and several others)
- `pkh` (`tr_pkh_in_tap_leaf_md_v0_6`)
- `sortedmulti_a` (the centerpiece)
- multi-leaf TapTree with `sortedmulti_a` (Coldcard)
- recovery-path shape (Coldcard)

But the table does NOT exercise: `a:` wrapper, `d:` wrapper, `j:` wrapper, `n:` wrapper, `andor` (the spec's open question implicitly notes only a few wrappers). Also `hash256`, `ripemd160`, `hash160`, and `or_c`, `or_i` get no coverage. These are all newly admitted Tap-leaf operators in v0.6.

For a v0.6 release whose explicit purpose is "admit-set widening", the corpus should at minimum exercise once per newly-admitted Terminal variant, even via synthetic `tr(@0/**, and_v(v:<op>, pk(@1/**)))` shapes. Recommended additions (5-7 more):

- `tr_andor_in_tap_leaf_md_v0_6` — `tr(@0/**, andor(pk(@1/**), pk(@2/**), pk(@3/**)))` — exercises `andor` (3 children).
- `tr_or_c_in_tap_leaf_md_v0_6` — exercises `or_c`.
- `tr_or_i_in_tap_leaf_md_v0_6` — exercises `or_i`.
- `tr_hash256_htlc_md_v0_6`, `tr_ripemd160_htlc_md_v0_6`, `tr_hash160_htlc_md_v0_6` — three more hash terminals to lock byte order across all four (Q3 ties in here).
- `tr_a_wrapper_in_tap_leaf_md_v0_6` — `tr(@0/**, and_b(pk(@1/**), a:pk(@2/**)))` or similar — exercises `a:`.
- `tr_d_wrapper_in_tap_leaf_md_v0_6` — exercises `d:`.

If 10 is a hard quota for the spec to hit, at least **note as a FOLLOWUPS item** that comprehensive per-Terminal positive coverage will be filled in by the implementation phase. The current 10 don't lock the byte format for the majority of the newly-admitted operators.

### IMP-6 (cross-check #6): Migration is roughly complete; one explicit pub-API item to add

§9.1 covers the five major breaking changes. One additional item to add for completeness:

6. **`tag_to_bip388_name`** at `decode.rs:813` is `pub(crate)`, not pub, so no migration concern. **However** the public `Tag` enum's `from_byte` at `tag.rs:138` returns `None` for bytes 0x24–0x31 in v0.6 (formerly `Some(Tag::Reserved*)`). Any external code that did `Tag::from_byte(0x24)` and pattern-matched on a `Reserved*` variant gets a compile error (because the variants are gone) AND a runtime behavior change (None vs Some). The compile error covers the safety concern, but the migration note should state both: "matching on Reserved\* variants no longer compiles; `Tag::from_byte` for these bytes returns `None` instead of `Some(...)`. Code that defensively matched these to error out can simply rely on the `None` arm."

Also: the spec says `Error::TapLeafSubsetViolation` is retained. `Error` is `#[non_exhaustive]` per `MD_SCOPE_DECISION_2026-04-28.md` line 71. Confirmed-OK, no migration concern for Error variants. But note the open question from MD_SCOPE_DECISION line 103: should the variant be renamed to a more general `SubsetViolation`? The spec does not address this. Recommend either: (a) explicitly defer the rename to v0.7+ (since rename is a breaking-change anyway and there's no urgency pre-1.0), OR (b) bundle the rename into v0.6 since that's the breaking-change boundary. **Recommendation: (b)** — pre-1.0, the rename is cheap; rename to `SubsetViolation { operator, leaf_index }` (drop the "TapLeaf" prefix since the explicit-call validator can be used for Segwitv0 subsets too in principle). Add this to §5 and §9.1.

### IMP-7 (cross-check #7): Acceptance criteria — the BIP draft `Reserved*` paragraph cleanup is missing

§11 lists 9 items. Missing: an explicit acceptance criterion that the BIP draft's `Reserved*` discussion (line 455, plus the inline comment in the Tag table at lines 455-457) is updated consistently with the Tag-space rework. This ties to §12 Q5. Add:

- 10. **BIP draft cleanup**: the `Reserved*` paragraph at `bip-mnemonic-descriptor.mediawiki:455` is removed or rewritten to read "Tags 0x24–0x31 are unallocated; they were reserved in v0.5 for descriptor-codec inline-key compatibility but were removed in v0.6 since MD's BIP-388 wallet-policy framing forbids inline keys" — and the Tag table is updated to the v0.6 §2.2 layout.

Also add a recommended acceptance criterion:

- 11. **No regression in error-coverage CI**: `tests/error_coverage.rs` (referenced at `Cargo.toml:51`) still passes with the renamed `Error::SubsetViolation` (if renamed per IMP-6).

## §12 open questions — explicit answers

### Q1 — Tag layout finalization

The proposed §2.2 layout is internally consistent and groups operators coherently. Specific recommendations:

- **`TapTree` at 0x07 vs 0x08**: 0x07 is correct given it's structurally adjacent to `Tr=0x06`. Reading `decode_tr_inner` (decode.rs:264-299), the multi-leaf path peeks for the inner-node framing byte; semantically `TapTree` belongs adjacent to `Tr`. Endorse 0x07.
- **`Bare` at 0x32**: see Q4 below — recommend dropping rather than relocating.
- **Multisig family contiguous at 0x08–0x0B**: good. The four variants (Multi, SortedMulti, MultiA, SortedMultiA) are visually a unit and the byte layout reflects that.
- **Wrappers contiguous at 0x0C–0x12**: good.
- **Logical operators contiguous at 0x13–0x1A**: good. (Optional micro-nit: BIP 388's source ordering is and_v / and_b / andor / or_b / or_c / or_d / or_i / thresh — same as proposed. Endorse.)
- **`Fingerprints` retained at 0x35**: §2.2 keeps this but doesn't explain why. The reason (preserving the v0.2 wire byte for the fingerprints block tag, since Fingerprints framing already shipped) is worth a one-line note.

**Final recommendation:** §2.2 is good as-is, modulo the "drop Bare" question (Q4).

### Q2 — Encoder catch-all behaviour

Recommend option (a): exhaustive match. Tag-emit unconditionally for any in-Tag-set Terminal. Rationale:

- Confirms the new "format is neutral" framing.
- Compiler-checked exhaustiveness catches future miniscript upgrades that add Terminal variants (forces a v0.7+ Tag allocation discussion).
- `Terminal::Multi` and `Terminal::SortedMulti` reaching a Tap-context encoder would be a miniscript-upstream bug; emitting the wire byte regardless is harmless because no decoder will produce that AST shape from a tap context (the decoder has its own Tap-context dispatcher).

Caveat: see CRIT-2 — the spec must enumerate this explicitly so the implementer doesn't decide between (a)/(b)/(c) under their own judgment.

### Q3 — Hash terminal byte order

Auditing the existing encoder/decoder:
- `Sha256`: encoder emits `h.as_byte_array()` directly (encode.rs:311); decoder reads `cur.read_array::<32>()` and calls `bitcoin::hashes::sha256::Hash::from_byte_array(bytes)` (decode.rs:505-507). **Internal byte order, no reversal.**
- `Hash256`: encoder emits `h.as_byte_array()` where `h: miniscript::hash256::Hash` (a forward-display newtype around sha256d::Hash); decoder reads via `miniscript::hash256::Hash::from_byte_array` (decode.rs:514-516). **Internal byte order, NOT reversed display order** — this is explicitly documented in the encoder comment at encode.rs:316-319.
- `Ripemd160`: encoder emits internal byte order; decoder reads and calls `bitcoin::hashes::ripemd160::Hash::from_byte_array` (decode.rs:517-520). **Internal byte order.**
- `Hash160`: same pattern, `bitcoin::hashes::hash160::Hash::from_byte_array` (decode.rs:521-524). **Internal byte order.**

**Answer:** All four hash terminals encode their *internal* byte order (which for `Sha256/Ripemd160/Hash160` happens to be network/wire order; for `Hash256` it is the SHA256d internal order, NOT the conventional reversed-display-order). The spec should cite the encoder comment at encode.rs:316-319 (which is the only place in tree where this distinction is documented) and reaffirm: byte order is invariant from v0.5 to v0.6 for all four hash terminals. No spec change needed; no implementation surprise expected.

### Q4 — `Tag::Bare` retention

Recommend **drop** `Tag::Bare` entirely in v0.6.

Rationale:
- The encoder rejects `Descriptor::Bare` permanently per BIP draft scope (encode.rs:176-179: "top-level bare() is permanently rejected (legacy non-segwit out of scope per design)").
- The decoder rejects `Tag::Bare` at the top level with a permanent rejection (decode.rs:71-74).
- It is not used as an inner tag anywhere.
- The BIP draft Tag table line at `bip-mnemonic-descriptor.mediawiki:390` already says "Top-level: REJECTED. Not used in any nested context in MD's accepted surface."
- "Reserved for future bare-descriptor support" is unconvincing: bare descriptors are pre-2014 and explicitly excluded from BIP 388, so MD has no path back to them. If a future MD ever needed them, allocating a fresh byte is trivial.
- Dropping Tag::Bare frees byte 0x32 — but §2.2 puts Placeholder at 0x33 in v0.6, which is the right move. So byte 0x32 simply becomes unallocated. Neat.

**Concrete fix to §2.2:** remove the "Bare top-level (1) — 0x32 Bare" block; note "byte 0x32 unallocated (formerly v0.5 Placeholder; intentionally left unallocated to avoid reusing a byte that v0.5 emitted in valid encodings)" or similar. **Caveat:** there's a mild argument for leaving 0x32 "intentionally never re-used" so a v0.5 → v0.6 transcoder mistake (if anyone built one) surfaces immediately as `from_byte = None` rather than being silently misinterpreted. Worth mentioning in §2.4.

Also: remove `Tag::Bare` from the §11 acceptance check (currently the spec implies it survives as a never-emitted tag). And remove the `Tag::Bare => "bare"` arm in `tag_to_bip388_name` at decode.rs:822.

### Q5 — `p2-inline-key-tags` BIP draft cleanup

Recommend: **rewrite, do not remove entirely.**

Rationale: the BIP draft's `Reserved*` discussion at line 455 currently says "Tags 0x24–0x31 are reserved for future descriptor-codec compatibility (... v1+ may expose them for foreign-xpub support)". In v0.6, this is wrong on two counts: (a) the tags are dropped, not reserved; (b) v1+ exposure for foreign-xpub is no longer the plan (per FOLLOWUPS `p2-inline-key-tags` status `wont-fix`).

Remove-entirely is *too* clean — readers of v0.6+ who encounter v0.5 Tag-table documentation in old commits or external mirrors won't have the "what happened to 0x24–0x31" answer in the latest draft. Keep a one-line note for historical orientation:

> Tags 0x24–0x31 are unallocated. (In MD v0.5 and earlier, these bytes were reserved for descriptor-codec inline-key compatibility; MD v0.6 dropped them since MD's BIP-388 wallet-policy framing forbids inline keys. See `MD_SCOPE_DECISION_2026-04-28.md` for rationale.)

Tag 0x34 stays "reserved" as today (no change). Tag 0x36+ stays "reserved" (no change).

The spec's §7.3 should pin this as the exact text (or close to it) so the BIP-draft edit is unambiguous.

## Concerns / deviations summary

The spec is in good shape — none of the findings are *blocking*. CRIT-1 (decoder audit miss) and CRIT-2 (encoder catch-all under-specified for tap-illegal Terminals) are spec-text-level fixes (one paragraph each); CRIT-3 (§2.3 "swap" wording + missing True row) is editorial; IMP-4..IMP-7 are small spec-text adds; the §12 questions all have concrete recommendations above.

If all of the above are addressed inline in the spec, the implementation plan can be written with confidence. Status `DONE_WITH_CONCERNS` rather than `BLOCKED` because none of the issues prevent the plan from being drafted — they just narrow what the plan must cover.

## Nits and nice-to-haves (collect for FOLLOWUPS)

These do NOT block the spec. Suggested for FOLLOWUPS entries:

- **`v06-spec-true-row`**: §2.3 table should add an explicit `True` row (0x01 unchanged) for table completeness, or note "operators not listed are byte-identical from v0.5 to v0.6".
- **`v06-spec-bare-rationale-line`**: §2.4 should add a one-liner rationale for why byte 0x32 is left unallocated (not re-used) post-Bare-drop, namely "v0.5 emitted Placeholder=0x32 in every encoded MD string; reusing 0x32 in v0.6 for a different operator would be hostile to anyone who hand-coded a v0.5 → v0.6 transcoder, while leaving it unallocated surfaces the version mismatch as a clean `from_byte=None` error".
- **`v06-spec-fingerprints-rationale`**: §2.2 should explain in one line why `Fingerprints=0x35` stays put (preserve v0.2-shipped fingerprints-block byte for any external decoder that already looks at it).
- **`v06-error-rename-tapleaf-to-subset`**: rename `Error::TapLeafSubsetViolation` to `Error::SubsetViolation` in v0.6 since `validate_tap_leaf_subset` is the one place it's raised but the variant name presumes Tap-context — explicit-call use for Segwitv0 subsets is plausible.
- **`v06-corpus-per-terminal-positive-coverage`**: add positive vectors for `andor`, `or_c`, `or_i`, `hash256`, `ripemd160`, `hash160`, `a:`, `d:`, `j:`, `n:` so every newly-admitted Terminal has at least one round-trip fixture locking its v0.6 byte form. The 10 spec-listed vectors miss roughly half.
- **`v06-spec-decoder-arms-checklist`**: §4.3 should be a concrete Add/Keep checklist per Tag, not a narrative table, so the implementer has a flat to-do list. (Tied to CRIT-1.)
- **`v06-bip-draft-line-pinning`**: §7.3 should pin specific line numbers (BIP draft Tag table at `bip-mnemonic-descriptor.mediawiki:371-453`, `Reserved*` paragraph at line 455) rather than "TBD by audit".
- **`v06-spec-tag-byte-display-table`**: consider adding a final-state alphabetical index `Tag → byte` after §2.2 (the current §2.2 is grouped-by-purpose; an alphabetical secondary listing makes audit-by-name fast).

End of review.
