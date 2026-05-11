# Implementation plan — md-codec v0.30

> **Status:** Cycle 1 (design) frozen 2026-05-10. Cycles 2–5 (implementation) pending. SPEC: `design/SPEC_v0_30_wire_format.md`. Validation: `design/agent-reports/spike-v0.30-{q9,q10,q11,q13,q6}-pre-spec.md`.
>
> **Companion files:**
> - `design/SPEC_v0_30_wire_format.md` — canonical wire-format specification
> - `design/agent-reports/spike-v0.30-*-pre-spec.md` — Phase 0a empirical validation
> - `design/FOLLOWUPS.md` `v2-design-questions` entry — original catalog (most items now disposed of per SPEC §1.5)
> - `bip/bip-mnemonic-descriptor.mediawiki` — BIP draft (Phase I rewrite target)

---

## 1. Overview + dependency DAG

WF-redesign targets `version = 4`. All v0.x decode paths are deleted. Implementation runs across Cycles 2–5; Cycle 1 (design-only) is closed by tagging `md-codec-v0.30-spec-frozen`.

### Phase dependency DAG

```
A(tag-space) ──┬──> C(multi packing) ──> F(NUMS flag)
               ├──> E(walker norm)
               └──> G(error taxonomy)  <── B,C,E,F (all)

B(header)  ─────────────────────────> G

G(errors) ──────> H(corpus regen)
H ──────────────> I(BIP rewrite)
H ──────────────> J(final tag)
I ──────────────> J
```

Phase D (TLV length prefix) was contemplated but **deleted** when SW2 was reverted per Phase 3.6 (SPEC §9). All cross-decode-path sweeps run in Phase G (closing commit of Cycle 3).

No spike-cycle precursors needed: all five Phase 0a spikes (Q9, Q10, Q11, Q13, Q6) were completed as agent-driven analysis during Cycle 1.

---

## 2. Codebase surface scan

Per-touch-point table grouped by SPEC section, citing exact `file:line` for each.

