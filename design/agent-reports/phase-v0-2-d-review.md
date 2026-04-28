# Phase D review — Opus 4.7

**Status:** APPROVE_WITH_FOLLOWUPS
**Subject:** commit `6f6eae9` (`p2-taproot-tr-taptree`, cherry-picked from worktree `267036f`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec compliance + algorithmic + BIP-edit + test coverage + code quality
**Role:** reviewer

## Findings

### Spec deviations (vs `design/PHASE_v0_2_D_DECISIONS.md`)

(none) — D-1 through D-5 all honored.

- **D-1 (single-leaf only)**: encoder rejects multi-leaf at `encode.rs:127-131` and non-zero-depth at `encode.rs:133-138`; decoder rejects `Tag::TapTree=0x08` in two places (`decode.rs:181-186` and `decode.rs:594-598`). No `Tag::TapTree` is ever emitted.
- **D-2 (subset enforced both directions)**: `validate_tap_leaf_subset` at `encode.rs:480-503` recurses through `AndV`/`OrD` children and `Check`/`Verify` wrapper children; called at encode (`encode.rs:142`) and decode (`decode.rs:194`). Wrapper choice (`c:`/`v:` allowed; `a:`/`s:`/`d:`/`j:`/`n:` rejected) matches the report rationale.
- **D-3 (top-level Tr only)**: top-level Tr dispatched only from `decode_descriptor` (`decode.rs:64`); mid-tree `Tag::Tr` falls through `decode_terminal` catch-all → `PolicyScopeViolation`. Tap-context recursion never re-dispatches `Tag::Tr` either.
- **D-4 (multi_a Phase 2 arms preserved)**: existing arms intact and now exercised by `taproot_single_leaf_multi_a_round_trips`.
- **D-5 (other top-level rejections)**: `Tag::Sh | Tag::Pkh | Tag::Wpkh | Tag::Bare` rejection intact at `decode.rs:65-67`.

### Algorithmic correctness

- Tap-context typing correct: `Miniscript<DescriptorPublicKey, Tap>` and `Terminal<DescriptorPublicKey, Tap>` (encode.rs:357-376; decode.rs:493-501). Segwitv0 path unchanged.
- `Cursor::is_empty()` and `peek_byte()` (cursor.rs:103-117) off-by-one correct. `peek_byte` does not advance.
- `Tr::new(internal_key, tap_tree)` called with `Some(TapTree::leaf(leaf))` for single-leaf and `None` for key-path-only (decode.rs:175-189) — matches upstream constructor signature.
- Subset validator recursion complete; no reachable hole.
- Nested-Tr rejection structurally guaranteed (no `Tag::Tr` arm in inner-context dispatchers — falls through catch-all).
- No new `unwrap`/`panic!`/`unreachable!` in production paths; all such occurrences in `#[cfg(test)] mod tests`.

### BIP-edit correctness

- Heading at line 421 renamed: `====Taproot tree (forward-defined)====` → `====Taproot tree====`. Verified.
- Tag table at line 314: `0x08` clarified as reserved for v1+ multi-leaf; v0 single-leaf direct-encoding rule documented. Verified.
- Lines 423-425: rewritten to specify wire layout + explicit "MUST reject `0x08` in tap-leaf position" clause. Subset clause preserved + expanded to call out `c:`/`v:` wrappers.
- **Byte-layout examples (lines 433-482) reproduce EXACTLY** via live encoder: `003303063200`, `0033030632000c1b3201`, `0033030632001a0203320132023203`, `003303063200160c1b3201110e1f90010c1b3202`. Annotations accurate.

### Test coverage

All 9 dispatch scenarios present in `tests/taproot.rs`:
1. `tr(K)` key-path-only round-trip (line 38)
2. `tr(K, pk(K2))` single-leaf pk (line 50+)
3. `tr(K, multi_a(2, K0, K1, K2))` single-leaf multi_a (line 60+)
4. `tr(K, or_d(pk, and_v(v:older, pk)))` nested allowed (line 75+)
5. `tr(K, sha256(...))` rejection (line 97+)
6. wrapper rejection scenarios (line 120+)
7. multi-leaf TapTree decode rejection (line 144+)
8. nested-Tr decode rejection (line 166+, synthesized via wsh-byte splice)
9. corpus fixtures correctly DEFERRED to Phase F (per agent report); `vectors.rs` unchanged; `gen_vectors --verify v0.1.json` byte-stable
- Conformance gate: `rejects_tap_leaf_subset_violation` registered for the exhaustiveness mirror.

### Quality blockers

(none)

### Quality important

(none)

### Quality nits (3)

- **N-1**: decode-side error operator naming in `decode_tap_terminal` (`decode.rs:603`) uses `format!("{:?}", other)` on `Tag` (PascalCase: `"Sha256"`, `"Thresh"`), whereas encode-side `tap_terminal_name` uses BIP 388 lowercase (`"sha256"`, `"thresh"`). User-facing diagnostics differ between encode and decode rejection of the same operator. **(Filed as `phase-d-tap-decode-error-naming-parity`.)**
- **N-2**: hand-crafted nested `Tag::Tr` inside a tap leaf surfaces `TapLeafSubsetViolation { operator: "Tr" }` rather than `PolicyScopeViolation` (decode.rs:601-604). Semantically the violation is "wrong scope," not "wrong subset." Cosmetic; doesn't affect rejection correctness.
- **N-3**: `Tag::TapTree` rejection duplicated in two places (decode.rs:181-186 and 594-598). Minor; consistent message.

## Disposition

| Finding | Action |
|---|---|
| D-1..D-5 | All honored — no action |
| Algorithmic correctness | Sound — no action |
| BIP-edit correctness | Verified via live encoder — no action |
| Test coverage | All 9 scenarios present — no action |
| N-1 (encode/decode error naming parity) | New FOLLOWUPS: `phase-d-tap-decode-error-naming-parity` (v0.2-nice-to-have) |
| N-2 (TapLeafSubsetViolation vs PolicyScopeViolation for nested Tr) | Acknowledged; cosmetic; no action |
| N-3 (TapTree rejection duplication) | Acknowledged; deduplicating would obscure both call sites; no action |

Plus 3 entries proposed by the implementer agent:
- `phase-d-tap-leaf-wrapper-subset-clarification` (v0.3 — broader wrapper-set widening if signers document broader safe support)
- `phase-d-taproot-corpus-fixtures` (v0.2 Phase F — corpus + vector additions deferred)
- `phase-d-tap-miniscript-type-check-parity` (v0.3 — full Tap-context type-check rules beyond the named subset)

## Verdict

APPROVE_WITH_FOLLOWUPS — Phase D clear; D-1 through D-5 all honored.
