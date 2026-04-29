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
//! # Tap-context internal key
//!
//! `script_context = ScriptContext::Tap` requires a Taproot internal
//! key. Per Plan reviewer #1 Concern 2 (Important), the API takes a
//! caller-supplied `Option<DescriptorPublicKey>`:
//!   - `Some(k)`: use `k` as the internal key.
//!   - `None`: rust-miniscript's `compile_tr` derives an unspendable
//!     NUMS internal key for script-path-only spends.

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
/// - `internal_key`: Tap-context only. `None` → rust-miniscript's
///   `compile_tr` synthesises an unspendable NUMS internal key for
///   script-path-only spends. Ignored for `Segwitv0`.
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
    internal_key: Option<DescriptorPublicKey>,
) -> Result<Vec<u8>, Error> {
    let concrete: Concrete<DescriptorPublicKey> =
        policy.parse().map_err(|e: miniscript::Error| {
            Error::PolicyParse(e.to_string())
        })?;

    let descriptor: Descriptor<DescriptorPublicKey> = match script_context {
        ScriptContext::Segwitv0 => {
            let ms = concrete
                .compile::<miniscript::Segwitv0>()
                .map_err(|e| Error::Miniscript(e.to_string()))?;
            Descriptor::new_wsh(ms).map_err(|e| Error::Miniscript(e.to_string()))?
        }
        ScriptContext::Tap => concrete
            .compile_tr(internal_key)
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

    #[test]
    fn tap_pk_with_internal_key_compiles_and_encodes() {
        let internal: DescriptorPublicKey = KEY_1
            .parse()
            .expect("internal key must parse as DescriptorPublicKey");
        let leaf_policy = format!("pk({KEY_2})");
        let bytes = policy_to_bytecode(
            &leaf_policy,
            &EncodeOptions::default(),
            ScriptContext::Tap,
            Some(internal),
        )
        .expect("tap leaf with caller-supplied internal key compiles");
        assert!(bytes.len() > 3);
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