### SPEC §2 — Header layout (Q8, Q10, SW1)

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-codec/src/header.rs:16–21` | `Header` struct | Field replacement: 3-bit `version` → 4-bit | Q8, SW1 |
| `crates/md-codec/src/header.rs:28–32` | `Header::write` | Behavior: emit `[paths:1][v3:1][v2:1][v1:1][v0:1]`; version=4 constant | Q8, Q10, SW1 |
| `crates/md-codec/src/header.rs:36–51` | `Header::read` | Behavior: 4-bit version at bits 3..0; remove reserved-bit check; `WireVersionMismatch` for `version != 4` | Q8, Q10, SW1, §11 |
| `crates/md-codec/src/header.rs:25` | version constant | Removal+new: drop `V0_11_VERSION = 0`; add `WF_REDESIGN_VERSION: u8 = 4` | Q8 |
| `crates/md-codec/src/chunk.rs:27–49` | `ChunkHeader::write` | Behavior: emit `[v3][v2][v1][v0][chunked]` at bits 4..0; bounds-check & version-width changes at lines 28–41 also in scope (`& 0b1111` not `& 0b0111`) | Q10, SW1 |
| `crates/md-codec/src/chunk.rs:56–71` | `ChunkHeader::read` | Behavior: 4-bit version at bits 4..1; chunked-flag at bit 0; `_reserved` removed | Q10, SW1, §11 |
| `crates/md-codec/src/chunk.rs:12` | `ChunkHeader` struct | Doc update: 4-bit version | Q8 |
| `crates/md-codec/src/decode.rs:14–52` | `decode_payload` | Behavior: remove v0.x decode entirely; on entry, examine first 5-bit symbol's bit 0 to auto-dispatch single vs chunked | Q10, SW3, clean-break |
| `crates/md-codec/src/decode.rs:54–62` | `decode_md1_string` | Behavior: minimal — strips codex32 wrapper then delegates to `decode_payload`. May not change at all under Phase B; included for public API stability | Q10 |
| `crates/md-codec/src/chunk.rs:193–248` | `split` | Behavior: emits `version=4`; `SINGLE_STRING_PAYLOAD_BIT_LIMIT` unchanged | Q8 |
| `crates/md-codec/src/chunk.rs:263–347` | `reassemble` | Behavior: rejects v0.x with `WireVersionMismatch` | Q10, §11 |
| `crates/md-codec/src/encode.rs:82–86` | `encode_payload` | Behavior: writes `version=4`; new `kiw` formula | Q8, SW3 |

### SPEC §3 — Tag space (Q4, Q7, Q13)

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-codec/src/tag.rs:13–87` | `Tag` enum | Variant set rewrite: 6-bit primary codes; promote Hash256/Ripemd160/RawPkH/False/True to primary 0x1F–0x23 | Q7, Q13 |
| `crates/md-codec/src/tag.rs:89` | `EXTENSION_PREFIX` | Removal+new: drop `0x1F`; add `EXTENSION_PREFIX_6BIT: u8 = 0x3F` | Q13 |
| `crates/md-codec/src/tag.rs:94–133` | `Tag::codes` | Behavior: returns 6-bit primary + 4-bit extension sub | Q13 |
| `crates/md-codec/src/tag.rs:136–142` | `Tag::write` | Behavior: `write_bits(..., 6)` primary; `write_bits(..., 4)` extension | Q13 |
| `crates/md-codec/src/tag.rs:145–193` | `Tag::read` | Behavior: `read_bits(6)` primary; 4-bit extension; reserved range 0x24–0x3E → `TagOutOfRange` | Q7, Q13, §11 |
| `crates/md-codec/src/tlv.rs:203` | `TlvSection::write` (TLV tag emit) | **NO CHANGE from v0.x** (Q13 split: TLV tags stay 5-bit; only bytecode tags grow to 6-bit) | — |
| `crates/md-codec/src/tlv.rs:229` | `TlvSection::read` (TLV tag read) | **NO CHANGE from v0.x** (Q13 split) | — |
| `crates/md-codec/src/tree.rs:78–93` | `write_node` Variable arm | Behavior: **fixed-5 for k-1, n-1** (Q4 lock); multi-family raw key indices (no child tag); Thresh keeps full children | Q4, Q9 |
| `crates/md-codec/src/tree.rs:185–196` | `read_node_with_depth` Multi*/Thresh arm | Behavior: **fixed-5 for k/n** (Q4 lock); multi-family reads raw `kiw` indices | Q4, Q9 |

### SPEC §4 — Multi child packing (Q9)

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-codec/src/tree.rs:18–27` | `Body` enum | New variant: split `Body::Variable` into `Body::Variable { k, children: Vec<Node> }` (Thresh) and `Body::MultiKeys { k, indices: Vec<u8> }` (multi-family) | Q9 |
| `crates/md-codec/src/tree.rs:78–92` | `write_node` multi-family path | Behavior: emit `k-1(5 fixed) | n-1(5 fixed) | idx_0(kiw) | ... | idx_{n-1}(kiw)` | Q9 |
| `crates/md-codec/src/tree.rs:185–196` | `read_node_with_depth` multi-family path | Behavior: read fixed-5 k/n; read n raw `kiw` indices; construct `Body::MultiKeys` | Q9 |
| `crates/md-codec/src/encode.rs:67–94` | `encode_payload` / `Descriptor::key_index_width` | Behavior: `kiw = ⌈log₂(n)⌉` (NUMS flag, not sentinel) | SW3 (via §7) |
| `crates/md-cli/src/format/text.rs:56–59` | `render_node` Multi* arms | Behavior: read indices from `Body::MultiKeys`; render `@i` per index | Q9 |
| `crates/md-cli/src/parse/template.rs` | multi-family walk | Behavior: builds `Body::MultiKeys` instead of full child Nodes | Q9 |

### SPEC §5 — Walker normalization (Q12)

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-cli/src/parse/template.rs` | `walk_tr` / `walk_tap_leaf` (c: sites) | Behavior: emit bare `Tag::PkK`/`Tag::PkH` at c:-positions; never wrap with `Tag::Check` | Q12 |
| `crates/md-cli/src/format/text.rs:60–81` | `render_node` PkK/PkH arms | Behavior: when parent is Check-context, render as `pk(K)` / `pkh(K)` without re-emitting `c:` prefix | Q12 |
| `crates/md-cli/src/format/text.rs:167–169` | `render_node` Check arm | Behavior: Check on wire only wraps non-key children | Q12 |

