# v0.10 spec review (opus, pass 2)

**Date:** 2026-04-29
**Spec:** design/SPEC_v0_10_per_at_N_paths.md (commit 82f763e)
**Reviewer:** opus-4.7 (pass 2, focused on pass-1 finding verifications)

## Summary

**Verdict: minor-fixes.** Eleven of the fifteen pass-1 findings landed cleanly. F1 (byte sequence), F2 (`decoded_origin_paths`), F3 (truncation variant), F6 (count=0), F7 (walker test dropped), F11 (mutual exclusion invariant), F12 (o3 reshape), F14 (Appendix Q2 lift), F15 (test rewrite note) — all fully verified. F8/F9/F10/F13 confirmed as folded/deferred per implementer's status block.

But **F5 only partially landed**: the implementer correctly removed the `Error::ConflictingPathDeclarations` definition from §4 and updated the §3 dispatch sketch to use `BytecodeErrorKind::UnexpectedTag`, but two stale references survive — line 97 in §2 prose still says the conflict surfaces as `Error::ConflictingPathDeclarations`, and the §6 wire-format-break summary table at line 492 still lists `Error::ConflictingPathDeclarations` as a new v0.10 variant. These are pure search-and-replace misses, fixable in seconds.

Additionally, **F4's redirect of the structural error variant didn't propagate into the §4 encoder code sketch** (lines 312–315): the encoder still uses `Error::OriginPathsCountMismatch` for both the `u8::try_from` overflow and the explicit `count_u8 > 32` guard. Both should be `BytecodeErrorKind::OriginPathsCountTooLarge` (or, more naturally for the encoder side, an internal-invariant violation since BIP 388 already caps placeholder count upstream of `to_bytecode` — see F16 below). This is the F4 fix not propagating cleanly through the encoder's mirror of the same structural check.

Two new findings (F16, F17) below — neither is a wire-format-affecting concern; both are internal-consistency cleanups before plan-writing.

## Pass-1 finding verifications

- **F1 (byte sequence)** — ✅ verified. §2 lines 147–153 derive `m/48'/0'/0'/100'` correctly: count=4 → `0x04`, `48'` → 97 = `0x61`, `0'` → 1 = `0x01`, `0'` → `0x01`, `100'` → 201 = LEB128 `0xC9 0x01`. Full path: `FE 04 61 01 01 C9 01` (7 bytes). Examples B (line 157) and C (line 170) both show the corrected sequence. Block-size accounting at line 164 ("OriginPaths block size: 11 bytes (1 tag + 1 count + 1 + 1 + 7 explicit)") is right: `36 03 05 05 FE 04 61 01 01 C9 01` is exactly 11 bytes.

- **F2 (decoded_origin_paths field)** — ✅ verified. §4 lines 269–283 introduce the "Round-trip stability — `decoded_origin_paths` field" subsection with the rationale (without it, decode→encode loses divergence and re-emits SharedPath). Field is documented on `WalletPolicy`, populated by `from_bytecode`, and consulted as Tier 1 in the precedence chain. The §4 "Encoder per-`@N`-path precedence chain" (lines 286–294) explicitly enumerates the four tiers (Tier 0 override, Tier 1 `decoded_origin_paths`, Tier 2 KIV, Tier 3 shared-path fall-through). Mutual-exclusion invariant stated at line 283.

- **F3 (truncation error variant)** — ✅ verified. §5 line 453 (`n_orig_paths_truncated`) now references `BytecodeErrorKind::UnexpectedEnd`, matching the existing `decode_path` / `decode_declaration` convention. Comment at §3 line 233 is consistent ("`BytecodeErrorKind::UnexpectedEnd` on cursor exhaustion").

- **F4 (count split)** — ⚠️ partially landed. The split itself is in: §1 item 5 (lines 53–57) lists both `BytecodeErrorKind::OriginPathsCountTooLarge` (structural) and `Error::OriginPathsCountMismatch` (semantic) with the structural-vs-semantic rationale. §3 `decode_origin_paths` (lines 219–236) correctly uses `BytecodeErrorKind::OriginPathsCountTooLarge { count, max: 32 }` for the structural reject. §3 line 239 calls out the v0.6 strip-Layer-3 alignment. **However**, the §4 encoder code sketch (lines 312–315) uses the wrong variant — see F16 below.

