//! Coldcard tap-leaf miniscript subset.

use crate::SignerSubset;

/// Coldcard tap-leaf miniscript subset.
///
/// **Source:** `Coldcard/firmware` repo, `edge` branch, `docs/taproot.md`
/// §"Allowed descriptors". Verified at edge HEAD on 2026-04-28.
///
/// Documented allowed shapes (per `docs/taproot.md`):
///   - `tr(key)` — single-sig keypath
///   - `tr(internal_key, sortedmulti_a(2, @0, @1))`
///   - `tr(internal_key, pk(@0))`
///   - `tr(internal_key, {sortedmulti_a(...), pk(@2)})`
///   - `tr(internal_key, {or_d(pk(@0), and_v(v:pkh(@1), older(1000))), pk(@2)})`
///
/// Operators extracted (desugared-AST naming):
///   - `pk_k` (from `pk(K)` desugaring + as `pk_k` directly)
///   - `pk_h` (from `pkh(K)` desugaring)
///   - `sortedmulti_a` (Coldcard's documented multisig flavour for tap leaves)
///   - `or_d`
///   - `and_v`
///   - `older`
///   - `c:` (required for `pk(K)` and `pkh(K)` desugaring)
///   - `v:` (required for `and_v(v:..., ...)` and `v:pkh(...)`)
///
/// **`multi_a` deliberately omitted:** Coldcard's `docs/taproot.md`
/// allowed-descriptors list documents only `sortedmulti_a` for tap-leaf
/// multisig — bare `multi_a` is not cited. If a future revision admits
/// `multi_a`, add it back with a citation note.
pub const COLDCARD_TAP: SignerSubset = SignerSubset {
    name: "Coldcard tap-leaf (firmware/edge as of 2026-04-28)",
    allowed_operators: &[
        "pk_k",
        "pk_h",
        "sortedmulti_a",
        "or_d",
        "and_v",
        "older",
        "c:",
        "v:",
    ],
};
