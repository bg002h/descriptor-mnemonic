//! Ledger tap-leaf miniscript subset.

use crate::SignerSubset;

/// Ledger tap-leaf miniscript subset.
///
/// **Source:** `LedgerHQ/vanadium`, `apps/bitcoin/common/src/bip388/cleartext.rs`.
/// Verified on 2026-04-28.
///
/// Representative subset of variants admitted (from the `cleartext.rs`
/// enum). The list below is non-exhaustive — additional variants exist
/// covering single-sig + timelock combinations and combined heightlock/
/// timelock multisig shapes — but every variant in `cleartext.rs` builds
/// from the operator union enumerated below, so the allowlist itself is
/// complete:
///   - `Singlesig` (key-only `tr`)
///   - `SortedMultisig` (`sortedmulti_a`)
///   - `Multisig` (`multi_a`)
///   - `RelativeHeightlockMultiSig` (`and_v(v:multi_a, older(n<65536))`)
///   - `RelativeTimelockMultiSig` (`and_v(v:multi_a, older(time-encoding range))`)
///   - `AbsoluteHeightlockMultiSig` (`and_v(v:multi_a, after(n<500_000_000))`)
///   - `AbsoluteTimelockMultiSig` (`and_v(v:multi_a, after(n>=500_000_000))`)
///   - SingleSig + `older` / `after` variants (relative + absolute heightlock/timelock)
///   - Multisig + `older` / `after` variants symmetric to the above
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
