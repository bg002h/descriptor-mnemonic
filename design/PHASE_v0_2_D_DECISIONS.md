# Phase v0.2-D — Taproot Tr/TapTree decisions

Resolves the open design questions flagged in `design/IMPLEMENTATION_PLAN_v0.2.md` Phase D, in advance of implementer dispatch.

## D-1 — Single-leaf only (no TapTree wrapper tag emitted)

**Decision**: Phase D ships single-leaf taproot exclusively. Multi-leaf `TapTree` is reserved for v1+.

**Rationale**: The BIP itself constrains v0 (`bip/bip-wallet-descriptor-mnemonic.mediawiki:421-423`):
> "The taproot tree encoding is left as a future refinement: in v0, taproot wallet policies are supported with the constraint that the tree contains at most one leaf, encoded directly as a sub-tree without intermediate `TapTree` nodes."

**Encoding shape** for `tr(internal_key, leaf_ms)`:
- `Tag::Tr` (`0x06`)
- internal key encoding (`Tag::Placeholder` + index byte)
- leaf miniscript encoded directly, **without** wrapping in a `Tag::TapTree` (`0x08`) node

**Encoding shape** for key-path-only `tr(internal_key)`:
- `Tag::Tr` (`0x06`)
- internal key encoding (`Tag::Placeholder` + index byte)
- *(end of Tr's bytecode region — no leaf bytes follow)*

**Tag::TapTree (`0x08`) status**: stays reserved in `tag.rs` (already there). Decoder MUST reject `0x08` with `Error::PolicyScopeViolation("multi-leaf TapTree reserved for v1+")`. Encoder never emits it.

## D-2 — Per-leaf miniscript subset enforcement at BOTH encode and decode

**Decision**: enforce the Coldcard subset (`pk` / `pk_h` / `multi_a` / `or_d` / `and_v` / `older`) at **both** encode time AND decode time.

**Rationale**: The BIP MUST clause (`bip/bip-wallet-descriptor-mnemonic.mediawiki:425`):
> "Implementations supporting taproot MUST also enforce the per-leaf miniscript subset constraints required by deployed hardware signers (notably Coldcard, which restricts to `pk`, `pk_h`, `multi_a`, `or_d`, `and_v`, `older` as of edge firmware)."

The MUST is unambiguous. Both directions cheap (one tree walk each):
- **encode time** catches authoring errors (caller builds a `Tr` with an out-of-subset leaf)
- **decode time** catches malicious or malformed inputs (someone hand-crafts a bytecode whose leaf decodes to an out-of-subset miniscript)

**Implementation**: a single `fn validate_tap_leaf_subset(ms: &Miniscript<DescriptorPublicKey, Tap>) -> Result<(), Error>` in `bytecode/encode.rs` (or a new `bytecode/tap_subset.rs` module if cleaner). Recursive walk over the `Terminal` AST; reject any operator outside the subset with a new `Error::TapLeafSubsetViolation { operator: String }` (or reuse `PolicyScopeViolation` with a descriptive message).

**Subset definition** — exhaustively, the allowed `Terminal` variants:
- `Terminal::PkK(_)` — `pk(key)`
- `Terminal::PkH(_)` — `pk_h(key)` (note: `RawPkH` still rejected as it's a non-policy form)
- `Terminal::MultiA(thresh, keys)` — `multi_a(k, ...keys)`
- `Terminal::OrD(_, _)` — `or_d(left, right)`
- `Terminal::AndV(_, _)` — `and_v(left, right)`
- `Terminal::Older(_)` — `older(n)`

Plus wrapper terminals if they wrap an allowed inner: `Verify`, `Check`, `NonZero` etc. — Phase D agent should sanity-check whether wrappers are subset-allowed by Coldcard's actual implementation (consult Coldcard docs or BIP discussions). If wrappers are ambiguous, default to **rejecting** them in v0.2 and adding an entry to FOLLOWUPS for clarification.

## D-3 — Bytecode region delimiter for "no leaf" case

**Decision**: at top level, end-of-bytecode means "no leaf"; the decoder reads `Tag::Tr` + internal key, then checks the cursor: if at end → key-path only; else → continue parsing as the leaf miniscript.

**Rationale**: at the top level the bytecode payload is the entire policy. A nested `Tr` (which v0.2 doesn't support) would need an explicit length, but single top-level `Tr` doesn't.

This means **`Tr` cannot appear nested inside another operator** in v0.2 — top-level only, same way `Wsh` is top-level only. Decoder rejects nested `Tr` with the same `PolicyScopeViolation` it already uses for misplaced operator tags.

## D-4 — `multi_a` already-shipped arms (Phase 2 carry-forward)

**Decision**: keep the existing `multi_a` arms in `bytecode/encode.rs:178` and `bytecode/decode.rs:222` (Phase 2 / Task 2.4 shipped them in advance for exactly this Phase D moment). Verify they do the right thing for taproot context (`Tap` instead of `Segwitv0`).

The Phase 2 Task 2.4 author explicitly anticipated Phase D ("ship the arm in case Tr is enabled later"). Phase D should not re-implement; just verify and exercise via tests.

## D-5 — `Bare`, `Wpkh`, `Pkh`, `Sh` rejection still in force

**Decision**: Phase D unblocks `Tr` only. The other reject sites in `decode.rs:58` (`Tag::Sh | Tag::Pkh | Tag::Wpkh | Tag::Bare`) and the corresponding encode-side rejections remain. They are out of v0.2 scope per BIP §3 (only `wsh()` and `tr()` are supported script types).

## Spec edits required (BIP draft)

In `bip/bip-wallet-descriptor-mnemonic.mediawiki`:

1. **Heading rename** (line 421): `====Taproot tree (forward-defined)====` → `====Taproot tree====` (drop "forward-defined" — v0.2 implements it).
2. **Tag table entry for `0x08`** (line 314): clarify that `0x08` is reserved-for-v1+ in v0.2 (multi-leaf TapTree); v0.2 single-leaf encodes the leaf miniscript directly without `0x08`.
3. **Add concrete encoding examples** to §"Taproot tree" showing key-path-only `tr(K)` and single-leaf `tr(K, ms)` byte layouts.
4. **Subset clause** (line 425): keep as-is; the implementation now enforces it.

Phase D agent applies these BIP edits in the same commit as the implementation.

## Out of scope (deferred)

- Multi-leaf `TapTree` encoding (depth-aware Merkle-path bits, depth-first leaf list with `Tag::TapTree` framing) — v1+, tracked via a new FOLLOWUPS entry filed by the Phase D agent if the wrapper-terminal subset question above also defers to v1+.
- Internal-key choice beyond `Tag::Placeholder` references (e.g., `unspendable()` — BIP-341 NUMS-point convention) — Phase D may surface this; if so, file a v0.3 FOLLOWUPS entry rather than expand scope.
- Tap-miniscript-specific type-check rules beyond the subset filter (Coldcard may enforce more than the subset names; full type-check parity is out of v0.2 scope unless the agent finds a concrete gap).
