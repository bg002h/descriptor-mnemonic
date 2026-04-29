//! Concrete-Policy compiler wrapper API.
//!
//! Available only when the `compiler` cargo feature is enabled. Wraps
//! rust-miniscript's policy compiler and projects the resulting
//! descriptor into the BIP 388 wallet-policy template form md-codec
//! encodes.
//!
//! The compiler input format is rust-miniscript's high-level
//! Concrete-Policy syntax (e.g. `or(pk(K1), and(pk(K2), older(144)))`)
//! using fully-qualified [`miniscript::DescriptorPublicKey`] strings —
//! NOT BIP 388 `@N/**` placeholders. The wrapper compiles to optimal
//! miniscript, projects to a wallet policy via
//! `WalletPolicy::from_descriptor`, and encodes the resulting bytecode
//! verbatim through the standard `to_bytecode` path.
//!
//! # Tap-context internal key — `unspendable_key` semantics
//!
//! `script_context = ScriptContext::Tap` is plumbed through to
//! rust-miniscript's `Concrete::compile_tr(unspendable_key)`. This is
//! a *fallback hint*, not a "force this internal key" override:
//!
//! 1. `compile_tr` first calls `extract_key(unspendable_key)`, which
//!    walks the policy tree looking for a key that can serve as the
//!    internal key (typically the highest-probability single-key spend).
//! 2. If extraction succeeds, the extracted key becomes the Taproot
//!    internal key, the policy's contribution to the script tree drops
//!    that key, and `unspendable_key` is **unused**.
//! 3. If extraction fails (no extractable single-key spend), `compile_tr`
//!    falls back to `unspendable_key`. Passing `None` lets the upstream
//!    derive an unspendable NUMS internal key for script-path-only
//!    spends.
//!
//! Concretely: calling
//! `policy_to_bytecode("pk(K2)", &opts, Tap, Some(K1))` produces
//! `tr(K2)` (single-key spend; K1 unused), **not** `tr(K1, pk(K2))`.
//! To force a specific internal key, build the descriptor manually
//! via `miniscript::Descriptor::new_tr` and pass through the standard
//! `WalletPolicy::from_descriptor` path.
//!
//! Per Plan reviewer #1 Concern 2 (Important), the API takes a
//! caller-supplied `Option<DescriptorPublicKey>`. Parameter renamed to
//! `unspendable_key` in v0.7.2 to mirror the upstream naming and make
//! the fallback semantics obvious at the call site.

use miniscript::descriptor::{Descriptor, DescriptorPublicKey, WalletPolicy as InnerWalletPolicy};
use miniscript::policy::Concrete;

use crate::{EncodeOptions, Error, WalletPolicy};

/// Script context for policy compilation.
///
/// Mirrors rust-miniscript's `ScriptContext` constraint at the level of
/// granularity md-codec consumers care about (Segwitv0 vs Tap). Other
/// contexts (`Legacy`, `BareCtx`) are deliberately unrepresented — MD
/// only encodes BIP 388 wallet policies, which restrict the top-level
/// to `wsh()` (Segwitv0) or `tr()` (Tap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptContext {
    /// Segwit v0 (`wsh()` descriptors).
    Segwitv0,
    /// Tapscript (`tr()` descriptors).
    Tap,
}