### SPEC §7 — NUMS flag (SW3)

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-codec/src/tree.rs:38–45` | `Body::Tr` | New field: `is_nums: bool`; remove sentinel doc-comment | SW3 |
| `crates/md-codec/src/tree.rs:94–99` | `write_node` Tr arm | Behavior: emit `is_nums(1)`; `key_index(kiw)` only when `is_nums=false`; `kiw = ⌈log₂(n)⌉` | SW3 |
| `crates/md-codec/src/tree.rs:197–209` | `read_node_with_depth` Tr arm | Behavior: read `is_nums`; conditional `key_index` | SW3, §11 |
| `crates/md-codec/src/encode.rs:40–43` | `Descriptor::key_index_width` | Behavior: formula `(32 - n.leading_zeros())` (drop +1) | SW3 |
| `crates/md-codec/src/decode.rs:24` | `decode_payload` `kiw` calc | Behavior: same formula change | SW3 |
| `crates/md-cli/src/parse/template.rs:17–18` | NUMS detect / `walk_tr` | Behavior: NUMS sets `Body::Tr { is_nums: true, key_index: 0 }` (index unused) | SW3 |
| `crates/md-cli/src/format/text.rs:34–45` | `render_node` Tr arm | Behavior: branch on `is_nums`; remove `key_index == n` sentinel check | SW3 |

### SPEC §9 — TLV section framing (NO CHANGE — SW2 reverted)

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-codec/src/tlv.rs:86–207` | `TlvSection::write` | **NO CHANGE from v0.x** (TLV section uses implicit end-of-stream + rollback-as-padding contract retained) | — |
| `crates/md-codec/src/tlv.rs:210–306` | `TlvSection::read` | **NO CHANGE from v0.x** (rollback-as-padding logic retained) | — |

### SPEC §11 — Error taxonomy

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-codec/src/error.rs:7–344` | `Error` enum + new `ContextKind` enum | Variant rewrite: add `WireVersionMismatch`, `MalformedHeader`, `TagOutOfRange`, `OperatorContextViolation { tag, context: ContextKind }`, `NUMSSentinelConflict`; remove `ReservedHeaderBitSet`, `UnknownPrimaryTag`, `UnknownExtensionTag`; rename `UnsupportedVersion` → `WireVersionMismatch`. **`TLVLengthOverflow` and `TLVLengthMismatch` were considered for SW2 framing but are NOT added** (SW2 reverted; rollback-as-padding contract retained from v0.x). | §11 |

### Public API + docs

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-codec/src/lib.rs` (re-exports) | `pub use` of `Tag`, `Error`, `Header`, `ChunkHeader`, `decode_md1_string`, `decode_payload`, `encode_md1_string`, `Descriptor` | Re-export surface: enum-variant renames propagate to downstream consumers (`md-cli`, `md-signer-compat`); recompile required | A, B, C, F, G |
| `crates/md-codec/src/lib.rs:8–11` | crate-level doc-comment | Doc-prose rewrite: "v0.11 wire format" / "5-bit header" / "3-bit version + reserved bit" → v0.30 equivalents | Phase J |
| `crates/md-codec/src/header.rs:1–9` | header module doc-comment | Doc-prose rewrite: references to v0.11 header layout | Phase B |
| `crates/md-codec/src/chunk.rs:1–5` | chunk module doc-comment | Doc-prose rewrite: references to v0.11 chunk header | Phase B |
| `crates/md-codec/Cargo.toml` | package version field | Version bump: `0.18.x` → `0.30.0` | Phase J |

### Corpus + BIP

