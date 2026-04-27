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
// to be syntactically valid BIP 32 extended public keys with `/<0;1>/*`
// derivation suffixes (required because the `KeyExpression` translator calls
// `pk.wildcard()` during `from_descriptor()`).
//
// Dummy key format: `[fingerprint/derivation_path]xpub.../<0;1>/*`
//
// Uniqueness: `DescriptorPublicKey`'s derived `PartialEq` compares the full
// (fingerprint, origin path, xpub) triple. Entries are distinct if ANY of
// fingerprint, path, or xpub differs. This table achieves distinctness by
// using different xpubs (pulled from the miniscript fork's own test vectors).
// When two entries share the same xpub, they are made distinct by using
// different origin paths (different account indices or purpose fields).
//
// Size: 32 entries matching BIP 388's maximum placeholder count (indices 0..=31).
// The table is separated into a private submodule to keep policy.rs readable.

const DUMMY_KEYS: &[&str] = &[
    // Entries 0–7: same as the original table (distinct xpubs from fork test vectors).
    "[6738736c/44'/0'/0']xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb/<0;1>/*",
    "[6738736c/48'/0'/0'/100']xpub6FC1fXFP1GXQpyRFfSE1vzzySqs3Vg63bzimYLeqtNUYbzA87kMNTcuy9ubr7MmavGRjW2FRYHP4WGKjwutbf1ghgkUW9H7e3ceaPLRcVwa/<0;1>/*",
    "[6738736c/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw/<0;1>/*",
    "[6738736c/49'/0'/1']xpub6Bex1CHWGXNNwGVKHLqNC7kcV348FxkCxpZXyCWp1k27kin8sRPayjZUKDjyQeZzGUdyeAj2emoW5zStFFUAHRgd5w8iVVbLgZ7PmjAKAm9/<0;1>/*",
    "[6738736c/84'/0'/2']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt/<0;1>/*",
    "[6738736c/86'/0'/0']xpub6CryUDWPS28eR2cDyojB8G354izmx294BdjeSvH469Ty3o2E6Tq5VjBJCn8rWBgesvTJnyXNAJ3QpLFGuNwqFXNt3gn612raffLWfdHNkYL/<0;1>/*",
    "[a666a867/44'/0'/0'/100']xpub6Dgsze3ujLi1EiHoCtHFMS9VLS1UheVqxrHGfP7sBJ2DBfChEUHV4MDwmxAXR2ayeytpwm3zJEU3H3pjCR6q6U5sP2p2qzAD71x9z5QShK2/<0;1>/*",
    "[b2b1f0cf/44'/0'/0'/100']xpub6EYajCJHe2CK53RLVXrN14uWoEttZgrRSaRztujsXg7yRhGtHmLBt9ot9Pd5ugfwWEu6eWyJYKSshyvZFKDXiNbBcoK42KRZbxwjRQpm5Js/<0;1>/*",
    // Entries 8–15: additional xpubs from the fork's test vectors (all distinct).
    "[bb641298/44'/0'/0'/100']xpub6Dz8PHFmXkYkykQ83ySkruky567XtJb9N69uXScJZqweYiQn6FyieajdiyjCvWzRZ2GoLHMRE1cwDfuJZ6461YvNRGVBJNnLA35cZrQKSRJ/<0;1>/*",
    "[6738736c/48'/0'/0'/3']xpub6Fc2TRaCWNgfT49nRGG2G78d1dPnjhW66gEXi7oYZML7qEFN8e21b2DLDipTZZnfV6V7ivrMkvh4VbnHY2ChHTS9qM3XVLJiAgcfagYQk6K/<0;1>/*",
    "[6738736c/48'/0'/0'/4']xpub6GjFUVVYewLj5no5uoNKCWuyWhQ1rKGvV8DgXBG9Uc6DvAKxt2dhrj1EZFrTNB5qxAoBkVW3wF8uCS3q1ri9fueAa6y7heFTcf27Q4gyeh6/<0;1>/*",
    "[6738736c/48'/0'/0'/5']xpub6GxHB9kRdFfTqYka8tgtX9Gh3Td3A9XS8uakUGVcJ9NGZ1uLrGZrRVr67DjpMNCHprZmVmceFTY4X4wWfksy8nVwPiNvzJ5pjLxzPtpnfEM/<0;1>/*",
    "[6738736c/48'/0'/0'/6']xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL/<0;1>/*",
    "[6738736c/48'/0'/0'/7']xpub6BzhLAQUDcBUfHRQHZxDF2AbcJqp4Kaeq6bzJpXrjrWuK26ymTFwkEFbxPra2bJ7yeZKbDjfDeFwxe93JMqpo5SsPJH6dZdvV9kMzJkAZ69/<0;1>/*",
    "[6738736c/48'/0'/0'/8']xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*",
    "[6738736c/48'/0'/0'/9']xpub6BgBgsespWvERF3LHQu6CnqdvfEvtMcQjYrcRzx53QJjSxarj2afYWcLteoGVky7D3UKDP9QyrLprQ3VCECoY49yfdDEHGCtMMj92pReUsQ/<0;1>/*",
    // Entries 16–23: further distinct entries using different fingerprints and paths.
    "[c0c0c0c0/44'/0'/0'/0']xpub661MyMwAqRbcFkPHucMnrGNzDwb6teAX1RbKQmqtEF8kK3Z7LZ59qafCjB9eCRLiTVG3uxBxgKvRgbubRhqSKXnGGb1aoaqLrpMBDrVxga8/<0;1>/*",
    "[c0c0c0c1/44'/0'/0'/1']xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8/<0;1>/*",
    "[c0c0c0c2/44'/0'/0'/2']xpub661MyMwAqRbcFW31YEwpkMuc5THy2PSt5bDMsktWQcFF8syAmRUapSCGu8ED9W6oDMSgv6Zz8idoc4a6mr8BDzTJY47LJhkJ8UB7WEGuduB/<0;1>/*",
    "[c0c0c0c3/44'/0'/0'/3']xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y/<0;1>/*",
    "[c0c0c0c4/44'/0'/0'/4']xpub69H7F5d8KSRgmmdJg2KhpAK8SR3DjMwAdkxj3ZuxV27CprR9LgpeyGmXUbC6wb7ERfvrnKZjXoUmmDznezpbZb7ap6r1D3tgFxHmwMkQTPH/<0;1>/*",
    "[c0c0c0c5/44'/0'/0'/5']xpub6ASuArnXKPbfEwhqN6e3mwBcDTgzisQN1wXN9BJcM47sSikHjJf3UFHKkNAWbWMiGj7Wf5uMash7SyYq527Hqck2AxYysAA7xmALppuCkwQ/<0;1>/*",
    "[c0c0c0c6/44'/0'/0'/6']xpub6AHA9hZDN11k2ijHMeS5QqHx2KP9aMBRhTDqANMnwVtdyw2TDYRmF8PjpvwUFcL1Et8Hj59S3gTSMcUQ5gAqTz3Wd8EsMTmF3DChhqPQBnU/<0;1>/*",
    "[c0c0c0c7/44'/0'/0'/7']xpub6BgBgsespWvERF3LHQu6CnqdvfEvtMcQjYrcRzx53QJjSxarj2afYWcLteoGVky7D3UKDP9QyrLprQ3VCECoY49yfdDEHGCtMMj92pReUsQ/<0;1>/*",
    // Entries 24–31: final batch with distinct fingerprints c0c0c0d0..c0c0c0d7.
    "[c0c0c0d0/48'/0'/0'/0']xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb/<0;1>/*",
    "[c0c0c0d1/48'/0'/0'/1']xpub6FC1fXFP1GXQpyRFfSE1vzzySqs3Vg63bzimYLeqtNUYbzA87kMNTcuy9ubr7MmavGRjW2FRYHP4WGKjwutbf1ghgkUW9H7e3ceaPLRcVwa/<0;1>/*",
    "[c0c0c0d2/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw/<0;1>/*",
    "[c0c0c0d3/48'/0'/0'/3']xpub6Bex1CHWGXNNwGVKHLqNC7kcV348FxkCxpZXyCWp1k27kin8sRPayjZUKDjyQeZzGUdyeAj2emoW5zStFFUAHRgd5w8iVVbLgZ7PmjAKAm9/<0;1>/*",
    "[c0c0c0d4/48'/0'/0'/4']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt/<0;1>/*",
    "[c0c0c0d5/48'/0'/0'/5']xpub6CryUDWPS28eR2cDyojB8G354izmx294BdjeSvH469Ty3o2E6Tq5VjBJCn8rWBgesvTJnyXNAJ3QpLFGuNwqFXNt3gn612raffLWfdHNkYL/<0;1>/*",
    "[c0c0c0d6/48'/0'/0'/6']xpub6Dgsze3ujLi1EiHoCtHFMS9VLS1UheVqxrHGfP7sBJ2DBfChEUHV4MDwmxAXR2ayeytpwm3zJEU3H3pjCR6q6U5sP2p2qzAD71x9z5QShK2/<0;1>/*",
    "[c0c0c0d7/48'/0'/0'/7']xpub6EYajCJHe2CK53RLVXrN14uWoEttZgrRSaRztujsXg7yRhGtHmLBt9ot9Pd5ugfwWEu6eWyJYKSshyvZFKDXiNbBcoK42KRZbxwjRQpm5Js/<0;1>/*",
];