/// Parse a high-level Concrete Policy expression, compile to optimal
/// miniscript, and encode the result as MD bytecode.
///
/// Available only when the `compiler` cargo feature is enabled.
///
/// # Parameters
///
/// - `policy`: Concrete-Policy expression in rust-miniscript syntax
///   (e.g. `or(pk(<xpub1>), and(pk(<xpub2>), older(144)))`).
/// - `options`: standard `EncodeOptions` (shared_path overrides,
///   fingerprints, force-chunked / force-long-code flags, etc.).
/// - `script_context`: `Segwitv0` for `wsh()`, `Tap` for `tr()`.
/// - `unspendable_key`: Tap-context fallback internal key. Plumbed to
///   `Concrete::compile_tr(unspendable_key)`, which prefers a key
///   extracted from the policy itself and only falls back to this
///   parameter when no extraction is possible. `None` lets upstream
///   derive an unspendable NUMS internal key for script-path-only
///   spends. **This is not a "force this internal key" override** —
///   see the module-level `# Tap-context internal key` section for
///   the precedence rule. Ignored for `Segwitv0`.
///
/// # Errors
///
/// - [`Error::PolicyParse`] on Concrete-Policy parse failure.
/// - [`Error::Miniscript`] on compiler unsatisfiable / sanity error.
/// - [`Error::PolicyScopeViolation`] if `WalletPolicy::from_descriptor`
///   rejects the compiler output (e.g. compiled to a top-level shape
///   MD does not encode).
/// - Any encode-side error from the standard `to_bytecode` path.
pub fn policy_to_bytecode(
    policy: &str,
    options: &EncodeOptions,
    script_context: ScriptContext,
    unspendable_key: Option<DescriptorPublicKey>,
) -> Result<Vec<u8>, Error> {
    let concrete: Concrete<DescriptorPublicKey> = policy
        .parse()
        .map_err(|e: miniscript::Error| Error::PolicyParse(e.to_string()))?;

    let descriptor: Descriptor<DescriptorPublicKey> = match script_context {
        ScriptContext::Segwitv0 => {
            let ms = concrete
                .compile::<miniscript::Segwitv0>()
                .map_err(|e| Error::Miniscript(e.to_string()))?;
            Descriptor::new_wsh(ms).map_err(|e| Error::Miniscript(e.to_string()))?
        }
        ScriptContext::Tap => concrete
            .compile_tr(unspendable_key)
            .map_err(|e| Error::Miniscript(e.to_string()))?,
    };

    // Project the descriptor to BIP 388 wallet-policy template form.
    let inner = InnerWalletPolicy::from_descriptor(&descriptor)
        .map_err(|e| Error::PolicyScopeViolation(e.to_string()))?;
    // WalletPolicy fields are private; round-trip via Display + FromStr.
    let template_str = inner.to_string();
    let wallet_policy: WalletPolicy = template_str.parse()?;
    wallet_policy.to_bytecode(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two known-valid xpubs with origin metadata (BIP 388 requires
    // origin-prefixed xpubs in real wallet policies).
    const KEY_1: &str = "[6738736c/86'/0'/0']xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb/<0;1>/*";
    const KEY_2: &str = "[6738736c/86'/0'/1']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw/<0;1>/*";

    #[test]
    fn segwitv0_pk_compiles_and_encodes() {
        let policy = format!("pk({KEY_1})");
        let bytes = policy_to_bytecode(
            &policy,
            &EncodeOptions::default(),
            ScriptContext::Segwitv0,
            None,
        )
        .expect("simple pk policy compiles in Segwitv0");
        // header + SharedPath + tree
        assert!(bytes.len() > 3, "bytecode must be non-trivial");
        assert_eq!(bytes[0], 0x00, "header byte");
    }

    #[test]
    fn segwitv0_or_pk_pk_compiles_and_encodes() {
        let policy = format!("or(pk({KEY_1}),pk({KEY_2}))");
        let bytes = policy_to_bytecode(
            &policy,
            &EncodeOptions::default(),
            ScriptContext::Segwitv0,
            None,
        )
        .expect("or(pk, pk) policy compiles in Segwitv0");
        assert!(bytes.len() > 5);
    }

    /// `unspendable_key` is a fallback hint per upstream `compile_tr`
    /// semantics — when the policy contains a `pk(K)` shape, the
    /// extracted key K becomes the internal key and `unspendable_key`
    /// is unused. This test exercises that path: compile of `pk(K2)`
    /// with `unspendable_key = Some(K1)` produces a single-key
    /// `tr(K2)` (K1 ignored). Documented as `unspendable_key`
    /// fallback semantics in the module-level rustdoc.
    #[test]
    fn tap_pk_passes_unspendable_key_fallback_unused() {
        let unspendable: DescriptorPublicKey = KEY_1
            .parse()
            .expect("unspendable_key fallback must parse as DescriptorPublicKey");
        let leaf_policy = format!("pk({KEY_2})");
        let bytes = policy_to_bytecode(
            &leaf_policy,
            &EncodeOptions::default(),
            ScriptContext::Tap,
            Some(unspendable),
        )
        .expect("tap leaf with caller-supplied unspendable_key compiles");
        assert!(bytes.len() > 3);
    }

    /// Per Plan reviewer #1 Concern 2: when the caller passes
    /// `unspendable_key = None`, rust-miniscript's `compile_tr` synthesises
    /// an unspendable NUMS internal key. This test exercises that path
    /// end-to-end (compile → project to wallet policy → encode bytecode)
    /// to guard against silent regressions in either the upstream NUMS
    /// derivation or the wallet-policy projection's tolerance for it.
    #[test]
    fn tap_pk_with_nums_unspendable_key_compiles_and_encodes() {
        let leaf_policy = format!("pk({KEY_1})");
        let bytes = policy_to_bytecode(
            &leaf_policy,
            &EncodeOptions::default(),
            ScriptContext::Tap,
            None, // NUMS path
        )
        .expect("tap leaf with NUMS-synthesised internal key must compile");
        assert!(bytes.len() > 3);
        assert_eq!(bytes[0], 0x00, "header byte");
    }

    #[test]
    fn parse_error_surfaces_as_policy_parse() {
        let bad = "not a policy";
        let err = policy_to_bytecode(
            bad,
            &EncodeOptions::default(),
            ScriptContext::Segwitv0,
            None,
        )
        .expect_err("garbage input must fail");
        assert!(
            matches!(err, Error::PolicyParse(_) | Error::Miniscript(_)),
            "expected PolicyParse or Miniscript, got {err:?}"
        );
    }
}
