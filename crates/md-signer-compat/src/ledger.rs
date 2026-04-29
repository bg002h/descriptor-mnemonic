//! Ledger tap-leaf miniscript subset.

use crate::SignerSubset;

/// Ledger tap-leaf miniscript subset.
///
/// **Source:** `LedgerHQ/vanadium`, `apps/bitcoin/common/src/bip388/cleartext.rs`.
/// Verified on 2026-04-28.
///
/// Variants admitted (from the `cleartext.rs` enum):
///   - `Singlesig` (key-only `tr`)
///   - `SortedMultisig` (`sortedmulti_a`)
///   - `Multisig` (`multi_a`)
///   - `RelativeHeightlockMultiSig` (`and_v(v:multi_a, older(n<65536))`)
///   - `RelativeTimelockMultiSig` (`and_v(v:multi_a, older(time-encoding range))`)
///   - `AbsoluteHeightlockMultiSig` (`and_v(v:multi_a, after(n<500_000_000))`)
///   - `AbsoluteTimelockMultiSig` (`and_v(v:multi_a, after(n>=500_000_000))`)
///
/// Operators extracted (desugared-AST naming):
///   - `pk_k` (single-sig keypath)
///   - `pk_h`
///   - `multi_a`
///   - `sortedmulti_a`
///   - `and_v`
///   - `older`
///   - `after`
///   - `c:` (for sugar desugaring)
///   - `v:` (for `and_v(v:..., ...)`)
pub const LEDGER_TAP: SignerSubset = SignerSubset {
    name: "Ledger tap-leaf (LedgerHQ/vanadium as of 2026-04-28)",
    allowed_operators: &[
        "pk_k",
        "pk_h",
        "multi_a",
        "sortedmulti_a",
        "and_v",
        "older",
        "after",
        "c:",
        "v:",
    ],
};
