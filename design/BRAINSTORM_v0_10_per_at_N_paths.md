# Brainstorm — md1 v0.10.0: per-`@N` origin path-tag allocation

**Status:** Brainstorm draft — for user review before spec.
**Tracks:** FOLLOWUPS `md-per-at-N-path-tag-allocation`. Wire-format-breaking → v0.10.0 minor bump.
**Companion in mk1:** `md-per-N-path-tag-allocation` (mk1 has already declared the cross-format authority-precedence semantics; the wire-format byte allocation is purely md-side).

## 1. Problem restatement

BIP 388 wallet policies routinely use different origin paths per cosigner. Real-world example:

```
[fp1/48'/0'/0'/2']xpub_A   ← cosigner 1 (P2WSH multisig, mainnet account 0)
[fp2/48'/0'/0'/100']xpub_B ← cosigner 2 (P2WSH multisig, mainnet account 100)
[fp3/87'/0'/0']xpub_C      ← cosigner 3 (BIP 87 generic multisig)
```

md1 v0.x carries a **single** shared `Tag::SharedPath = 0x34` declaration that constrains all `@N` keys to the same origin path. Any policy with per-cosigner path divergence cannot round-trip.

mk1 has declared its side (`mk1 SPEC §5.1`):

> mk1's `origin_path` is **authoritative** for the xpub's derivation. md1's per-`@N` path is the policy's **expected** path (descriptive). Mismatch → recovery orchestrator rejects assembly.

The remaining decision is **how md1 encodes the per-`@N` paths** in canonical bytecode.

## 2. Current bytecode layout (for reference)

```
[header: 1 byte] [Tag::SharedPath=0x34 + indicator + ...] [Tag::Fingerprints=0x35 + count + 4*N bytes]? [tree bytes]
```

- **Header bits free:** 3, 1, 0 (bit 2 is `Fingerprints`, bits 7–4 are version=0).
- **Tag bytes free:** `0x36+` (next clean; current allocations end at `0x35`); plus `0x24–0x31` (reclaimed from the Reserved range when `p2-inline-key-tags` was wont-fixed in v0.6) and `0x32` (intentionally-unallocated gap from the v0.5→v0.6 renumber).
- **Path encoding** (per `bytecode/path.rs`): single byte for dictionary-form (e.g. `0x06 = m/48'/0'/0'/1'`) or `0xFE` followed by LEB128 component count + per-component `2*index+hardened_bit`.

## 3. Questions

### Q1 — Tag byte allocation

**Option A: `0x36` (next clean).** Continues the natural sequence after `Fingerprints=0x35`. New tag allocation; no reuse of historically-reserved bytes.

**Option B: `0x24` (start of reclaimed Reserved range).** Uses the first byte of the dropped 0x24–0x31 range. Keeps high bytes free for future tags but reuses bytes previously documented as reserved.

**Option C: `0x32` (intentionally-unallocated gap).** The single byte left unallocated by the v0.5→v0.6 renumber. Tightly compacted; leaves 0x36+ for future tags.

**My take:** **A — `0x36`.** Cleanest, no historical baggage. Compactness arguments for B/C are weak — md1's tag space is sparse and we have ~200 unused bytes; no need to cram. The 0x36 slot is also where `Tag::RecoveryHints` was sketched in `design/POLICY_BACKUP.md` for BIP 393 v1+ work, but that's also speculative — first-come-first-served is fine.

**Sub-question:** does this leave Q-393 (`Tag::RecoveryHints`) homeless? If yes, allocate it to `0x37` simultaneously and document, even if not implemented yet.

### Q2 — Encoding shape

**Option A: List of (indicator, [explicit-bytes]) tuples, dense (one per `@N` in placeholder-index order).**

```
Tag::OriginPaths(0x36) | varint(N) | path_decl_0 | path_decl_1 | ... | path_decl_{N-1}
```

where `path_decl_i` reuses the existing path-encoding from `bytecode/path.rs` (single byte for dictionary form, or `0xFE + LEB128 count + components` for explicit). N = placeholder count, must match the count derivable from the tree.

**Option B: Sparse list of (placeholder-index, indicator, [explicit-bytes]) tuples.** Only `@N` slots whose path differs from `SharedPath` get an entry; others fall back to SharedPath.

**Option C: Dense list (Option A) but coexists with `Tag::SharedPath` — SharedPath is the "fallback when an `@N` slot encodes a no-path indicator (e.g., `0xFD`)."**