- **F5 (UnexpectedTag reuse)** — ⚠️ partially landed. §3 dispatch (lines 200–214) correctly emits `BytecodeErrorKind::UnexpectedTag { expected, got }` for the conflict case, with explanatory note. §4 "Error updates" no longer defines `Error::ConflictingPathDeclarations` (line 394 explicitly notes it was dropped). §5 `n_conflicting_path_declarations` (line 456) correctly uses the new shape. **However**, two stale references survive: line 97 in §2 ("Encountering `Tag::OriginPaths (0x36)` when bit 3 = 0 ... is `Error::ConflictingPathDeclarations`.") and line 492 in the §6 wire-format-break summary table (`Error::ConflictingPathDeclarations` listed as exists-no→yes). Both must be deleted/rewritten — see F17 below.

- **F6 (count=0 / count too large)** — ✅ verified. §2 line 108 specifies "MUST be in `1..=32`". §3 line 221 rejects both `count == 0` and `count > 32` with the same structural variant. §5 has both `n_orig_paths_count_zero` (line 454) and `n_orig_paths_count_too_large` (line 455).

- **F7 (walker test dropped)** — ✅ verified. §5 hand-AST coverage list (lines 463–468) makes no mention of `origin_paths_walker_reports_first_violation`. Line 474 has the explicit justification (flat sequential loop with early `?` return — assertion would be trivially true).

- **F8 (encoder try_from cleanup)** — folded per implementer's status block. The current encoder sketch is internally inconsistent (see F16) but that's downstream of F4, not F8.

- **F9 (cap-in-encode_path)** — ✅ verified per implementer's status block; §4 line 333 and §3 line 232 are consistent that `encode_path` / `decode_path` enforce `MAX_PATH_COMPONENTS` internally for both `Tag::SharedPath` and `Tag::OriginPaths` reuse.

- **F10 (BytecodeHeader builder API)** — deferred per status block; acceptable.

- **F11 (mutual exclusion invariant)** — ✅ verified. §4 line 283 has the "Invariant: at most one of `decoded_shared_path` and `decoded_origin_paths` is `Some`" line, with defense-in-depth check noted.

- **F12 (o3 reshape)** — ✅ verified. §5 line 443 references `o3_wsh_sortedmulti_2of4_divergent_paths` with the correct multi-key wsh shape. Parenthetical correctly attributes the reshape to F12 and identifies the original pkh-only-1-placeholder category error.

- **F13 (confirmation)** — ✅ no change required, confirmed accurate.

- **F14 (Appendix Q2 lift)** — ✅ verified. Appendix A item 2 (line 576) is struck-through and marked "Resolved at spec time per opus review F14", with a pointer to §4's precedence chain.

- **F15 (test rewrite migration note)** — ✅ verified. §6 lines 558–560 have a dedicated subsection "Existing tests affected by `MAX_PATH_COMPONENTS = 10`" describing the `decode_path_round_trip_multi_byte_component_count` rewrite from component-count to child-index dimension.

## New findings

### F16: Encoder code sketch in §4 still uses `OriginPathsCountMismatch` for the structural reject

**Severity:** strong (blocks plan-writing because it locks-in an incorrect error path on the encoder side)
**Location:** §4 lines 312–315

The encoder sketch reads:

```rust
let count_u8 = u8::try_from(placeholder_paths.len())
    .map_err(|_| Error::OriginPathsCountMismatch { expected: ..., got: placeholder_paths.len() })?;
if count_u8 > 32 {
    return Err(Error::OriginPathsCountMismatch { ... });
}
```

But F4 redirected the *structural* count-too-large case to `BytecodeErrorKind::OriginPathsCountTooLarge`, not `OriginPathsCountMismatch`. The latter is the *semantic* check ("tree has N placeholders, OriginPaths declares M"), which by definition doesn't apply at the encoder — the encoder *generates* the OriginPaths block from a known tree, so a count-vs-tree mismatch is impossible (it'd be an internal-invariant break, not a user-visible error).

Two correct shapes:
1. **Structural-error consistency.** Use `BytecodeErrorKind::OriginPathsCountTooLarge` for both the `try_from` overflow and the `> 32` check, matching the decoder side.
2. **Internal-invariant assertion.** Since BIP 388 caps placeholder count at 32 *before* `to_bytecode` (the policy-construction layer), the encoder can `expect(...)` after `u8::try_from`, treating any failure as a programming error. This matches the existing fingerprints-block emission pattern in `policy.rs::to_bytecode` (lines 423–427 per pass-1 F8).

**Recommendation:** Pick (2) — `expect()` is more honest about where the cap is enforced (upstream at policy construction) and keeps user-visible errors on the decoder side only. If the team prefers belt-and-suspenders, use (1) — but never `OriginPathsCountMismatch`, which doesn't apply at the encoder layer.

### F17: Two stale `Error::ConflictingPathDeclarations` references survive F5

**Severity:** strong (semantic contradiction — the spec says "use `BytecodeErrorKind::UnexpectedTag`" in §3 + §4 + §5 but says "use `Error::ConflictingPathDeclarations`" in §2 + §6)
**Location:** §2 line 97 and §6 line 492

