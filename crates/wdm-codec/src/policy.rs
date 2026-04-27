//! WDM wallet policy newtype wrapping `miniscript::descriptor::WalletPolicy`.
//!
//! # Design decisions (Phase 5)
//!
//! - `key_count()` is derived by counting unique `@N` placeholder indices in
//!   the template string, because the inner type's `key_info` and `template`
//!   fields are private (D-3).
//! - `shared_path()` extracts the origin derivation path from the first key by
//!   materializing the descriptor via dummy keys + `into_descriptor()`, then
//!   reading the origin on the first `DescriptorPublicKey` (D-4 follow-up in 5-B).
//!   Returns `None` for template-only policies (no key_info attached).
//! - `to_canonical_string()` replaces the `/**` BIP 388 shorthand with
//!   `/<0;1>/*` per BIP §"Round-trip canonical form". The inner type's
//!   `Display` uses `/**` (D-5).
//! - `to_bytecode` / `from_bytecode` use Approach B (dummy-key materialization)
//!   because the fork's `WalletPolicy` does not expose the template AST
//!   directly; see PHASE_5_DECISIONS.md D-7 and D-8.

use std::str::FromStr;

use bitcoin::bip32::DerivationPath;
use miniscript::descriptor::{DescriptorPublicKey, WalletPolicy as InnerWalletPolicy};

use crate::Error;
use crate::bytecode::cursor::Cursor;
use crate::bytecode::decode::decode_template;
use crate::bytecode::encode::encode_template;
use crate::bytecode::header::BytecodeHeader;
use crate::bytecode::path::{decode_declaration, encode_declaration};

// ---------------------------------------------------------------------------
// Dummy-key table (Approach B)
// ---------------------------------------------------------------------------
//
// To materialize a `Descriptor<DescriptorPublicKey>` from a template-only
// `WalletPolicy` (which has no real keys attached), we substitute placeholder
// indices with hardcoded dummy xpubs. These xpubs are encoder-internal only
// and are NEVER published or used for actual key derivation. They are selected
// to be syntactically valid BIP 32 extended public keys with distinct
// fingerprints (making them distinct for `DescriptorPublicKey` equality checks)
// and `/<0;1>/*` derivation suffixes (required for the `KeyExpression`
// translator that calls `pk.wildcard()` during `from_descriptor()`).
//
// Dummy key format: `[000000NN/84'/0'/0']<xpub>/<0;1>/*`
//
// The xpub base is the same known-valid value for all entries (BIP 32 test
// vector); uniqueness comes from the fingerprint byte and the `/N'` account
// index in the origin path.
//
// v0.1 supports up to MAX_DUMMY_KEYS placeholder keys. BIP 388 limits
// wallet policies to a practical maximum much lower than 32; 8 entries cover
// common use-cases (single-sig, 2-of-3, 3-of-5, etc.). To support more,
// add entries with distinct fingerprints and origin account indices.

// All 8 entries use proven-valid xpubs taken from the miniscript fork's own
// test vectors. Each xpub+fingerprint combination is distinct so that
// `DescriptorPublicKey`'s derived `PartialEq` treats them as separate keys.
const DUMMY_KEYS: &[&str] = &[
    "[6738736c/44'/0'/0']xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb/<0;1>/*",
    "[6738736c/48'/0'/0'/100']xpub6FC1fXFP1GXQpyRFfSE1vzzySqs3Vg63bzimYLeqtNUYbzA87kMNTcuy9ubr7MmavGRjW2FRYHP4WGKjwutbf1ghgkUW9H7e3ceaPLRcVwa/<0;1>/*",
    "[6738736c/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw/<0;1>/*",
    "[6738736c/49'/0'/1']xpub6Bex1CHWGXNNwGVKHLqNC7kcV348FxkCxpZXyCWp1k27kin8sRPayjZUKDjyQeZzGUdyeAj2emoW5zStFFUAHRgd5w8iVVbLgZ7PmjAKAm9/<0;1>/*",
    "[6738736c/84'/0'/2']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt/<0;1>/*",
    "[6738736c/86'/0'/0']xpub6CryUDWPS28eR2cDyojB8G354izmx294BdjeSvH469Ty3o2E6Tq5VjBJCn8rWBgesvTJnyXNAJ3QpLFGuNwqFXNt3gn612raffLWfdHNkYL/<0;1>/*",
    "[a666a867/44'/0'/0'/100']xpub6Dgsze3ujLi1EiHoCtHFMS9VLS1UheVqxrHGfP7sBJ2DBfChEUHV4MDwmxAXR2ayeytpwm3zJEU3H3pjCR6q6U5sP2p2qzAD71x9z5QShK2/<0;1>/*",
    "[b2b1f0cf/44'/0'/0'/100']xpub6EYajCJHe2CK53RLVXrN14uWoEttZgrRSaRztujsXg7yRhGtHmLBt9ot9Pd5ugfwWEu6eWyJYKSshyvZFKDXiNbBcoK42KRZbxwjRQpm5Js/<0;1>/*",
];