**My take:** **A (dense).** Simplest invariant: when per-`@N` is present, it is the complete authority for all `@N` paths. Sparse encoding (B) saves bytes when paths cluster around a default but introduces a "what's the fallback" coupling that's harder to reason about — and the byte savings are marginal (a single dictionary-byte path is already 1 byte). C is worse: it requires defining a "no-path" sentinel and explaining when to use it. Dense + replace-SharedPath is the cleanest mental model.

**Sub-question:** count prefix encoding. LEB128 / varint? Or just rely on the placeholder count derivable from the tree? **My take:** include explicit count prefix anyway — defense-in-depth against tree-walking bugs and a clearer error path (`OriginPathsCountMismatch { expected, got }`).

### Q3 — Coexistence with `Tag::SharedPath`

**Option A: When per-`@N` paths present, `Tag::SharedPath` MUST be absent.** Mutually exclusive at the wire level. Encoder picks one.

**Option B: When per-`@N` paths present, `Tag::SharedPath` is reserved (must be absent OR if present, MUST be ignored / MUST trigger error).**

**Option C: Both can be present — SharedPath as the "common case advice," per-`@N` as the override list.** Per-`@N` takes precedence when both are present.

**My take:** **A — strict mutual exclusion at wire level.** "Pick one" is the simplest decoder/encoder invariant. Decoder error: `Error::ConflictingPathDeclarations` on encounter. Encoder default: emit per-`@N` only if any `@N` actually differs; otherwise fall back to SharedPath. Two decision rules:

- **Encoder side:** "if any policy `@N` has a different origin path, emit per-`@N`; otherwise emit SharedPath."
- **Decoder side:** "exactly one of `SharedPath` or `OriginPaths` MUST be present in the path-declaration position."

### Q4 — Header flag bit

`Tag::Fingerprints` is gated by header bit 2 (the only currently-allocated extension flag). Do we need a header flag for the per-`@N` block, or does its presence in the tag stream signal itself?

**Option A: Tag presence signals itself; no header flag needed.** The decoder sees `Tag::OriginPaths` at position 1 (after header) instead of `Tag::SharedPath`, distinguishes by tag byte.

**Option B: Header bit 3 = "OriginPaths flag" (parallel to Fingerprints).** Decoder checks header bit before parsing path declaration.

**My take:** **B — header bit 3.** Symmetry with `Fingerprints`. Costs nothing (header bit was reserved anyway). Makes future additions easier — pattern matches "every variable section is gated by a header flag, present-or-absent at known offsets." Also: tag-byte-only signaling is fragile if the bytecode parser ever needs to skip ahead (e.g., for streaming parse). Header-flag signaling is robust to that.

**Sub-question:** can header bit 3 be used to signal the **presence-or-absence** of any path declaration whatsoever (allowing zero-path encodings for "policy has no key origins" cases — irrelevant to current scope, but does the bit mean "OriginPaths present (vs SharedPath)" or "policy has any path info at all"?)? **My take:** the bit means specifically "OriginPaths block present" (the path-declaration position is mutually-exclusive Q3 Option A). Default (bit clear) = SharedPath in path-decl position. Set = OriginPaths in path-decl position. Both rule out the "no path at all" case, which v0.x already disallows.

### Q5 — Authority precedence semantics with mk1

mk1's BIP §"Authority precedence" already pins:
- mk1's `origin_path` is **authoritative**.
- md1's per-`@N` path is **descriptive** (expected).
- Mismatch → recovery orchestrator rejects.

**Sub-question:** is anything additional needed on the md1 wire-format side? **My take:** no. The semantics are fully specified by mk1's prose. md1 just emits the expected path; consistency-checking is orchestrator-level.

**Sub-question 2:** should md1's BIP §"Per-`@N` path declaration" cite mk1's authority-precedence subsection by URL/reference? **My take:** yes — symmetric cross-reference (md1 says "authoritative semantics live in mk1 §5.1," mk1 already says "wire-format byte allocation lives in md1's BIP"). Closes the ambiguity loop.

### Q6 — Interaction with `Tag::Fingerprints` (0x35)

`Tag::Fingerprints` is already a per-`@N` block (one fingerprint per placeholder). If we add per-`@N` paths, are these two blocks separate or merged?

**Option A: Separate blocks.** Keep `Tag::Fingerprints` exactly as-is. `Tag::OriginPaths` is a parallel sibling block. Order on wire: `[header] [path-decl: SharedPath OR OriginPaths] [Fingerprints]? [tree]`.

**Option B: Merged "OriginInfo" block** combining (fingerprint, path) per `@N`. New tag `Tag::OriginInfo`; old `Tag::Fingerprints` deprecated for v1.0 cleanup but kept for v0.x compat.