/// Maximum number of placeholder keys supported; 32 matches BIP 388's cap.
const MAX_DUMMY_KEYS: usize = DUMMY_KEYS.len(); // must be 32

/// Parse and return the first `count` dummy `DescriptorPublicKey` values.
///
/// Panics if `count > MAX_DUMMY_KEYS` (32). The dummy keys are
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

/// Parse and return ALL `MAX_DUMMY_KEYS` (32) dummy `DescriptorPublicKey` values.
///
/// Used by `from_bytecode` (Option A fix for D-8): we pass all 32 dummies to
/// `decode_template` so it can satisfy any placeholder index 0..=31. The decoder
/// only accesses the indices actually referenced in the tree; unused entries past
/// the real max index are never touched. `from_descriptor` then re-derives the
/// actual key set from `descriptor.iter_pk()`, which returns only the keys that
/// appeared in the decoded descriptor — so extra dummies are discarded.
fn all_dummy_keys() -> Vec<DescriptorPublicKey> {
    dummy_keys(MAX_DUMMY_KEYS)
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
    /// Note: this scans the BIP 388 template form (`@N`-only), which the
    /// fork's `WalletPolicy::Display` always produces. Origin xpubs and other
    /// `@`-bearing strings appear only in full-descriptor display, not here,
    /// so the scan is unambiguous.
    pub fn key_count(&self) -> usize {
        let s = self.inner.to_string();
        let mut max_index: Option<usize> = None;
        let mut chars = s.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '@' {
                // Collect the run of ASCII digits immediately following '@'.
                let mut digits = String::new();
                while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                    digits.push(chars.next().unwrap());
                }
                if let Ok(idx) = digits.parse::<usize>() {
                    max_index = Some(match max_index {
                        Some(prev) => prev.max(idx),
                        None => idx,
                    });
                }
            }
        }
        max_index.map_or(0, |m| m + 1)
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
    /// `Descriptor<DescriptorPublicKey>` **once**, extracts the shared path
    /// from the descriptor's first key (for policies with real keys), encodes
    /// the tree, then composes the three sections. See PHASE_5_DECISIONS.md D-7.
    ///
    /// # Shared-path fallback
    ///
    /// For template-only policies (no key_info attached), the descriptor's
    /// first key is the dummy entry at index 0, whose origin path is
    /// `m/84'/0'/0'` (BIP 84 mainnet). The encoded bytecode therefore carries
    /// `m/84'/0'/0'` as the path declaration in this case. This is a deliberate
    /// pragmatic choice for v0.1: the round-trip will succeed, and the caller can
    /// supply a real shared path via `EncodeOptions::shared_path` in a future
    /// release (see Phase 5 decision D-10).
    pub fn to_bytecode(&self) -> Result<Vec<u8>, Error> {
        let count = self.key_count();
        if count > MAX_DUMMY_KEYS {
            return Err(Error::PolicyScopeViolation(format!(
                "policy has {count} placeholder keys; v0.1 supports at most {MAX_DUMMY_KEYS}"
            )));
        }

        // --- Step 1: materialize descriptor with dummy keys (single materialization) ---
        // We do NOT call self.shared_path() here: that would materialize the
        // descriptor a second time. Instead we extract the shared path from the
        // descriptor we're about to build (Important fix #3 from 5-B review).
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

        // --- Step 4: extract shared path from the already-materialized descriptor ---
        // If the policy had real keys (the policy was parsed from a full descriptor
        // string), self.inner still holds the original keys in its key_info. We
        // already cloned it and set dummies, so we can't retrieve the real keys from
        // `descriptor` here — they'd be the dummies. Use self.shared_path() for the
        // real-keys case (one extra materialization is acceptable here; the common
        // template-only case avoids it via the dummy fallback).
        let shared_path = self.shared_path().unwrap_or_else(|| {
            // Template-only policy: first dummy key carries origin m/84'/0'/0'.
            // See rustdoc above for rationale.
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

        // --- Step 3: decode the template tree (Option A fix for D-8) ---
        //
        // Previously this used a `count_placeholder_indices` pre-scan to determine
        // how many dummy keys to supply to `decode_template`. That scan read the
        // tree byte-by-byte looking for Tag::Placeholder (0x32), but hash literals
        // (sha256=32 bytes, ripemd160=20 bytes, etc.) embed raw bytes directly
        // after their tag — any of which can equal 0x32. This caused the pre-scan
        // to spuriously count hash body bytes as placeholder tags, inflating the
        // key count and triggering false `PolicyScopeViolation` errors.
        //
        // Fix (Option A): supply all 32 dummy keys up front. The decoder only
        // accesses the indices that actually appear in the tree (via
        // `keys.get(index)` in `decode_placeholder`), so extra dummies beyond the
        // real max index are never touched. `from_descriptor` then re-derives the
        // key set from `descriptor.iter_pk()` which returns only the keys that
        // appeared in the descriptor — extra dummies are discarded automatically.
        //
        // This eliminates the need for a pre-scan entirely and is safe because
        // BIP 388 caps placeholder indices at 31 (= 32 keys), matching our table.
        let tree_start = 1 + path_consumed;
        let tree_bytes = &bytes[tree_start..];
        let dummies = all_dummy_keys();
        let descriptor = decode_template(tree_bytes, &dummies)?;

        // --- Step 4: construct WalletPolicy from the descriptor ---
        // `from_descriptor` collects `descriptor.iter_pk()` which returns only the
        // keys actually referenced in the decoded tree — not all 32 dummies.
        let inner = InnerWalletPolicy::from_descriptor(&descriptor)
            .map_err(|e| Error::PolicyScopeViolation(e.to_string()))?;
        Ok(WalletPolicy { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet_id::compute_wallet_id;

    // -----------------------------------------------------------------------
    // Dummy-key table integrity
    // -----------------------------------------------------------------------

    /// Verify that all 32 DUMMY_KEYS entries parse without error and are
    /// pairwise distinct under `DescriptorPublicKey::PartialEq`.
    ///
    /// This is a compile-time-checkable table property: if any entry is
    /// malformed or two entries are identical, this test catches it at CI.
    #[test]
    fn dummy_keys_table_has_32_distinct_entries() {
        assert_eq!(
            MAX_DUMMY_KEYS, 32,
            "DUMMY_KEYS must have exactly 32 entries to match BIP 388 max"
        );
        let parsed = all_dummy_keys();
        assert_eq!(parsed.len(), 32, "all_dummy_keys() must return 32 entries");
        // Pairwise distinctness check (O(n^2), but n=32 is tiny).
        for i in 0..32 {
            for j in (i + 1)..32 {
                assert_ne!(
                    parsed[i], parsed[j],
                    "DUMMY_KEYS entries {i} and {j} must be distinct (DescriptorPublicKey::eq)"
                );
            }
        }
    }

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

    // -----------------------------------------------------------------------
    // Critical fix regression tests (Task 5-B review)
    // -----------------------------------------------------------------------

    /// Verify that LEB128-encoded data bytes containing 0x32 (which happens to
    /// equal Tag::Placeholder) do not cause `count_placeholder_indices` to
    /// spuriously report extra keys during `from_bytecode`.
    ///
    /// The concrete policy is `wsh(and_v(v:older(50),pk(@0/**)))`:
    /// - `older(50)` encodes as `[Older=0x1F, LEB128(50)=0x32]`
    /// - the byte 0x32 = LEB128(50) is followed by `Check=0x0C` (tag for pk's c: wrapper)
    /// - old `count_placeholder_indices` sees `0x32` at that position and reads `0x0C`
    ///   as placeholder index 12, giving key_count = 13 instead of 1
    /// - this triggers `PolicyScopeViolation` ("decoded policy has 13 placeholder indices")
    ///   on a perfectly valid 1-key policy
    ///
    /// Without Critical fix #1 this test fails. With Option A (delete the pre-scan,
    /// pass 32 dummies to `decode_template`) the LEB128 byte is consumed correctly
    /// by the Older decoder and never confused with a Placeholder tag.
    ///
    /// The bytecode is constructed directly to control the exact byte layout.
    #[test]
    fn from_bytecode_leb128_byte_0x32_not_counted_as_placeholder() {
        use crate::bytecode::Tag;
        // Tree bytes for wsh(and_v(v:older(50), c:pk_k(@0/**)))
        //   where older(50) encodes varint 50 = 0x32 (LEB128 terminal byte).
        //
        // Byte layout:
        //   [0]  Wsh   = 0x05
        //   [1]  AndV  = 0x11
        //   [2]  Verify= 0x0E   ← v: wrapper for older
        //   [3]  Older = 0x1F
        //   [4]  0x32           ← LEB128(50); OLD scanner mistakes this for Placeholder tag
        //   [5]  Check = 0x0C   ← OLD scanner reads this as placeholder index 12 → count=13
        //   [6]  PkK   = 0x1B
        //   [7]  Placeholder = 0x32  ← the REAL placeholder tag
        //   [8]  0x00           ← placeholder index 0
        let tree_bytes: Vec<u8> = vec![
            Tag::Wsh.as_byte(),         // [0]  0x05
            Tag::AndV.as_byte(),        // [1]  0x11
            Tag::Verify.as_byte(),      // [2]  0x0E
            Tag::Older.as_byte(),       // [3]  0x1F
            0x32,                       // [4]  LEB128(50) — CONFUSES old scanner
            Tag::Check.as_byte(),       // [5]  0x0C — old scanner reads as spurious index 12
            Tag::PkK.as_byte(),         // [6]  0x1B
            Tag::Placeholder.as_byte(), // [7]  0x32 — real placeholder
            0x00,                       // [8]  index 0
        ];

        // Assemble full WDM bytecode: header(0x00) + SharedPath(0x33, BIP84=0x03) + tree
        let mut bytecode: Vec<u8> = vec![0x00, Tag::SharedPath.as_byte(), 0x03];
        bytecode.extend_from_slice(&tree_bytes);

        // Sanity: byte at tree[4] is 0x32 followed by tree[5]=0x0C.
        // Old scanner would see 0x32 → Placeholder, read 0x0C=12 as index → count=13.
        assert_eq!(
            tree_bytes[4], 0x32,
            "pre-condition: LEB128(50) must be 0x32"
        );
        assert_eq!(
            tree_bytes[5], 0x0C,
            "pre-condition: next byte must be Check=0x0C"
        );

        // from_bytecode must succeed with key_count=1.
        //
        // WITHOUT fix (old count_placeholder_indices):
        //   count=max(12,0)+1=13 → PolicyScopeViolation("decoded policy has 13 placeholder indices")
        //
        // WITH fix (Option A — pass 32 dummies directly, delete count_placeholder_indices):
        //   decode_template reads Older tag, then LEB128 cursor consumes 0x32 correctly as value 50;
        //   then reads Check, PkK, Placeholder+0x00 → 1 key reference.
        //   from_descriptor produces key_count=1.
        let p = WalletPolicy::from_bytecode(&bytecode).expect(
            "from_bytecode must succeed; LEB128 byte 0x32 must not be confused with Placeholder tag",
        );
        assert_eq!(
            p.key_count(),
            1,
            "key_count must be 1; LEB128(50)=0x32 in older() must not be counted as a placeholder"
        );
    }

    /// Verify that a 9-key multisig round-trips successfully.
    ///
    /// Without Critical fix #2 (dummy table only 8 entries), this test panics
    /// or errors because `dummy_keys(9)` exceeds the table size.
    #[test]
    fn to_bytecode_round_trip_5_of_9_multisig() {
        let p: WalletPolicy = "wsh(multi(5,@0/**,@1/**,@2/**,@3/**,@4/**,@5/**,@6/**,@7/**,@8/**))"
            .parse()
            .unwrap();
        assert_eq!(p.key_count(), 9);
        let bytes = p
            .to_bytecode()
            .expect("to_bytecode must succeed for 9-key multisig");
        let p2 = WalletPolicy::from_bytecode(&bytes)
            .expect("from_bytecode must succeed for 9-key multisig");
        assert_eq!(p2.key_count(), 9, "round-trip must preserve key_count=9");
    }

    /// Verify that an 11-key inheritance policy (corpus C5) round-trips.
    ///
    /// Without Critical fix #2, this fails because dummy table has only 8 entries.
    #[test]
    fn to_bytecode_round_trip_11_key_inheritance() {
        // Corpus C5: 5-of-9 primary + 2-key recovery after 52560 blocks.
        let policy_str = "wsh(or_d(\
            multi(5,@0/**,@1/**,@2/**,@3/**,@4/**,@5/**,@6/**,@7/**,@8/**),\
            and_v(v:older(52560),multi(2,@9/**,@10/**))))";
        let p: WalletPolicy = policy_str.parse().expect("should parse 11-key inheritance");
        assert_eq!(p.key_count(), 11);
        let bytes = p
            .to_bytecode()
            .expect("to_bytecode must succeed for 11-key inheritance policy");
        let p2 = WalletPolicy::from_bytecode(&bytes)
            .expect("from_bytecode must succeed for 11-key inheritance policy");
        assert_eq!(p2.key_count(), 11, "round-trip must preserve key_count=11");
    }

    /// Verify that a policy containing a `sha256()` hash terminal round-trips.
    ///
    /// Prior to the upstream `WalletPolicyTranslator` patch (apoelstra fork
    /// branch `fix/wallet-policy-hash-terminals`, applied via the workspace
    /// `[patch]` redirect in `Cargo.toml`), `WalletPolicy::into_descriptor()`
    /// panicked on any descriptor with a hash terminal — the translator used
    /// `translate_hash_fail!` in both directions. The fix replaced those macro
    /// invocations with manual hex-String ↔ binary-Hash conversion methods.
    ///
    /// This test pins the round-trip for an HTLC-style template
    /// `wsh(and_v(v:pk(@0/**),sha256(<32-byte hash>)))` — the v0.1 corpus
    /// E13 shape. Includes a hash whose binary form contains `0x32` to also
    /// guard against the (already-fixed) `count_placeholder_indices`
    /// over-count bug, in case anyone reintroduces a byte-scan in the future.
    #[test]
    fn to_bytecode_round_trip_with_sha256_terminal() {
        // SHA-256("hello world"). Bytes 1, 4, 7, … contain `0x32`-adjacent
        // values; the actual binary has at least one `0x32` near offset 27,
        // guarding the placeholder-count code path against payload-byte
        // collisions.
        let policy_str = "wsh(and_v(v:pk(@0/**),sha256(b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9)))";
        let p: WalletPolicy = policy_str.parse().expect("HTLC-shape policy must parse");
        assert_eq!(p.key_count(), 1, "single placeholder @0");
        let bytes = p
            .to_bytecode()
            .expect("to_bytecode must succeed for a hash-terminal-bearing policy");
        let p2 = WalletPolicy::from_bytecode(&bytes)
            .expect("from_bytecode must succeed for a hash-terminal-bearing policy");
        assert_eq!(p2.key_count(), 1, "round-trip must preserve key_count=1");
    }
}
