//! Upstream-shapes integration tests (Task 6.14).
//!
//! The descriptor-codec project (CC0) exercises a corpus of 9 policy "shapes"
//! that cover its operator table. This file re-encodes those shapes in BIP 388
//! `@i` placeholder form and verifies each one round-trips cleanly through the
//! MD encoder: `parse → encode → decode → structural-equality check`.
//!
//! # Top-level scope restriction (Phase 2 D-4)
//!
//! MD v0.1 only supports `wsh(...)` at the top level. Bare, Sh, Pkh, Wpkh,
//! and Tr descriptors are rejected by the encoder. Accordingly, every shape
//! here is wrapped in `wsh(...)`.
//!
//! # Shape-to-operator mapping
//!
//! | Test            | Operator tag  | Bytecode tag |
//! |-----------------|---------------|--------------|
//! | `shape_pk`      | `pk(K)`       | `PkK = 0x1B` + `Check = 0x0C` |
//! | `shape_pkh`     | `c:pk_h(K)`   | `PkH = 0x1C` + `Check = 0x0C` |
//! | `shape_multi`   | `multi(k,…)`  | `Multi = 0x19` |
//! | `shape_sortedmulti` | `sortedmulti(k,…)` | `SortedMulti = 0x09` |
//! | `shape_and_v`   | `and_v(V, T)` | `AndV = 0x11` |
//! | `shape_or_d`    | `or_d(B, T)`  | `OrD = 0x16` |
//! | `shape_or_i`    | `or_i(X, X)`  | `OrI = 0x17` |
//! | `shape_andor`   | `andor(B,T,T)` | `AndOr = 0x13` |
//! | `shape_thresh`  | `thresh(k,…)` | `Thresh = 0x18` |

mod common;

// ---------------------------------------------------------------------------
// 1. pk(K) — basic key check (wraps pk_k with c:)
// ---------------------------------------------------------------------------

/// `wsh(pk(@0/**))` — single-key segwit-v0 P2WSH script using `pk`.
///
/// In miniscript, `pk(K)` desugars to `c:pk_k(K)`: the `Check` wrapper (0x0C)
/// applied to `PkK` (0x1B). This is the simplest meaningful MD template.
#[test]
fn shape_pk() {
    common::round_trip_assert("wsh(pk(@0/**))");
}

// ---------------------------------------------------------------------------
// 2. pkh(K) — pubkeyhash inside wsh (c:pk_h form)
// ---------------------------------------------------------------------------

/// `wsh(c:pk_h(@0/**))` — key-hash inside P2WSH using the explicit `c:pk_h`
/// miniscript form.
///
/// Top-level `pkh(...)` is rejected by MD v0.1 (D-4 scope restriction), so
/// we use the equivalent inner-tree form `c:pk_h(K)` which type-checks as a
/// segwit-v0 miniscript B-type fragment. The `Check` (0x0C) + `PkH` (0x1C)
/// bytecode sequence is exercised here.
#[test]
fn shape_pkh() {
    common::round_trip_assert("wsh(c:pk_h(@0/**))");
}

// ---------------------------------------------------------------------------
// 3. multi(k, ...) — k-of-n multisig
// ---------------------------------------------------------------------------

/// `wsh(multi(2,@0/**,@1/**,@2/**))` — 2-of-3 multisig.
///
/// Exercises the `Multi` (0x19) tag, the threshold varint, and the multi-key
/// placeholder sequence. This is the most common real-world MD use case.
#[test]
fn shape_multi() {
    common::round_trip_assert("wsh(multi(2,@0/**,@1/**,@2/**))");
}

// ---------------------------------------------------------------------------
// 4. sortedmulti(k, ...) — BIP 67 sorted multisig
// ---------------------------------------------------------------------------

/// `wsh(sortedmulti(2,@0/**,@1/**,@2/**))` — 2-of-3 BIP 67 sorted multisig.
///
/// Exercises the `SortedMulti` (0x09) tag. BIP 67 key ordering is transparent
/// to the MD bytecode (keys are always identified by placeholder index); the
/// tag signals to the decoder that keys must be sorted before script assembly.
#[test]
fn shape_sortedmulti() {
    common::round_trip_assert("wsh(sortedmulti(2,@0/**,@1/**,@2/**))");
}

// ---------------------------------------------------------------------------
// 5. and_v(V, T) — verify-and conjunction
// ---------------------------------------------------------------------------

/// `wsh(and_v(v:pk(@0/**),pk(@1/**)))` — both keys must sign.
///
/// Exercises the `AndV` (0x11) tag. The left child uses `v:pk(K)` which
/// type-checks as V-type; the right child is `pk(K)` which is T-type. This
/// satisfies the `and_v(V, T)` → T type rule for segwit-v0 miniscript.
#[test]
fn shape_and_v() {
    common::round_trip_assert("wsh(and_v(v:pk(@0/**),pk(@1/**)))");
}

// ---------------------------------------------------------------------------
// 6. or_d(B, T) — or-dup disjunction
// ---------------------------------------------------------------------------

/// `wsh(or_d(pk(@0/**),pk(@1/**)))` — either key may sign.
///
/// Exercises the `OrD` (0x16) tag. `pk(K)` has type B (checksig), which is
/// the required type for both operands of `or_d(B, T)` in segwit-v0.
#[test]
fn shape_or_d() {
    common::round_trip_assert("wsh(or_d(pk(@0/**),pk(@1/**)))");
}

// ---------------------------------------------------------------------------
// 7. or_i(X, X) — or-with-if-branch
// ---------------------------------------------------------------------------

/// `wsh(or_i(pk(@0/**),pk(@1/**)))` — if-branched or-choice between two keys.
///
/// Exercises the `OrI` (0x17) tag. Both operands are B-type `pk(K)` fragments.
/// `or_i` accepts any matching pair and produces the same type as its children.
#[test]
fn shape_or_i() {
    common::round_trip_assert("wsh(or_i(pk(@0/**),pk(@1/**)))");
}

// ---------------------------------------------------------------------------
// 8. andor(B, T, T) — conditional-and-or
// ---------------------------------------------------------------------------

/// `wsh(andor(pk(@0/**),pk(@1/**),pk(@2/**)))` — if @0 signs then @1 must sign,
/// else @2 must sign.
///
/// Exercises the `AndOr` (0x13) tag. The first operand is B-type; the second
/// and third are T-type. All three are `pk(K)` which satisfies these constraints
/// in segwit-v0 miniscript.
#[test]
fn shape_andor() {
    common::round_trip_assert("wsh(andor(pk(@0/**),pk(@1/**),pk(@2/**)))");
}

// ---------------------------------------------------------------------------
// 9. thresh(k, ...) — variable-arity threshold
// ---------------------------------------------------------------------------

/// `wsh(thresh(2,pk(@0/**),s:pk(@1/**),s:pk(@2/**)))` — 2-of-3 threshold.
///
/// Exercises the `Thresh` (0x18) tag, the threshold varint, and the variable-
/// arity child list. The first child is B-type `pk(K)`; subsequent children
/// use `s:pk(K)` (the `Swap` wrapper, 0x0B) to satisfy the `W`-type constraint
/// that `thresh` imposes on all but the first operand in segwit-v0 miniscript.
#[test]
fn shape_thresh() {
    common::round_trip_assert("wsh(thresh(2,pk(@0/**),s:pk(@1/**),s:pk(@2/**)))");
}