| File:line | Function | Change type | Driven by |
|---|---|---|---|
| `crates/md-codec/tests/vectors/manifest.rs:14–32` | `MANIFEST` | Corpus regen: all `md1` strings invalidated; regen post A–G | A–G |
| `crates/md-codec/tests/smoke.rs` | all tests | Behavior: struct init updates (`Body::Tr` adds `is_nums`, `Body::MultiKeys`); bit-length pin updates | SW3, Q9, Q13 |
| `crates/md-codec/tests/chunking.rs` | chunk round-trips | Behavior: `version: 4` constants | Q8, Q10 |
| `bip/bip-mnemonic-descriptor.mediawiki:178–802` | BIP sections (per SPEC §12) | Corpus regen + prose rewrite (12 sections) | Phase I |

---

## 3. Per-phase breakdown

### Phase A — Tag-space rework

- **Files:** `crates/md-codec/src/tag.rs:89–193`, `tlv.rs:203,229` (tag-width change limited to ~2 LOC each; overlap with potential Phase D removed since SW2 reverted), `error.rs` (TagOutOfRange add; UnknownPrimaryTag/UnknownExtensionTag remove — owned entirely by Phase A)
- **LOC:** ~90 changed, ~20 new
- **Tests:** tag round-trips rewrite; new `tag_reserved_range_rejected` test
- **Commit boundary:** atomic; all tag encode/decode in lockstep
- **Stop condition:** `cargo test -p md-codec --lib` passes (unit tests only); `cargo test -p md-codec --test smoke` and `--test chunking` are EXPECTED TO FAIL after Phase A (stale corpus encoded with old tag widths); the "all-green" criterion is satisfied only after Phase H. Specific Phase-A tests to pass: `tag_encode_decode_round_trip`, `tag_reserved_range_rejected` (new), all `tag_primary_*` unit tests.
- **Dependencies:** none

### Phase B — Header layout

- **Files:** `header.rs:16–51`, `chunk.rs:27–71`, `error.rs` (WireVersionMismatch + MalformedHeader add; ReservedHeaderBitSet + UnsupportedVersion remove)
- **LOC:** ~70 changed, ~30 new
- **Tests:** `header_rejects_reserved_bit` removed; `header_rejects_version_mismatch` new (covers v0.x version=0 single-payload + v0.x-chunked-misread-as-version-2); 2 new explicit v0.x rejection tests
- **Commit boundary:** atomic; header + chunk header in lockstep
- **Stop condition:** header + chunk unit tests pass
- **Dependencies:** none (independent of A)

### Phase C — Multi child packing

- **Files:** `tree.rs:18–27` (Body::MultiKeys add), `tree.rs:78–93`, `tree.rs:185–196`, `md-cli/src/parse/template.rs`, `md-cli/src/format/text.rs`
- **LOC:** ~80 changed, ~30 new
- **Tests:** `sortedmulti_2of3_round_trip` rewrites; new `multi_keys_body_round_trip`; thresh tests unchanged
- **Commit boundary:** atomic; AST + encoder + decoder + CLI in lockstep
- **Stop condition:** multi round-trips pass
- **Dependencies:** Phase A (6-bit tag in place)

### Phase E — Walker normalization