**Option C: Per-`@N` path block carries *both* fingerprint and path inline,** dropping `Tag::Fingerprints` entirely from v0.10+.

**My take:** **A — keep separate.** Fingerprints already shipped with its own block in v0.7 / v0.8 corpora; merging means re-encoding existing test vectors and backwards-compatibility complexity. Separate blocks are cleaner: each has its own header flag bit, each is independently optional, parsing is non-coupled. The tradeoff is wire size — a policy with both fingerprints AND per-`@N` paths uses two tags (1+1 byte tag-and-count overhead duplicated). For a 3-cosigner multisig that's 2 bytes of overhead — negligible.

### Q7 — PolicyId / Tier-3 hash impact (NEW — surfaced during brainstorm)

The Tier-3 `PolicyId = SHA-256(canonical_bytecode)[0..16]`. If per-`@N` paths are part of canonical bytecode, then **two policies that differ only in per-`@N` paths produce different PolicyIds**.

**Implication:** mk1's policy-id stub on a key card identifies the *policy template + per-`@N` path layout*, not just the script template. A rotation of cosigner accounts (`m/48'/0'/0'/2'` → `m/48'/0'/0'/3'` for one cosigner) breaks PolicyId match — even though the script and threshold are unchanged.

**Two design routes:**

**Route X: Per-`@N` paths included in canonical bytecode (PolicyId-affecting).** Maximum determinism — same policy + same paths → same PolicyId. Loses the "PolicyId identifies just the template" framing. mk1's policy-id stub becomes more sensitive.

**Route Y: Per-`@N` paths excluded from canonical bytecode used for PolicyId.** PolicyId unchanged across path layouts; per-`@N` paths are "metadata alongside the canonical hash," similar in spirit to how fingerprints today are gated by a flag bit (bit 2) and historically were considered "outside the policy template" — though this isn't quite literally true because fingerprints DO affect canonical_bytecode currently.

Wait — let me check that. **Verify:** does `to_bytecode` include the fingerprints block in the bytes hashed for PolicyId?

If it does, then "fingerprints are part of canonical bytecode" is the existing precedent, and per-`@N` paths should follow the same rule (Route X — PolicyId-affecting).

If it doesn't, the existing precedent is "fingerprints are excluded from PolicyId" and per-`@N` paths could follow that pattern (Route Y).

**This is the highest-stakes sub-question and needs verification before we proceed.** Filed as a brainstorm-action below.

### Q8 — Path component count cap

mk1 caps path component count at 10 (D-Q3). md1 currently has its own internal cap (need to verify — likely from `bytecode/path.rs::MAX_PATH_COMPONENTS`).

**Sub-question:** should md1's per-`@N` paths and md1's `SharedPath` share the same cap? Should md1's cap match mk1's 10? **My take:** yes — they should match. mk1 inherits md1's path encoding by mirror clause; cap divergence is a hazard. If md1's cap is currently different, this is a v0.10 alignment opportunity.

### Q9 — Encoder default behavior

When user calls `encode(policy, EncodeOptions::default())` with a policy that has divergent per-`@N` paths, what does v0.10 do?

**Option A: Auto-detect and emit per-`@N` block when needed.** Default behavior changes silently between v0.9 (lossy — drops per-`@N` info) and v0.10 (preserves it).

**Option B: Require explicit opt-in via `EncodeOptions::with_per_at_n_paths(true)`.** Default v0.10 still emits SharedPath only; user opts in to richer encoding.

**Option C: Auto-detect, but emit `Error::PolicyScopeViolation` when policy has divergent paths and auto-encoding isn't enabled.** Forces user to be explicit.

**My take:** **A — auto-detect.** Reasoning: a v0.x policy that fit single-shared-path was lossy (it silently dropped per-`@N` info). v0.10 should fix that bug-class by default. Migration story: existing v0.x callers see no behavior change for policies with shared paths; callers with previously-divergent paths now get correct encodings. The opt-in (B) feels like premature caution — if the encoder *can* preserve information, it *should*.

**Counter-argument:** auto-detect makes round-trip tests harder to reason about across v0.9 ↔ v0.10. **Counter-counter:** that's exactly the wire-format-bump signal SemVer is for.

### Q10 — Migration story for old encodings

Old v0.x encodings (with only `SharedPath`) decode under v0.10 unchanged — wire-additive at the decoder for `SharedPath`-only inputs. The header bit 3 is 0; the decoder routes to SharedPath path-decl as before.