/// Maximum number of keys supported by the dummy table.
const MAX_DUMMY_KEYS: usize = DUMMY_KEYS.len();

/// Parse and return the first `count` dummy `DescriptorPublicKey` values.
///
/// Panics if `count > MAX_DUMMY_KEYS` (8). The dummy keys are
/// encoder-internal only and must never be published.
fn dummy_keys(count: usize) -> Vec<DescriptorPublicKey> {
    assert!(
        count <= MAX_DUMMY_KEYS,
        "requested {count} dummy keys but only {MAX_DUMMY_KEYS} are available"
    );
    DUMMY_KEYS[..count]
        .iter()
        .map(|s| {
            DescriptorPublicKey::from_str(s)
                .unwrap_or_else(|e| panic!("dummy key {s:?} failed to parse: {e}"))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// WalletPolicy newtype
// ---------------------------------------------------------------------------

/// WDM wallet policy: thin newtype around `miniscript::descriptor::WalletPolicy`
/// with WDM-specific canonical-form output and shared-path extraction.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletPolicy {
    inner: InnerWalletPolicy,
}

impl FromStr for WalletPolicy {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        InnerWalletPolicy::from_str(s)
            .map(|inner| WalletPolicy { inner })
            .map_err(|e| crate::Error::PolicyParse(e.to_string()))
    }
}

impl WalletPolicy {
    /// Construct from a parsed inner policy. Used for tests and bytecode
    /// round-trip (Task 5-B).
    pub fn from_inner(inner: InnerWalletPolicy) -> Self {
        Self { inner }
    }

    /// Convert to BIP 388 canonical string form.
    ///
    /// Canonical form requires (per BIP §"Round-trip canonical form"):
    /// - No optional whitespace
    /// - `/**` expanded to `/<0;1>/*`
    /// - Hardened components use `'` (not `h` or `H`)
    /// - Key information vector ordered by ascending placeholder index
    ///
    /// The inner `Display` already handles whitespace, hardened markers, and
    /// ordering; this method additionally expands the `/**` shorthand.
    pub fn to_canonical_string(&self) -> String {
        self.inner.to_string().replace("/**", "/<0;1>/*")
    }

    /// Number of unique placeholder keys in the template.
    ///
    /// Derived by scanning the template string for `@N` tokens and returning
    /// `max_index + 1`. For a well-formed BIP 388 template the indices are
    /// sequential starting at 0, so this equals the distinct key count.
    ///
    /// Note: `inner.to_string()` writes only the template portion (no `@`
    /// outside `@N` placeholder tokens), so this scan is unambiguous.
    pub fn key_count(&self) -> usize {
        let s = self.inner.to_string();
        let mut max_index: Option<u32> = None;
        let mut chars = s.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '@' {
                // Collect the run of ASCII digits immediately following '@'.
                let mut digits = String::new();
                while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                    digits.push(chars.next().unwrap());
                }
                if let Ok(idx) = digits.parse::<u32>() {
                    max_index = Some(match max_index {
                        Some(prev) => prev.max(idx),
                        None => idx,
                    });
                }
            }
        }
        max_index.map_or(0, |m| m as usize + 1)
    }

    /// The shared derivation path used by all `@i` placeholders, if any.
    ///
    /// For policies created from full descriptor strings (with origin info),
    /// this returns the origin path of the first key. Returns `None` if the
    /// policy was created from a template string only (no key info attached).
    ///
    /// BIP 388 requires all keys to share the same origin derivation path;
    /// this method returns the first key's origin as the canonical representative.
    pub fn shared_path(&self) -> Option<DerivationPath> {
        // To extract the origin path we materialize the descriptor. A
        // template-only policy (no key_info) cannot be materialized — clone
        // and try; if it fails, return None.
        let descriptor = self.inner.clone().into_descriptor().ok()?;
        // Extract the origin (BIP 32 fingerprint + derivation path) from the
        // first public key. BIP 388 requires all keys to share the same origin
        // path, so the first one is representative.
        let first_key = descriptor.iter_pk().next()?;
        match &first_key {
            DescriptorPublicKey::XPub(xpub) => xpub.origin.as_ref().map(|(_, path)| path.clone()),
            DescriptorPublicKey::MultiXPub(multi) => {
                multi.origin.as_ref().map(|(_, path)| path.clone())
            }
            DescriptorPublicKey::Single(_) => None,
        }
    }

    /// Read-only access to the wrapped miniscript WalletPolicy.
    #[doc(hidden)]
    pub fn inner(&self) -> &InnerWalletPolicy {
        &self.inner
    }

    // -----------------------------------------------------------------------
    // Bytecode encoding / decoding (Task 5-B)
    // -----------------------------------------------------------------------

    /// Encode this policy as canonical WDM bytecode.
    ///
    /// Format: `[BytecodeHeader] [PathDeclaration] [TreeBytes]`
    ///
    /// Uses **Approach B** (dummy-key materialization): clones the inner
    /// policy, sets dummy keys via `set_key_info`, materializes a full
    /// `Descriptor<DescriptorPublicKey>`, encodes the tree, then composes the
    /// three sections. See PHASE_5_DECISIONS.md D-7.
    pub fn to_bytecode(&self) -> Result<Vec<u8>, Error> {
        let count = self.key_count();
        if count > MAX_DUMMY_KEYS {
            return Err(Error::PolicyScopeViolation(format!(
                "policy has {count} placeholder keys; v0.1 supports at most {MAX_DUMMY_KEYS}"
            )));
        }

        // --- Step 1: materialize descriptor with dummy keys ---
        let dummies = dummy_keys(count);
        let mut inner_clone = self.inner.clone();
        inner_clone
            .set_key_info(&dummies)
            .map_err(|e| Error::PolicyScopeViolation(e.to_string()))?;
        let descriptor = inner_clone
            .into_descriptor()
            .map_err(|e| Error::PolicyScopeViolation(e.to_string()))?;

        // --- Step 2: build placeholder map (dummy_key[i] → i) ---
        let mut placeholder_map = std::collections::HashMap::new();
        for (i, key) in dummies.iter().enumerate() {
            placeholder_map.insert(key.clone(), i as u8);
        }

        // --- Step 3: encode the descriptor tree ---
        let tree_bytes = encode_template(&descriptor, &placeholder_map)?;

        // --- Step 4: determine the shared path ---
        // Use the actual policy's shared_path if available (real keys attached),
        // else fall back to a default. For template-only policies we use the
        // dummy origin path (m/84'/0'/0').
        let shared_path = self.shared_path().unwrap_or_else(|| {
            // Default: BIP 84 mainnet — the dummy keys' origin path.
            DerivationPath::from_str("m/84'/0'/0'").expect("hardcoded BIP 84 path is always valid")
        });

        // --- Step 5: compose [header][path declaration][tree bytes] ---
        let header = BytecodeHeader::new_v0(false);
        let mut out = Vec::new();
        out.push(header.as_byte());
        out.extend_from_slice(&encode_declaration(&shared_path));
        out.extend_from_slice(&tree_bytes);
        Ok(out)
    }

    /// Decode canonical WDM bytecode into a `WalletPolicy`.
    ///
    /// The resulting policy is constructed from the decoded descriptor using
    /// dummy keys as placeholders; key_info contains the dummy keys, not real
    /// keys. Real key info must be supplied separately (e.g., during restore
    /// flow via `set_key_info`).
    ///
    /// # Errors
    ///
    /// - `Error::InvalidBytecode { .. }` — truncated or malformed header/path.
    /// - `Error::UnsupportedVersion(v)` — bytecode uses an unsupported version.
    /// - `Error::PolicyScopeViolation(..)` — fingerprints flag is set (deferred
    ///   to v0.2), or the template violates v0.1 scope.
    pub fn from_bytecode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.is_empty() {
            return Err(Error::InvalidBytecode {
                offset: 0,
                kind: crate::error::BytecodeErrorKind::UnexpectedEnd,
            });
        }

        // --- Step 1: parse and validate the header byte ---
        let header = BytecodeHeader::from_byte(bytes[0])?;
        if header.fingerprints() {
            return Err(Error::PolicyScopeViolation(
                "v0.1 does not support the fingerprints block; use the no-fingerprints form (header byte 0x00)".to_string(),
            ));
        }

        // --- Step 2: parse the path declaration ---
        let mut cursor = Cursor::new(&bytes[1..]);
        let _shared_path = decode_declaration(&mut cursor)?;
        let path_consumed = cursor.offset();

        // --- Step 3: decode the template tree ---
        // We use a two-pass approach: first count distinct placeholder indices
        // in the remaining bytes to determine key_count, then decode with
        // exactly that many dummy keys.
        let tree_start = 1 + path_consumed;
        let tree_bytes = &bytes[tree_start..];
        let key_count = count_placeholder_indices(tree_bytes)?;

        let dummies = dummy_keys(key_count);
        let descriptor = decode_template(tree_bytes, &dummies)?;

        // --- Step 4: construct WalletPolicy from the descriptor ---
        let inner = InnerWalletPolicy::from_descriptor(&descriptor)
            .map_err(|e| Error::PolicyScopeViolation(e.to_string()))?;
        Ok(WalletPolicy { inner })
    }
}