- **Files:** `md-cli/src/parse/template.rs`, `md-cli/src/format/text.rs:60–81,167–169` (file-level conflict with Phase C on `text.rs`; sequencing C → E means E rebases on Phase C's changes to the same file)
- **LOC:** ~40 changed
- **Tests:** Q12 regression (`8de2df1` pattern, test `wsh_pkh_shorthand_collapse_round_trips` at `crates/md-cli/tests/template_roundtrip.rs:237`) updated; new `pkh_key_leaf_bare_on_wire` test
- **Commit boundary:** atomic (walker + renderer must stay in sync)
- **Stop condition:** test `wsh_pkh_shorthand_collapse_round_trips` passes with updated assertion reflecting bare `Tag::PkK`/`Tag::PkH` on wire; new test `pkh_key_leaf_bare_on_wire` passes
- **Dependencies:** Phase A (6-bit Tag::PkK); Phase C (same file, different functions)

### Phase F — NUMS flag removal

- **Files:** `tree.rs:38–45,94–99,197–209`, `encode.rs:40–43`, `decode.rs:24`, `md-cli/src/parse/template.rs:17–18`, `md-cli/src/format/text.rs:34–45`
- **LOC:** ~50 changed
- **Tests:** all `tr_sentinel_*` tests rewrite; `tr_bip86_no_tree` updates with narrower `kiw`; new `tr_nums_flag_round_trip` and `tr_nums_flag_rejected_outside_tr` tests
- **Commit boundary:** atomic
- **Stop condition:** all tr round-trips pass; `NUMSSentinelConflict` fires correctly on `is_nums=0 ∧ key_index ≥ n`
- **Dependencies:** Phase C (kiw formula shared; multi must already use new kiw before Tr changes; otherwise multi encoder uses new kiw formula while emitting old per-child tags → miscount)

### Phase G — Error taxonomy refactor (single sweep, no G-partial)

- **Files:** `error.rs:7–344` (full enum rewrite + ContextKind enum), all decode paths in `tag.rs`, `header.rs`, `chunk.rs`, `tlv.rs`, `tree.rs`, `decode.rs`
- **LOC:** ~120 changed
- **Tests:** every `assert!(matches!(... Err(Error::X ...)))` updates; new `operator_context_violation_multi_body` and `nums_sentinel_conflict` tests
- **Commit boundary:** atomic sweep; compiler surfaces every call site
- **Stop condition:** `cargo test --all-features -p md-codec` clean
- **Dependencies:** A–F (all decode paths finalized)

### Phase H — Corpus regen + vector pin updates

- **Files:** `tests/vectors/manifest.rs`, `tests/smoke.rs`, `tests/chunking.rs`
- **LOC:** ~80 changed (string literals + constants)
- **Tests:** net zero new; all integration tests pass on new wire
- **Commit boundary:** atomic regen
- **Stop condition:** `cargo test --workspace` clean
- **Dependencies:** A–G complete

### Phase I — BIP draft rewrite

- **Files:** `bip/bip-mnemonic-descriptor.mediawiki` (12 sections per SPEC §12)
- **LOC:** ~400 changed
- **Tests:** none (docs)
- **Commit boundary:** single doc commit
- **Stop condition:** BIP examples derive from Phase H's verified vectors; bit-layout examples consistent
- **Dependencies:** Phase H

### Phase J — Final tag

- **Files:** `crates/md-codec/Cargo.toml` (version 0.18.x → 0.30.0), `CHANGELOG.md`, `crates/md-codec/src/lib.rs:8–11` (crate doc-comment update)
- **LOC:** ~10
- **Stop condition:** tag `md-codec-v0.30.0` pushed; CI passes
- **Dependencies:** Phase I

---

## 4. Risk register

| Phase | Hazard | Likelihood | Mitigation |
|---|---|---|---|
| A | Q7 tag renumbering breaks every literal-tag reference in tests + corpus | High (certain) | Phase H sequenced after A–G; pre-commit `grep -r "0x0A\\|0x0B\\|0x1F"` sweep; agent-reports get doc annotation noting the renumber |
| C | `Body::MultiKeys` requires updating every exhaustive `match body` arm; missed arms → compile error but logic-gap risk in CLI | Medium | Phase C commit triggers exhaustive-match compile errors; treat each as required fix |
| C+F | If F merged before C, multi encoder uses new `kiw` formula while emitting old per-child tags → miscount | Medium | Strict C → F order; never combine in single phase |
| E | Q12 walker context change risks breaking `c:pk_k` tap-leaf round-trip if renderer misses parent context | Medium | `8de2df1` test pattern is regression guard; Phase E stop condition requires updated test passes |
| B | v0.x rejection: off-by-one in `version = 4` discriminator could let v0.x slip through sometimes | Low-medium | Phase B stop condition includes 2 v0.x-fed rejection tests (single-payload + chunked-misread); SPEC §2.5 has the auto-dispatch trace as oracle |
| G | Error call-site sweep misses a path in rare feature-gate (`cli-compiler`) | Low | `cargo test --all-features` in stop condition |
| Cross-cutting | Doc-comment drift in `lib.rs:8–11`, `header.rs:1–9`, `chunk.rs:1–5` referencing "v0.11 wire format" / "3-bit version" / "reserved bit" | Medium | Doc-prose updates owned by Phase B (header.rs + chunk.rs module docs) and Phase J (lib.rs crate doc + final tag); pre-commit `grep -r "v0.11\\|3-bit version\\|reserved bit" crates/` sweep |
| Cross-cutting | `version=4` vs chunk-set-id collision: are mixed-version chunked payloads possible? | Closed | Chunk-set-id is derived from payload bytes via SHA-256 (`chunk.rs:200-210`); payload bytes include version field, so cross-version chunk-set-id collisions are cryptographically impossible. No mitigation needed. |
| Phase A + (planned D) overlap | N/A — Phase D was deleted per Phase 3.6 lock | N/A | N/A |
| Q6 | BCH polynomial change deferred per Phase 0a Spike Q6 | N/A | No mitigation needed |

---

## 5. Cycle sequencing

**Cycle 2 (core wire — Phases A + B):** A and B independent; each handles its own error variant additions/removals as part of the atomic phase commit. Together they form the minimal compilable basis for the tag and header layers. **Cycle 2 ends in a non-compiling-but-consistent state** — tree encoding still references multi-family children as full Nodes; CLI walker/renderer still emit old wire shapes. Compile errors in `tree.rs`, `md-cli/*`, and `tests/smoke.rs` are EXPECTED. Cycle 3's first act is to re-baseline by completing C–F.

**Cycle 3 (tree encoding — Phases C + E + F + G):** All tree-layer changes plus the final error-taxonomy sweep. Phase order: C → E → F (per dependency graph; F depends on C). Phase G is the closing commit of Cycle 3, completing the call-site sweep across all decode paths.

**Cycle 4 (stabilization — Phase H):** Corpus regen. Short cycle (no design decisions). End-of-cycle: `cargo test --workspace` clean.

**Cycle 5 (documentation — Phases I + J):** BIP rewrite + final tag (with Phase J's doc-comment updates in `lib.rs:8–11`). Separate cycle for review window between stable corpus and BIP publication.

---

## 6. Definition of done

**Per phase:** complete when (a) atomic commit passes `cargo test -p md-codec` (or `--workspace` for H), (b) per-phase code-reviewer round reaches 0 Critical / 0 Important findings, (c) §3 stop condition satisfied.

**BIP draft rewrite (Phase I)** is post-implementation: cannot begin until Phase H corpus is stable. SPEC §12 names invalidated sections; rewrite uses Phase H verified vectors as canonical examples.

**v0.30 rollout complete when Phase J ships:** final tag `md-codec-v0.30.0` pushed; CI passes; tagged corpus matches Phase H byte-for-byte.

---

## 7. Cross-repo mirror entries

**No cross-repo work needed for v0.30.** Phase 0a Spike Q6 deferred BCH polynomial separation; Q6 was the sole md1 v0.30 change that would have required a mk1 companion entry per `CLAUDE.md` cross-repo coordination convention. ms1 unaffected by all md1 BCH and wire-format changes. This section remains a no-op for v0.30.

---

## 8. Empirical validation summary

Validation corpus: 12 Claude-proposed basic wallets + 5 user-supplied recovery shapes. WF wins in all three "key-info-inlining" modes:

| Mode | v0.x corpus bits | v0.30 corpus bits | Δ corpus |
|------|-------------------|--------------------|----------|
| A (template + paths) | 2,893 | 2,734 | **−159** |
| B (+ fingerprints) | 5,175 | 5,003 | **−172** |
| C (+ xpubs) | 35,829 | 35,647 | **−182** |

Per-Q/SW attribution (Mode A baseline): Q9 −145, Q12 −110, Q13 +115 (bytecode tags), SW3 −13, Q4 0 (locked fixed-5), SW2 0 (reverted). Sum −153 ≈ corpus −159 (≤6-bit counting tolerance in complex wallets).

See SPEC §13 and the originating plan file (`/home/bcg/.claude/plans/typed-rolling-spindle.md` § Phase 3.5 / 3.5b / 3.6) for the full per-wallet bit and char-count tables.