Line 97 (§2 wire-format prose):

> Strict mutual exclusion: header bit 3 dispatches the path-decl tag. Encountering `Tag::OriginPaths (0x36)` when bit 3 = 0 (or `Tag::SharedPath (0x34)` when bit 3 = 1) is `Error::ConflictingPathDeclarations`.

Line 492 (§6 wire-format-break summary table):

> `| Error::ConflictingPathDeclarations` exists | no | yes |`

Both must be updated/deleted. Suggested replacements:

- **Line 97:** Change to "...is rejected as `Error::InvalidBytecode { offset: 1, kind: BytecodeErrorKind::UnexpectedTag { expected: <0x34 or 0x36 per the bit>, got: <other> } }`. See §3 'Path-decl dispatch' for the dispatch logic."
- **Line 492:** Delete the row entirely. The variant doesn't exist in v0.10. (No replacement row needed — `BytecodeErrorKind::UnexpectedTag` is pre-existing in v0.x and isn't a v0.10 addition.)

These are pure cleanup misses from F5; the implementer caught the §3 + §4 + §5 + status-block instances but missed §2 prose and §6 table.

## Confirmations

- **Internal consistency of the precedence chain.** The 4-tier chain in §4 (Tier 0 override → Tier 1 `decoded_origin_paths` → Tier 2 KIV → Tier 3 shared-path fall-through) is referenced consistently across §4 (introduction), Appendix A item 2 (resolution pointer), and the self-review checklist. No drift.

- **Precedence-chain → encoder dispatch flow is sound.** §4's `placeholder_paths_in_index_order()` is the entry point; the encoder dispatch (lines 300–321) consumes its return value and applies the all-paths-agree check. The Tier 0–3 chain is described in prose; the code sketch consumes the chain output via the helper. Coupling is clean.

- **Structural-vs-semantic error split is internally consistent on the decoder side.** §1 item 5, §3 `decode_origin_paths`, §3 line 239 commentary, §4 error-updates section, and §5 negative-vector list all agree: structural goes to `BytecodeErrorKind::OriginPathsCountTooLarge`, semantic goes to `Error::OriginPathsCountMismatch`. The only inconsistency is the encoder side (F16).

- **No CLI/JSON-output considerations introduced by the new error split.** `BytecodeErrorKind::OriginPathsCountTooLarge` and `Error::OriginPathsCountMismatch` both follow the existing `BytecodeErrorKind::* / Error::*` two-layer convention from v0.6 strip-Layer-3 — CLI/JSON consumers already handle this split for prior error variants. No new spec wording needed.

- **Block-size accounting in Example B.** 1+1+1+1+7 = 11 bytes is correct: tag (1) + count (1) + path_decl_0 = `0x05` (1) + path_decl_1 = `0x05` (1) + path_decl_2 = explicit `FE 04 61 01 01 C9 01` (7).

- **Wire-format header byte set `{0x00, 0x04, 0x08, 0x0C}` matches the bit-2/bit-3 cross product** and is consistent with the new `RESERVED_MASK = 0x03`.

- **No new ambiguity introduced by the F2 / F4 / F5 / F11 / F14 revisions.** The pass-1 fixes are surgical and the precedence chain + invariant statements compose cleanly.

## Open questions for the implementer

1. **F16 resolution: `expect()` vs `BytecodeErrorKind::OriginPathsCountTooLarge`?** Reviewer prefers `expect()` (the cap is enforced upstream at policy construction; `to_bytecode` failure here would be a logic bug, not a user-visible error path) but either is defensible.

2. **Should F17's line-97 rewrite cite the §3 dispatch logic explicitly?** Pointing readers to "§3 'Path-decl dispatch' for the dispatch logic" prevents the reader from having to chase the symbol. Mild preference for the explicit cross-ref.

3. **(Optional polish, not a finding.)** The §4 encoder code sketch's Tier 0–3 precedence-chain entry point `placeholder_paths_in_index_order()` is described as a `WalletPolicy` method but the prose at line 287 says it "consults" `EncodeOptions::origin_paths` (Tier 0). Ensure the actual signature in the plan uses `&EncodeOptions`, not just `&self`. The call site at line 301 passes no options, so either the signature needs an `opts` arg or the helper consults `self.encode_options()` somehow. Plan-time concern, not a spec defect.

4. **Is the test-vector count delta (44 → 45 base, optional 46/47) consistent with the v0.7 family-token-roll convention?** §5 line 445 says vector count grows; verify that the existing `o*` / `n_orig_*` naming doesn't collide with current corpus naming. (Plan-time check; no spec change needed.)