**Sub-question:** does v0.10 emit the same SharedPath-only bytecode for shared-path policies as v0.9? **My take:** yes — byte-identical for the subset. This means:
- Test vectors regenerated under "md-codec 0.10" family token: most stay byte-identical except for any vectors with previously-non-roundtrippable per-`@N` paths.
- Existing engraved cards remain decodable. v0.10 is **wire-additive** (existing valid bytestreams remain valid), even though its public API may break (`#[non_exhaustive]` Error gains variants, etc.).

**Subtlety:** if Q9-A is chosen (auto-detect), then a policy that v0.9's encoder lossily flattened to SharedPath will under v0.10 emit OriginPaths instead — same input, different output. That's "fixing a bug," not "breaking compat" in the wire-format sense. CHANGELOG should highlight.

### Q11 — Forward-compatibility hooks

After v0.10 ships, what natural extensions follow?

- BIP 393 recovery hints (`Tag::RecoveryHints` at `0x37` post-Q1, gated by a new header flag bit) — birthday hint + gap-limit + max silent-payment label index. v1+ work.
- Per-`@N` extended pubkey carrying (currently mk1's job; could md1 absorb in future v1.x?). Unlikely — that's mk1's domain by design, but worth noting.
- Per-`@N` Wallet-Instance-ID carriage. Also mk1 / orchestrator territory.

**My take:** for v0.10 we close the path question only. Don't speculate further; the `#[non_exhaustive]` posture and reserved bits 0/1 leave room.

### Q12 — Formalize Type 0 / Type 1 PolicyId typology

Surfaced 2026-04-29 user typology framing: PolicyId is best considered as a *type of* an ID. Type 0 = `WalletInstanceId` (template + paths + concrete xpubs; recovery-time computation). Type 1 = `PolicyId` (template + paths; engraved as 12-word phrase).

**Light:** add a BIP §"PolicyId types" subsection naming Type 0 / Type 1 explicitly as teaching aid; keep code names (`PolicyId`, `WalletInstanceId`) unchanged.

**Full:** rename in code (`Type0PolicyId` / `Type1PolicyId`); v0.10 wire-bump justifies the API churn.

**My take:** Light. The typology is useful for reasoning; existing names are descriptive. We just shipped v0.8→v0.9 rename — re-renaming creates churn for marginal clarity gain.

### Q13 — PolicyId UX (engraving optionality + fingerprint API)

Surfaced 2026-04-29 user observation: for short policies the 128-bit / 12-word PolicyId phrase is *longer* than the codex32 md1 string it summarizes. PolicyId isn't part of the codex32 string — it's a separate engraved Tier-3 anchor — so engraving the 12-word phrase is *additional* cost on top of the codex32 string, not redundant copies. For a typical short policy: ~25-30 chars (codex32) + ~50-70 chars (12-word phrase) ≈ 75-100 chars total, of which the 12-word phrase dominates.

**Three coupled sub-decisions:**

(a) **BIP language softening.** Currently the BIP framing implies the 12-word phrase is part of the canonical backup. Soften to: "MAY engrave; SHOULD if maintaining cross-verification with a digital backup of the codex32 string." Pure docs change.

(b) **Fingerprint API.** Add `PolicyId::fingerprint() → [u8; 4]` returning the first 32 bits as 8 hex chars (parallel to BIP 32 master-key fingerprints). Tools display this as a one-line identifier; users who want a *minimum-cost* engraved anchor get an 8-char option. ~10 lines of code; not a wire-format change.

(c) **Canonical PolicyId stays 128 bits / 12 words.** No shrinking. Reasoning: 128 bits is the BIP-39-tool-compatible form; shrinking the canonical breaks the v0.8 anchor semantics for marginal gain. The fingerprint API (b) provides the short form for users who want it.

**My take:** all three (Light bundle). Bundle into v0.10 alongside the per-`@N` path BIP work — same BIP review cycle, no scope expansion for the wire-format changes.

## 4. Resolved actions (A1, A2, A3)

**A1 — RESOLVED: PolicyId is computed via `compute_policy_id_for_policy` which calls `policy.to_bytecode(&EncodeOptions::default())`.** Default `EncodeOptions` has `fingerprints: None`, so the bytecode hashed for PolicyId today excludes the fingerprints block. **Existing precedent: PolicyId is computed over a "minimal canonical" form that excludes optional flag-gated blocks.**

This is interesting because it couples Q7 to Q9 directly:
- Under **Q9-A (auto-detect)**: a policy with divergent per-`@N` paths produces an `OriginPaths` block under default options (since the encoder *can* preserve the info, it does). PolicyId then includes per-`@N` paths.
- Under **Q9-B (opt-in)**: default options omits per-`@N` info; PolicyId is computed over the SharedPath form (lossy fallback); per-`@N` paths are "outside the policy template."

So Q7's resolution depends on Q9. **My take updates:** if we go Q9-A (which I already preferred), Q7 lands at Route X (PolicyId-affecting). If Q9-B, Q7 lands at Route Y (PolicyId-stable).

The semantic implication of Route X (with Q9-A): mk1's policy-id stub on a key card identifies the *policy template + per-`@N` path layout*, not just the script template. Two wallets with the same script but different per-cosigner accounts become different PolicyIds. mk1's BIP §"Naming and identifiers" prose ("Two distinct wallets that share an identical template share a Policy ID") would need updating to "share a template AND identical per-`@N` path layout."

**That's the right semantics**, IMO — two wallets with different per-cosigner accounts really *are* different wallets (different mk1 cards engraved), and shouldn't collide on PolicyId. The v0.8 framing was over-loose because v0.8 didn't carry per-`@N` paths and nothing depended on the distinction. v0.10 closes the loop.

**A2 — RESOLVED: md1 has no explicit `MAX_PATH_COMPONENTS` cap.** `bytecode/path.rs` uses LEB128-encoded varint count for explicit-form paths, with no enforced cap. Wire-level cap is `u64::MAX`; effective cap is bytecode/chunk-size limits.

mk1's 10-component cap is explicit per Q-3. **Decision implication:** v0.10 should align — introduce a `MAX_PATH_COMPONENTS = 10` constant in `bytecode/path.rs`, applied to both `SharedPath` and the new `OriginPaths` block. Wire-additive — old encodings that happen to have ≤10 components remain valid; encodings with >10 (which would only happen for adversarial or BIP-32-violating inputs) would now reject. Practically a no-op for real-world policies (BIP 32 paths rarely exceed 5–6 components).

**A3 — RESOLVED: no existing test vectors carry per-`@N` divergent paths.** All v0.x corpus vectors use single shared paths. v0.10's per-`@N` work is greenfield from the corpus standpoint — no regression vectors to worry about; we just add new positive vectors exercising the new tag.

## 5. For user input

The five original questions resolved (with my take), plus six surfaced:

| # | Question | My take | Status |
|---|---|---|---|
| Q1 | Tag byte allocation | A — `0x36` (next clean) | **LOCKED** (RecoveryHints moves to `0x37` in POLICY_BACKUP.md) |
| Q2 | Encoding shape | A — dense (one path-decl per `@N`) with explicit count prefix | open |
| Q3 | SharedPath coexistence | A — strict mutual exclusion at path-decl position | open |
| Q4 | Header flag bit | B — bit 3 = OriginPaths flag (symmetric with bit 2 = Fingerprints) | open |
| Q5 | Authority precedence with mk1 | No additional md1-side semantics needed; cross-reference mk1 §5.1 | open |
| Q6 | Interaction with Fingerprints | A — separate blocks | open |
| Q7 | PolicyId impact | Route X (per-`@N` paths affect PolicyId) — coupled to Q9-A. mk1 BIP §"Naming and identifiers" prose needs minor update. | **LOCKED** (per Type 0 / Type 1 framing) |
| Q8 | Path component count cap | Add `MAX_PATH_COMPONENTS = 10` to md1 (currently unbounded); aligns with mk1's Q-3. Wire-additive (no real-world policy exceeds 10). | open |
| Q9 | Encoder default | A — auto-detect emit per-`@N` | open |
| Q10 | Migration story | Wire-additive at decoder; auto-detect at encoder per Q9 | open |
| Q11 | Forward-compat hooks | Not in scope; leave room | open |
| Q12 | Type 0 / Type 1 PolicyId typology | Light (BIP teaching subsection; no code rename) | **LOCKED** |
| Q13 | PolicyId UX (engraving + fingerprint) | Light bundle: BIP softens "MAY engrave"; add `PolicyId::fingerprint() → [u8; 4]`; canonical stays 128 bits | **LOCKED** |

**Decisions needed from user:**

1. Approve / adjust each of Q1–Q11 (or wait until A1–A3 resolve).
2. Confirm v0.10.0 axis bump is acceptable (next breaking-change axis).
3. Confirm the workflow: spec → opus review → plan → opus review → phase-by-phase implementation → per-phase opus reviews.

After user input, A1–A3 resolve (cheap greps), spec is written, and the cascade begins.