// ---------------------------------------------------------------------------
// Placeholder index counter (for from_bytecode two-pass)
// ---------------------------------------------------------------------------

/// Count the number of distinct placeholder indices in a raw tree byte stream.
///
/// Scans the stream for `Tag::Placeholder` (0x32) bytes, reads the following
/// index byte, and returns `max_index + 1`. This is used by `from_bytecode` to
/// determine how many dummy keys to supply to `decode_template`.
///
/// Returns `Err(PolicyScopeViolation)` if the placeholder index would exceed
/// `MAX_DUMMY_KEYS`. A malformed stream (truncated placeholder) returns the
/// same error because we're doing a best-effort scan, not full decode.
fn count_placeholder_indices(tree_bytes: &[u8]) -> Result<usize, Error> {
    use crate::bytecode::Tag;
    let mut max_index: Option<u8> = None;
    let mut i = 0;
    while i < tree_bytes.len() {
        let b = tree_bytes[i];
        i += 1;
        if let Some(Tag::Placeholder) = Tag::from_byte(b) {
            if i >= tree_bytes.len() {
                // Truncated — the full decode will produce a proper error;
                // here we just bail with 0 which is safe (decode will error).
                break;
            }
            let idx = tree_bytes[i];
            i += 1;
            max_index = Some(match max_index {
                Some(prev) => prev.max(idx),
                None => idx,
            });
        }
    }
    let count = max_index.map_or(0, |m| m as usize + 1);
    if count > MAX_DUMMY_KEYS {
        return Err(Error::PolicyScopeViolation(format!(
            "decoded policy has {count} placeholder indices; v0.1 supports at most {MAX_DUMMY_KEYS}"
        )));
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet_id::compute_wallet_id;

    // -----------------------------------------------------------------------
    // Parsing
    // -----------------------------------------------------------------------

    #[test]
    fn from_str_parses_template() {
        let result = "wsh(pk(@0/**))".parse::<WalletPolicy>();
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    #[test]
    fn from_str_rejects_invalid() {
        let result = "not a policy".parse::<WalletPolicy>();
        assert!(
            matches!(result, Err(crate::Error::PolicyParse(_))),
            "expected PolicyParse, got {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Canonical string
    // -----------------------------------------------------------------------

    #[test]
    fn to_canonical_string_round_trip() {
        let input = "wsh(pk(@0/**))";
        let p1: WalletPolicy = input.parse().expect("should parse");
        let canonical = p1.to_canonical_string();
        let p2: WalletPolicy = canonical.parse().expect("canonical form should re-parse");
        assert_eq!(p1, p2, "re-parsed policy should equal original");
    }

    #[test]
    fn to_canonical_string_normalizes_wildcard_shorthand() {
        // BIP §"Round-trip canonical form": /** must expand to /<0;1>/*.
        let with_shorthand: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let already_canonical: WalletPolicy = "wsh(pk(@0/<0;1>/*))".parse().unwrap();
        let cs = with_shorthand.to_canonical_string();
        let ca = already_canonical.to_canonical_string();
        assert_eq!(
            cs, ca,
            "/** and /<0;1>/* inputs must canonicalize identically"
        );
        assert!(
            !cs.contains("/**"),
            "canonical string must not contain /** shorthand; got: {cs}"
        );
        assert!(
            cs.contains("/<0;1>/*"),
            "canonical string must contain /<0;1>/*; got: {cs}"
        );
    }

    // -----------------------------------------------------------------------
    // key_count
    // -----------------------------------------------------------------------

    #[test]
    fn key_count_for_single_placeholder() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        assert_eq!(p.key_count(), 1);
    }

    #[test]
    fn key_count_for_multisig() {
        let p: WalletPolicy = "wsh(sortedmulti(2,@0/**,@1/**,@2/**))".parse().unwrap();
        assert_eq!(p.key_count(), 3);
    }

    // -----------------------------------------------------------------------
    // shared_path
    // -----------------------------------------------------------------------

    #[test]
    fn shared_path_returns_none_for_template_only_policy() {
        // A template-only policy has no key_info attached and no origin paths.
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        assert!(
            p.shared_path().is_none(),
            "template-only policy must return None for shared_path"
        );
    }

    #[test]
    fn shared_path_returns_some_for_policy_with_keys() {
        // A policy parsed from a full descriptor string (with origin info)
        // should return the origin path of the first key.
        // BIP 84 mainnet: m/84'/0'/0' -> indicator 0x03
        let desc_str = "wsh(pk([6738736c/84'/0'/0']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt/<0;1>/*))";
        let p: WalletPolicy = desc_str.parse().expect("should parse full descriptor");
        let path = p.shared_path();
        assert!(
            path.is_some(),
            "policy with key_info must return Some(path)"
        );
        let expected = DerivationPath::from_str("m/84'/0'/0'").unwrap();
        assert_eq!(
            path.unwrap(),
            expected,
            "shared_path must return the origin derivation path"
        );
    }

    // -----------------------------------------------------------------------
    // inner accessor
    // -----------------------------------------------------------------------

    #[test]
    fn inner_returns_underlying_type() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let _inner: &InnerWalletPolicy = p.inner();
        // Smoke test: inner() exists and returns the right reference type.
        // Round-trip: inner's Display should produce the template string.
        let s = p.inner().to_string();
        assert!(
            s.contains("@0"),
            "inner Display should contain placeholder; got: {s}"
        );
    }

    // -----------------------------------------------------------------------
    // to_bytecode / from_bytecode round-trips
    // -----------------------------------------------------------------------

    #[test]
    fn to_bytecode_round_trip_single_key() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let bytes = p.to_bytecode().expect("to_bytecode should succeed");
        let p2 = WalletPolicy::from_bytecode(&bytes).expect("from_bytecode should succeed");
        // Check structural equality: both policies should have the same
        // template (key_count, canonical string).
        assert_eq!(
            p.key_count(),
            p2.key_count(),
            "round-trip must preserve key_count"
        );
        // Both should represent wsh(pk(@0/...)) — compare via template string
        // (strip the dummy keys' specific derivation paths from the representation).
        let s1 = p.inner().to_string();
        let s2 = p2.inner().to_string();
        assert!(
            s1.contains("@0") && s2.contains("@0"),
            "both must contain placeholder @0; got {s1:?} and {s2:?}"
        );
    }

    #[test]
    fn to_bytecode_round_trip_multisig() {
        let p: WalletPolicy = "wsh(sortedmulti(2,@0/**,@1/**,@2/**))".parse().unwrap();
        let bytes = p.to_bytecode().expect("to_bytecode should succeed");
        let p2 = WalletPolicy::from_bytecode(&bytes).expect("from_bytecode should succeed");
        assert_eq!(
            p.key_count(),
            p2.key_count(),
            "round-trip must preserve key_count"
        );
        assert_eq!(
            p2.key_count(),
            3,
            "multisig must have 3 keys after round-trip"
        );
    }

    #[test]
    fn to_bytecode_starts_with_header() {
        // First byte must be 0x00 (version 0, no fingerprints).
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let bytes = p.to_bytecode().unwrap();
        assert_eq!(
            bytes[0], 0x00,
            "first byte must be 0x00 (v0, no fingerprints)"
        );
    }

    #[test]
    fn to_bytecode_then_path_declaration() {
        // For a template-only policy the encoder uses the default path m/84'/0'/0'.
        // Path declaration: byte[1] = Tag::SharedPath (0x33), byte[2] = indicator 0x03.
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let bytes = p.to_bytecode().unwrap();
        assert_eq!(bytes[1], 0x33, "byte[1] must be Tag::SharedPath (0x33)");
        assert_eq!(
            bytes[2], 0x03,
            "byte[2] must be BIP 84 mainnet indicator (0x03)"
        );
    }

    #[test]
    fn to_bytecode_then_path_declaration_bip84_with_keys() {
        // Build a policy with a real BIP 84 mainnet key. The shared_path()
        // returns m/84'/0'/0' and the path declaration uses indicator 0x03.
        let desc_str = "wsh(pk([6738736c/84'/0'/0']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt/<0;1>/*))";
        let p: WalletPolicy = desc_str.parse().expect("should parse");
        let bytes = p.to_bytecode().unwrap();
        assert_eq!(bytes[0], 0x00, "header must be 0x00");
        assert_eq!(bytes[1], 0x33, "byte[1] must be Tag::SharedPath");
        assert_eq!(bytes[2], 0x03, "byte[2] must be BIP 84 indicator 0x03");
    }

    #[test]
    fn from_bytecode_rejects_truncated_header() {
        // Empty input must return Err.
        let result = WalletPolicy::from_bytecode(&[]);
        assert!(
            result.is_err(),
            "from_bytecode(&[]) must return Err; got {result:?}"
        );
    }

    #[test]
    fn from_bytecode_rejects_unsupported_version() {
        // 0x10 = version nibble 1, which is unsupported.
        let result = WalletPolicy::from_bytecode(&[0x10, 0x33, 0x03]);
        assert!(
            matches!(result, Err(Error::UnsupportedVersion(1))),
            "expected UnsupportedVersion(1), got {result:?}"
        );
    }

    #[test]
    fn from_bytecode_rejects_fingerprints_flag() {
        // 0x04 = version 0, fingerprints flag set — deferred to v0.2.
        let result = WalletPolicy::from_bytecode(&[0x04, 0x33, 0x03]);
        assert!(
            matches!(result, Err(Error::PolicyScopeViolation(_))),
            "expected PolicyScopeViolation for fingerprints flag, got {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Free-function wrappers
    // -----------------------------------------------------------------------

    #[test]
    fn encode_bytecode_free_fn_matches_method() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let via_method = p.to_bytecode().unwrap();
        let via_fn = crate::encode_bytecode(&p).unwrap();
        assert_eq!(
            via_method, via_fn,
            "encode_bytecode free fn must match to_bytecode method"
        );
    }

    #[test]
    fn decode_bytecode_free_fn_matches_method() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let bytes = p.to_bytecode().unwrap();
        let via_method = WalletPolicy::from_bytecode(&bytes).unwrap();
        let via_fn = crate::decode_bytecode(&bytes).unwrap();
        assert_eq!(
            via_method.key_count(),
            via_fn.key_count(),
            "decode_bytecode free fn must match from_bytecode method"
        );
    }

    // -----------------------------------------------------------------------
    // compute_wallet_id_for_policy
    // -----------------------------------------------------------------------

    #[test]
    fn compute_wallet_id_for_policy_matches_compute_wallet_id_of_to_bytecode() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let bytecode = p.to_bytecode().unwrap();
        let direct = compute_wallet_id(&bytecode);
        let via_policy = crate::wallet_id::compute_wallet_id_for_policy(&p).unwrap();
        assert_eq!(
            direct, via_policy,
            "compute_wallet_id_for_policy must equal compute_wallet_id(to_bytecode())"
        );
    }
}
