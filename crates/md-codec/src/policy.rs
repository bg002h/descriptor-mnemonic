//! MD wallet policy newtype wrapping `miniscript::descriptor::WalletPolicy`.
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

use bitcoin::bip32::{DerivationPath, Fingerprint};
use miniscript::descriptor::{DescriptorPublicKey, WalletPolicy as InnerWalletPolicy};

use crate::bytecode::cursor::Cursor;
use crate::bytecode::decode::decode_template;
use crate::bytecode::encode::encode_template;
use crate::bytecode::header::BytecodeHeader;
use crate::bytecode::path::{decode_declaration, encode_declaration};
use crate::chunking::EncodedChunk;
use crate::policy_id::{PolicyId, PolicyIdWords};
use crate::{EncodeOptions, Error};

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

/// A BIP 388 wallet policy with MD-specific canonical-form output and
/// shared-path extraction.
///
/// Thin newtype around `miniscript::descriptor::WalletPolicy`; constructed
/// from a BIP 388 string via `parse()` (i.e. [`std::str::FromStr`]):
///
/// ```
/// use std::str::FromStr;
/// use md_codec::WalletPolicy;
///
/// let p = WalletPolicy::from_str("wsh(pk(@0/**))")?;
/// # Ok::<(), md_codec::Error>(())
/// ```
///
/// Both forms are accepted:
/// - **Template form** (no key info attached): `wsh(pk(@0/**))`. The `/**`
///   suffix is the BIP 388 shorthand for `/<0;1>/*`.
/// - **Full descriptor** (origin + xpubs): `wsh(pk([fingerprint/path]xpub.../<0;1>/*))`
///
/// # Canonical form
///
/// [`Self::to_canonical_string`] returns the BIP 388 §"Round-trip canonical
/// form" output (no whitespace, hardened-with-`'`, `/**` expanded to
/// `/<0;1>/*`). This is the form the codec hashes for the Tier-3
/// [`crate::PolicyId`].
///
/// # Bytecode encoding
///
/// [`Self::to_bytecode`] emits the MD canonical bytecode: a 1-byte format
/// header, a path declaration, and the operator tree. [`Self::from_bytecode`]
/// is its inverse. See the [`crate::bytecode`] module for the on-the-wire
/// format and the BIP draft §"Canonical bytecode".
///
/// # Stability
///
/// Marked `#[non_exhaustive]` so that v0.2+ can grow private fields (e.g. a
/// cached canonical-form string) without breaking pattern matching.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletPolicy {
    inner: InnerWalletPolicy,
    /// The shared derivation path recovered from a bytecode decode, preserved
    /// so a subsequent `to_bytecode` produces a byte-identical re-encoding for
    /// template-only policies.
    ///
    /// `Some(path)` iff this policy was constructed via
    /// [`Self::from_bytecode`]; `None` for policies built via `parse()` /
    /// [`std::str::FromStr`] or any other constructor. Populating this field
    /// from the decoded `Tag::SharedPath` declaration is what makes
    /// `encode → decode → encode` first-pass byte-stable: without it the
    /// re-encode would otherwise pick up the dummy-key origin path
    /// (`m/44'/0'/0'`) attached to the substituted placeholder keys.
    ///
    /// Consulted by [`Self::to_bytecode`] under the Phase B precedence rule:
    /// `EncodeOptions::shared_path` > `decoded_shared_path` > `shared_path()`
    /// > BIP 84 mainnet fallback.
    ///
    /// **Equality semantics:** because `WalletPolicy` derives `PartialEq`,
    /// two logically-equivalent template-only policies — one built via
    /// `parse()` (`decoded_shared_path == None`) and one built via
    /// [`Self::from_bytecode`] (`decoded_shared_path == Some(_)`) — compare
    /// unequal even though their `inner` `WalletPolicy` is structurally
    /// identical. Callers that want construction-path-agnostic equality
    /// should compare via `.to_canonical_string()` instead of `==`.
    decoded_shared_path: Option<DerivationPath>,
}

impl FromStr for WalletPolicy {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        InnerWalletPolicy::from_str(s)
            .map(|inner| WalletPolicy {
                inner,
                decoded_shared_path: None,
            })
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

    /// Encode this policy as canonical MD bytecode.
    ///
    /// Format: `[BytecodeHeader] [PathDeclaration] [TreeBytes]`
    ///
    /// Uses **Approach B** (dummy-key materialization): clones the inner
    /// policy, sets dummy keys via `set_key_info`, materializes a full
    /// `Descriptor<DescriptorPublicKey>` **once**, extracts the shared path
    /// from the descriptor's first key (for policies with real keys), encodes
    /// the tree, then composes the three sections. See PHASE_5_DECISIONS.md D-7.
    ///
    /// # Shared-path precedence (Phase B)
    ///
    /// When choosing the path declaration to emit, `to_bytecode` consults
    /// (in order):
    ///
    /// 0. `opts.shared_path` — explicit caller override (e.g. from CLI
    ///    `--path`). When `Some`, takes precedence over every other source.
    /// 1. `self.decoded_shared_path` — populated by [`Self::from_bytecode`]
    ///    so a `decode → encode` cycle is byte-stable on the first pass.
    /// 2. `self.shared_path()` — for policies parsed from full descriptor
    ///    strings with real origin info.
    /// 3. BIP 84 mainnet fallback (`m/84'/0'/0'`) — for template-only policies
    ///    constructed via `parse()` with no decoded path and no override.
    ///    This is the v0.1 behavior preserved as the final-tier fallback.
    ///
    /// # Breaking change (v0.2)
    ///
    /// The signature changed from `to_bytecode(&self)` to
    /// `to_bytecode(&self, opts: &EncodeOptions)` to thread the
    /// `EncodeOptions::shared_path` override through. Callers that do not
    /// need an override can pass `&EncodeOptions::default()`.
    pub fn to_bytecode(&self, opts: &EncodeOptions) -> Result<Vec<u8>, Error> {
        let count = self.key_count();
        if count > MAX_DUMMY_KEYS {
            return Err(Error::PolicyScopeViolation(format!(
                "policy has {count} placeholder keys; v0.1 supports at most {MAX_DUMMY_KEYS}"
            )));
        }

        // --- Validate fingerprints (E-3) ---
        //
        // If the caller supplied fingerprints, validate the count up front so
        // we fail fast before the (relatively expensive) descriptor
        // materialization. The BIP MUST clause requires one fingerprint per
        // distinct `@i` placeholder.
        if let Some(fps) = &opts.fingerprints {
            if fps.len() != count {
                return Err(Error::FingerprintsCountMismatch {
                    expected: count,
                    got: fps.len(),
                });
            }
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

        // --- Step 4: select shared path per the Phase B precedence rule ---
        //
        // Precedence: opts.shared_path > decoded_shared_path > shared_path() >
        // default_path_for_v0_4_types > BIP 84 mainnet.
        //
        // Tier 0 (`opts.shared_path`) is the explicit caller override — for
        // example from `md encode --path bip48`. It wins unconditionally.
        //
        // Tier 1 (`decoded_shared_path`) is populated by `from_bytecode` so a
        // decode→encode cycle is byte-stable on the first pass. Without it the
        // re-encode would otherwise reach `shared_path()` and surface the
        // dummy-key origin (`m/44'/0'/0'`) that was attached during decode,
        // breaking byte-identity.
        //
        // Tier 2 (`shared_path()`) handles the real-keys case (policy parsed
        // from a full descriptor with origin info). One extra descriptor
        // materialization is acceptable here; the hot template-only case
        // avoids it via the fallback chain.
        //
        // Tier 3 (`default_path_for_v0_4_types`) fires ONLY for the new v0.4
        // top-level types (wpkh, sh(wpkh), sh(wsh)). wsh and tr return None
        // from this helper, falling through to the BIP 84 mainnet default
        // below — preserving wire-format identity with v0.3.x-shaped inputs.
        //
        // Tier 4 (BIP 84 mainnet fallback) fires for wsh/tr template-only
        // policies with no origin info — the pre-v0.4 behavior, preserved
        // unchanged to avoid changing the bytecode of existing v0.3.x inputs.
        let shared_path = opts.shared_path.clone().unwrap_or_else(|| {
            self.decoded_shared_path.clone().unwrap_or_else(|| {
                self.shared_path().unwrap_or_else(|| {
                    default_path_for_v0_4_types(&descriptor).unwrap_or_else(|| {
                        DerivationPath::from_str("m/84'/0'/0'")
                            .expect("hardcoded BIP 84 path is always valid")
                    })
                })
            })
        });

        // --- Step 5: compose [header][path declaration][fingerprints?][tree bytes] ---
        //
        // Per BIP §"Fingerprints block": the block, when present, follows the
        // path declaration and precedes the tree operators. The header bit 2
        // signals presence.
        let header = BytecodeHeader::new_v0(opts.fingerprints.is_some());
        let mut out = Vec::new();
        out.push(header.as_byte());
        out.extend_from_slice(&encode_declaration(&shared_path));
        if let Some(fps) = &opts.fingerprints {
            // Count was validated above; defense-in-depth try_from guards
            // against a future refactor that bypasses the validation funnel.
            // BIP 388 caps placeholder count at 32, well within u8 range.
            let count_u8 =
                u8::try_from(fps.len()).map_err(|_| Error::FingerprintsCountMismatch {
                    expected: count,
                    got: fps.len(),
                })?;
            out.push(crate::bytecode::Tag::Fingerprints.as_byte());
            out.push(count_u8);
            for fp in fps {
                out.extend_from_slice(fp.as_bytes());
            }
        }
        out.extend_from_slice(&tree_bytes);
        Ok(out)
    }

    /// Decode canonical MD bytecode into a `WalletPolicy`.
    ///
    /// The resulting policy is constructed from the decoded descriptor using
    /// dummy keys as placeholders; key_info contains the dummy keys, not real
    /// keys. Real key info must be supplied separately (e.g., during restore
    /// flow via `set_key_info`).
    ///
    /// If the bytecode header signals a fingerprints block (bit 2 = 1), the
    /// block is parsed and validated but discarded. To recover the parsed
    /// fingerprints alongside the policy, use
    /// [`Self::from_bytecode_with_fingerprints`].
    ///
    /// # Errors
    ///
    /// - `Error::InvalidBytecode { .. }` — truncated or malformed header/path,
    ///   or a fingerprints block whose tag, count byte, or 4-byte entries are
    ///   missing or wrong.
    /// - `Error::UnsupportedVersion(v)` — bytecode uses an unsupported version.
    /// - `Error::FingerprintsCountMismatch { .. }` — fingerprints block count
    ///   byte did not equal the placeholder count of the parsed template.
    /// - `Error::PolicyScopeViolation(..)` — the template violates v0.1/v0.2 scope.
    pub fn from_bytecode(bytes: &[u8]) -> Result<Self, Error> {
        Self::from_bytecode_with_fingerprints(bytes).map(|(p, _)| p)
    }

    /// Decode canonical MD bytecode into a `WalletPolicy` and the optional
    /// parsed fingerprints block.
    ///
    /// Same parsing semantics and error conditions as
    /// [`Self::from_bytecode`]; additionally returns
    /// `Some(Vec<Fingerprint>)` when the bytecode contained a fingerprints
    /// block, or `None` when the header bit 2 was 0.
    ///
    /// Used by [`crate::decode()`] to surface fingerprints on the decode-side
    /// public API (`DecodeResult::fingerprints`); not exposed in the
    /// crate-level prelude because external callers should prefer the
    /// pipelined [`crate::decode()`] entry point.
    pub fn from_bytecode_with_fingerprints(
        bytes: &[u8],
    ) -> Result<(Self, Option<Vec<Fingerprint>>), Error> {
        if bytes.is_empty() {
            return Err(Error::InvalidBytecode {
                offset: 0,
                kind: crate::error::BytecodeErrorKind::UnexpectedEnd,
            });
        }

        // --- Step 1: parse and validate the header byte ---
        let header = BytecodeHeader::from_byte(bytes[0])?;

        // --- Step 2: parse the path declaration ---
        let mut cursor = Cursor::new(&bytes[1..]);
        let shared_path = decode_declaration(&mut cursor)?;
        let path_consumed = cursor.offset();

        // --- Step 3 (Phase E): parse the optional fingerprints block ---
        //
        // The block, when present, sits between the path declaration and the
        // template tree (BIP §"Fingerprints block"). We read the tag byte,
        // count byte, and `count * 4` bytes here; count validation against
        // the reconstructed template happens after the tree is decoded.
        //
        // Offset arithmetic: header byte at index 0, path declaration spans
        // bytes[1..1+path_consumed], so the next byte is at 1+path_consumed.
        let mut cursor_offset = 1 + path_consumed;
        let mut fingerprints: Option<Vec<Fingerprint>> = None;
        let mut declared_count: Option<usize> = None;
        if header.fingerprints() {
            // Expect Tag::Fingerprints (0x35).
            if cursor_offset >= bytes.len() {
                return Err(Error::InvalidBytecode {
                    offset: cursor_offset,
                    kind: crate::error::BytecodeErrorKind::UnexpectedEnd,
                });
            }
            let tag_byte = bytes[cursor_offset];
            if tag_byte != crate::bytecode::Tag::Fingerprints.as_byte() {
                return Err(Error::InvalidBytecode {
                    offset: cursor_offset,
                    kind: crate::error::BytecodeErrorKind::UnexpectedTag {
                        expected: crate::bytecode::Tag::Fingerprints.as_byte(),
                        got: tag_byte,
                    },
                });
            }
            cursor_offset += 1;

            // Count byte.
            if cursor_offset >= bytes.len() {
                return Err(Error::InvalidBytecode {
                    offset: cursor_offset,
                    kind: crate::error::BytecodeErrorKind::UnexpectedEnd,
                });
            }
            let count = bytes[cursor_offset] as usize;
            cursor_offset += 1;

            // `count * 4` fingerprint bytes; reject mid-block truncation.
            let fp_bytes_end =
                cursor_offset
                    .checked_add(count * 4)
                    .ok_or(Error::InvalidBytecode {
                        offset: cursor_offset,
                        kind: crate::error::BytecodeErrorKind::UnexpectedEnd,
                    })?;
            if fp_bytes_end > bytes.len() {
                return Err(Error::InvalidBytecode {
                    offset: bytes.len(),
                    kind: crate::error::BytecodeErrorKind::UnexpectedEnd,
                });
            }
            let mut fps: Vec<Fingerprint> = Vec::with_capacity(count);
            for i in 0..count {
                let off = cursor_offset + i * 4;
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&bytes[off..off + 4]);
                fps.push(Fingerprint::from(buf));
            }
            cursor_offset = fp_bytes_end;
            fingerprints = Some(fps);
            declared_count = Some(count);
        }

        // --- Step 4: decode the template tree (Option A fix for D-8) ---
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
        let tree_bytes = &bytes[cursor_offset..];
        let dummies = all_dummy_keys();
        let descriptor = decode_template(tree_bytes, &dummies)?;

        // --- Step 5: construct WalletPolicy from the descriptor ---
        // `from_descriptor` collects `descriptor.iter_pk()` which returns only the
        // keys actually referenced in the decoded tree — not all 32 dummies.
        //
        // We stash the on-wire `shared_path` on the returned `WalletPolicy` so a
        // subsequent `to_bytecode` reproduces the original path declaration
        // verbatim. This is what makes `encode → decode → encode` first-pass
        // byte-stable for template-only policies.
        let inner = InnerWalletPolicy::from_descriptor(&descriptor)
            .map_err(|e| Error::PolicyScopeViolation(e.to_string()))?;
        let policy = WalletPolicy {
            inner,
            decoded_shared_path: Some(shared_path),
        };

        // --- Step 6 (Phase E): validate fingerprint count against template ---
        //
        // The BIP MUST clause: count == max(@i) + 1 == placeholder_count.
        // Validate here, after the template is parsed and we can derive the
        // authoritative placeholder count from the reconstructed policy.
        if let Some(declared) = declared_count {
            let derived = policy.key_count();
            if declared != derived {
                return Err(Error::FingerprintsCountMismatch {
                    expected: derived,
                    got: declared,
                });
            }
        }

        Ok((policy, fingerprints))
    }
}

// ---------------------------------------------------------------------------
// Default path-tier selector (Phase 3 — scoped to v0.4 top-level types)
// ---------------------------------------------------------------------------

/// Returns the natural default `DerivationPath` for v0.4-introduced top-level
/// types when no Tier 0 / Tier 1 path is available. Intentionally scoped:
/// existing `wsh` and `tr` types preserve their pre-v0.4 BIP 84 fallback
/// behavior to avoid changing the bytecode of v0.3.x-shaped no-origin inputs.
///
/// The caller (`to_bytecode`) inserts this as a new fall-through step between
/// Tier 1 (origin-extracted) and the existing BIP 84 final fallback. The
/// `wsh` and `tr` arms return `None`, so they continue to fall through to the
/// BIP 84 default unchanged.
///
/// The returned `DerivationPath` is later translated to a 1-byte indicator by
/// `bytecode/path.rs`'s `encode_path` → `path_to_indicator` dict-lookup; this
/// function does not deal in raw indicator bytes.
fn default_path_for_v0_4_types(
    d: &miniscript::Descriptor<DescriptorPublicKey>,
) -> Option<DerivationPath> {
    use miniscript::Descriptor;
    use miniscript::descriptor::ShInner;
    let path_str = match d {
        Descriptor::Wpkh(_) => "m/84'/0'/0'",
        Descriptor::Sh(sh) => match sh.as_inner() {
            ShInner::Wpkh(_) => "m/49'/0'/0'",
            ShInner::Wsh(_) => "m/48'/0'/0'/1'",
            _ => return None,
        },
        // wsh and tr preserve the pre-v0.4 BIP 84 fallback (wired in
        // to_bytecode's final unwrap_or_else arm). Explicitly returning None
        // here ensures they fall through to that arm unchanged.
        Descriptor::Wsh(_) | Descriptor::Tr(_) => return None,
        _ => return None,
    };
    Some(DerivationPath::from_str(path_str).expect("static path string is always valid"))
}

// ---------------------------------------------------------------------------
// MdBackup
// ---------------------------------------------------------------------------

/// The output of [`crate::encode()`]: chunks ready to engrave, plus the
/// Tier-3 12-word Policy ID for verification.
///
/// # Invariants
///
/// `chunks` is non-empty: a single-string backup has `chunks.len() == 1`
/// with `chunk_index == 0, total_chunks == 1`. A chunked backup has
/// `chunks.len() in 1..=32` and every chunk's `total_chunks` equals
/// `chunks.len()`. (A 1-chunk Chunked backup is possible when the user
/// passes [`crate::EncodeOptions::chunking_mode`] =
/// [`crate::ChunkingMode::ForceChunked`] on a small input.)
///
/// # What to do with this
///
/// The `raw: String` field of each [`crate::EncodedChunk`] is the
/// codex32-derived string starting with `md1…`. Engrave or print these
/// strings on durable media. The 12-word [`crate::PolicyIdWords`] is the
/// human-friendly Tier-3 [`crate::PolicyId`]; users can write this down
/// alongside the engraved cards as a verifier — at decode time, comparing
/// the 12-word form against the recovered policy's `PolicyId` confirms
/// "I have the right backup for this wallet".
///
/// # Stability
///
/// Marked `#[non_exhaustive]` so v0.2+ can add fields (e.g. encoder
/// metadata) without breaking pattern-matching consumers.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdBackup {
    /// The encoded chunks ready to engrave.
    pub chunks: Vec<EncodedChunk>,
    /// The 12-word BIP-39 representation of the Tier-3 Policy ID, for
    /// user verification.
    pub policy_id_words: PolicyIdWords,
    /// Master-key fingerprints associated with this backup, if any.
    ///
    /// Encode-side: mirrors [`crate::EncodeOptions::fingerprints`] supplied
    /// by the caller (so an `encode → user-side state` round-trip is
    /// observable without re-decoding). Decode-side: see
    /// [`crate::DecodeResult::fingerprints`] for the parsed-from-wire
    /// counterpart. `None` iff no fingerprints block was emitted/parsed;
    /// `Some(fps)` with `fps[i]` corresponding to placeholder `@i`.
    ///
    /// Phase E (v0.2). Recovery tools that surface this field to users MUST
    /// flag it as privacy-sensitive — fingerprints leak which seeds match
    /// which placeholders.
    pub fingerprints: Option<Vec<Fingerprint>>,
}

impl MdBackup {
    /// Reconstruct the 16-byte [`PolicyId`] from `policy_id_words`.
    ///
    /// The 12-word form is the storage format; this method converts back to
    /// the binary representation that is the derivation source for
    /// `truncate → ChunkSetId`.
    ///
    /// # Panics
    ///
    /// Panics if the stored `policy_id_words` do not form a valid BIP-39
    /// mnemonic — this should never happen for correctly constructed
    /// `MdBackup` values.
    pub fn policy_id(&self) -> PolicyId {
        let mnemonic = bip39::Mnemonic::parse(self.policy_id_words.to_string())
            .expect("PolicyIdWords must always form a valid BIP-39 mnemonic");
        let entropy = mnemonic.to_entropy();
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&entropy[..16]);
        PolicyId::new(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Tag;
    use crate::policy_id::compute_policy_id;

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
        let bytes = p
            .to_bytecode(&EncodeOptions::default())
            .expect("to_bytecode should succeed");
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
        let bytes = p
            .to_bytecode(&EncodeOptions::default())
            .expect("to_bytecode should succeed");
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
        let bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
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
        let bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath (0x33)"
        );
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
        let bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
        assert_eq!(bytes[0], 0x00, "header must be 0x00");
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath"
        );
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
    fn from_bytecode_with_fingerprints_flag_no_block_is_truncated() {
        // 0x04 = version 0, fingerprints flag set. Phase E removed the v0.1
        // PolicyScopeViolation rejection, so this byte sequence is now
        // structurally a valid header. With only the path declaration (and
        // no fingerprints block following), the decoder must report
        // UnexpectedEnd when it reaches for the Tag::Fingerprints byte.
        let result = WalletPolicy::from_bytecode(&[0x04, Tag::SharedPath.as_byte(), 0x03]);
        assert!(
            matches!(
                result,
                Err(Error::InvalidBytecode {
                    kind: crate::error::BytecodeErrorKind::UnexpectedEnd,
                    ..
                })
            ),
            "expected UnexpectedEnd for header 0x04 with no fingerprints block, got {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Free-function wrappers
    // -----------------------------------------------------------------------

    #[test]
    fn encode_bytecode_free_fn_matches_method() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let via_method = p.to_bytecode(&EncodeOptions::default()).unwrap();
        let via_fn = crate::encode_bytecode(&p).unwrap();
        assert_eq!(
            via_method, via_fn,
            "encode_bytecode free fn must match to_bytecode method"
        );
    }

    #[test]
    fn decode_bytecode_free_fn_matches_method() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
        let via_method = WalletPolicy::from_bytecode(&bytes).unwrap();
        let via_fn = crate::decode_bytecode(&bytes).unwrap();
        assert_eq!(
            via_method.key_count(),
            via_fn.key_count(),
            "decode_bytecode free fn must match from_bytecode method"
        );
    }

    // -----------------------------------------------------------------------
    // compute_policy_id_for_policy
    // -----------------------------------------------------------------------

    #[test]
    fn compute_policy_id_for_policy_matches_compute_policy_id_of_to_bytecode() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let bytecode = p.to_bytecode(&EncodeOptions::default()).unwrap();
        let direct = compute_policy_id(&bytecode);
        let via_policy = crate::policy_id::compute_policy_id_for_policy(&p).unwrap();
        assert_eq!(
            direct, via_policy,
            "compute_policy_id_for_policy must equal compute_policy_id(to_bytecode())"
        );
    }

    // -----------------------------------------------------------------------
    // Critical fix regression tests (Task 5-B review)
    // -----------------------------------------------------------------------

    /// Verify that LEB128-encoded data bytes whose value collides with the
    /// `Tag::Placeholder` byte do not cause `count_placeholder_indices` to
    /// spuriously report extra keys during `from_bytecode`.
    ///
    /// In v0.6 the Placeholder tag byte is `0x33` (=51 decimal), so we use
    /// `older(51)` to manufacture the collision: `older(51)` encodes as
    /// `[Older, LEB128(51)=0x33]` — the LEB128 byte equals the Placeholder
    /// tag byte. A naive scan-based decoder would mistake the data byte for
    /// a Placeholder tag and read the next byte as a placeholder index.
    ///
    /// Without the fix (old `count_placeholder_indices` pre-scan) this test
    /// fails. With Option A (delete the pre-scan, pass 32 dummies directly
    /// to `decode_template`) the LEB128 byte is consumed correctly by the
    /// Older decoder and never confused with a Placeholder tag.
    ///
    /// The bytecode is constructed directly to control the exact byte layout.
    #[test]
    fn from_bytecode_leb128_byte_collides_with_placeholder_tag_not_counted() {
        use crate::bytecode::Tag;
        // Tree bytes for wsh(and_v(v:older(51), c:pk_k(@0/**))) where
        // older(51) encodes varint 51 = 0x33 = Tag::Placeholder.as_byte().
        let tree_bytes: Vec<u8> = vec![
            Tag::Wsh.as_byte(),         // [0]
            Tag::AndV.as_byte(),        // [1]
            Tag::Verify.as_byte(),      // [2] v: wrapper for older
            Tag::Older.as_byte(),       // [3]
            0x33,                       // [4] LEB128(51) — collides with Placeholder tag byte
            Tag::Check.as_byte(),       // [5] OLD scanner would read as spurious placeholder index
            Tag::PkK.as_byte(),         // [6]
            Tag::Placeholder.as_byte(), // [7] the REAL placeholder
            0x00,                       // [8] index 0
        ];

        // Assemble full MD bytecode: header(0x00) + SharedPath + BIP84=0x03 + tree
        let mut bytecode: Vec<u8> = vec![0x00, Tag::SharedPath.as_byte(), 0x03];
        bytecode.extend_from_slice(&tree_bytes);

        // Sanity: byte at tree[4] equals Tag::Placeholder.as_byte() (the collision),
        // and the byte after it is Tag::Check (which OLD scanner would read as index).
        assert_eq!(
            tree_bytes[4],
            Tag::Placeholder.as_byte(),
            "pre-condition: LEB128(51) must equal Tag::Placeholder.as_byte()"
        );
        assert_eq!(
            tree_bytes[5],
            Tag::Check.as_byte(),
            "pre-condition: next byte must be Tag::Check"
        );

        // from_bytecode must succeed with key_count=1. Without the fix, the
        // pre-scan would over-count placeholder indices and trigger
        // PolicyScopeViolation. With Option A, decode_template consumes the
        // LEB128 byte inside Older correctly and only the real Placeholder
        // contributes to the key count.
        let p = WalletPolicy::from_bytecode(&bytecode).expect(
            "from_bytecode must succeed; LEB128 byte equal to Placeholder tag must not be confused",
        );
        assert_eq!(
            p.key_count(),
            1,
            "key_count must be 1; LEB128 byte inside older() must not be counted as a placeholder"
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
            .to_bytecode(&EncodeOptions::default())
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
            .to_bytecode(&EncodeOptions::default())
            .expect("to_bytecode must succeed for 11-key inheritance policy");
        let p2 = WalletPolicy::from_bytecode(&bytes)
            .expect("from_bytecode must succeed for 11-key inheritance policy");
        assert_eq!(p2.key_count(), 11, "round-trip must preserve key_count=11");
    }

    // Hash-terminal round-trip tests.
    //
    // Prior to the upstream `WalletPolicyTranslator` patch (apoelstra fork
    // branch `fix/wallet-policy-hash-terminals`, applied via the workspace
    // `[patch]` redirect in `Cargo.toml`), `WalletPolicy::into_descriptor()`
    // panicked on any descriptor with a hash terminal — the translator used
    // `translate_hash_fail!` in both directions. The fix replaced those macro
    // invocations with manual hex-String ↔ binary-Hash conversion methods.
    //
    // The single-type tests below pin one of each hash type
    // (sha256/hash256/ripemd160/hash160). The pair test exercises every
    // ordered combination of two distinct hash types in a single policy.
    //
    // Lowercase-hex test vectors of the correct length per hash type:
    // sha256/hash256 = 32 bytes (64 hex), ripemd160/hash160 = 20 bytes (40 hex).
    // The sha256 vector is SHA-256("hello world") whose binary form contains
    // `0x32` near offset 27, additionally guarding the (already-fixed)
    // `count_placeholder_indices` byte-scan bug against re-introduction.
    const HTLC_SHA256: &str = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    const HTLC_HASH256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const HTLC_RIPEMD160: &str = "1234567890abcdef1234567890abcdef12345678";
    const HTLC_HASH160: &str = "fedcba0987654321fedcba0987654321fedcba09";

    /// Round-trip a policy through `to_bytecode` → `from_bytecode` and
    /// assert the key count is preserved.
    fn assert_bytecode_round_trip(policy_str: &str, expected_key_count: usize) {
        let p: WalletPolicy = policy_str
            .parse()
            .unwrap_or_else(|e| panic!("policy must parse ({policy_str:?}): {e}"));
        assert_eq!(p.key_count(), expected_key_count, "input key_count");
        let bytes = p
            .to_bytecode(&EncodeOptions::default())
            .unwrap_or_else(|e| panic!("to_bytecode must succeed for {policy_str:?}: {e}"));
        let p2 = WalletPolicy::from_bytecode(&bytes)
            .unwrap_or_else(|e| panic!("from_bytecode must succeed for {policy_str:?}: {e}"));
        assert_eq!(
            p2.key_count(),
            expected_key_count,
            "round-trip must preserve key_count"
        );
    }

    #[test]
    fn to_bytecode_round_trip_with_sha256_terminal() {
        assert_bytecode_round_trip(&format!("wsh(and_v(v:pk(@0/**),sha256({HTLC_SHA256})))"), 1);
    }

    #[test]
    fn to_bytecode_round_trip_with_hash256_terminal() {
        assert_bytecode_round_trip(
            &format!("wsh(and_v(v:pk(@0/**),hash256({HTLC_HASH256})))"),
            1,
        );
    }

    #[test]
    fn to_bytecode_round_trip_with_ripemd160_terminal() {
        assert_bytecode_round_trip(
            &format!("wsh(and_v(v:pk(@0/**),ripemd160({HTLC_RIPEMD160})))"),
            1,
        );
    }

    #[test]
    fn to_bytecode_round_trip_with_hash160_terminal() {
        assert_bytecode_round_trip(
            &format!("wsh(and_v(v:pk(@0/**),hash160({HTLC_HASH160})))"),
            1,
        );
    }

    /// Round-trip every ordered pair of distinct hash types in the same
    /// policy, end-to-end through `to_bytecode` / `from_bytecode`.
    /// 4 hash types × 3 = 12 ordered pairs.
    ///
    /// This is the MD-side mirror of the fork's
    /// `all_ordered_pairs_of_distinct_hash_types_round_trip` test; here we
    /// additionally exercise the bytecode encoder/decoder over each pair,
    /// confirming that policies with multiple hash types of any kind
    /// round-trip cleanly through the canonical bytecode and back.
    #[test]
    fn to_bytecode_round_trip_with_all_pairs_of_hash_types() {
        let kinds: &[(&str, &str)] = &[
            ("sha256", HTLC_SHA256),
            ("hash256", HTLC_HASH256),
            ("ripemd160", HTLC_RIPEMD160),
            ("hash160", HTLC_HASH160),
        ];
        let mut pair_count = 0;
        for (a_idx, (a_kind, a_hex)) in kinds.iter().enumerate() {
            for (b_idx, (b_kind, b_hex)) in kinds.iter().enumerate() {
                if a_idx == b_idx {
                    continue;
                }
                let policy_str =
                    format!("wsh(and_v(v:and_v(v:pk(@0/**),{a_kind}({a_hex})),{b_kind}({b_hex})))");
                assert_bytecode_round_trip(&policy_str, 1);
                pair_count += 1;
            }
        }
        assert_eq!(pair_count, 12, "expected 4·3 = 12 ordered pairs");
    }

    // --- MdBackup ---

    #[test]
    fn md_backup_policy_id_round_trips_via_words() {
        let original_id = PolicyId::new([0xABu8; 16]);
        let words = original_id.to_words();
        let backup = MdBackup {
            chunks: vec![],
            policy_id_words: words,
            fingerprints: None,
        };
        let recovered_id = backup.policy_id();
        assert_eq!(
            recovered_id, original_id,
            "policy_id() must recover the original PolicyId"
        );
    }

    // -----------------------------------------------------------------------
    // decoded_shared_path — first-pass byte-stable round-trip
    // -----------------------------------------------------------------------

    /// `from_bytecode` must populate `decoded_shared_path` from the wire shared
    /// path, and `to_bytecode` must consult it so a `decode → encode` cycle is
    /// byte-identical to the original bytecode for template-only policies.
    ///
    /// This guards against two regressions simultaneously:
    /// - BIP 84 default fallback in `to_bytecode` (would emit `m/84'/0'/0'`)
    /// - dummy-key origin path leaking through (would emit `m/44'/0'/0'`)
    ///
    /// Using `m/48'/0'/0'/2'` (BIP 48 multisig P2WSH; named indicator 0x05)
    /// distinguishes the correct behavior from both fallbacks.
    #[test]
    fn from_bytecode_populates_decoded_shared_path_consulted_by_to_bytecode() {
        use crate::bytecode::Tag;

        // Build a minimal valid bytecode: header + SharedPath(BIP 48 indicator 0x05) + tree.
        // Tree: wsh(pk(@0/**)) = [Wsh, Check, PkK, Placeholder, 0x00].
        let original: Vec<u8> = vec![
            0x00,                       // header v0, no fingerprints
            Tag::SharedPath.as_byte(),  // 0x33
            0x05,                       // BIP 48 P2WSH multisig: m/48'/0'/0'/2'
            Tag::Wsh.as_byte(),         // 0x05
            Tag::Check.as_byte(),       // 0x0C
            Tag::PkK.as_byte(),         // 0x1B
            Tag::Placeholder.as_byte(), // 0x32
            0x00,                       // placeholder index 0
        ];

        // Decode then immediately re-encode.
        let policy = WalletPolicy::from_bytecode(&original).expect("decode must succeed");

        // decoded_shared_path must be populated with m/48'/0'/0'/2'.
        let expected_path = DerivationPath::from_str("m/48'/0'/0'/2'").unwrap();
        assert_eq!(
            policy.decoded_shared_path.as_ref(),
            Some(&expected_path),
            "from_bytecode must populate decoded_shared_path with the on-wire path"
        );

        // Re-encoding must produce byte-identical output (FIRST-pass byte-stability).
        let re_encoded = policy
            .to_bytecode(&EncodeOptions::default())
            .expect("re-encode must succeed");
        assert_eq!(
            re_encoded, original,
            "to_bytecode must consult decoded_shared_path; first-pass byte-equality required"
        );

        // Specifically the path-indicator byte must NOT be the BIP 84 fallback (0x03)
        // nor the dummy-key origin path (which would serialize differently). It must
        // be the BIP 48 named indicator 0x05.
        assert_eq!(
            re_encoded[2], 0x05,
            "shared-path indicator must round-trip as 0x05 (m/48'/0'/0'/2'), not the 0x03 BIP 84 fallback"
        );
    }

    #[test]
    fn parse_does_not_set_decoded_shared_path() {
        // Default constructor (FromStr) must leave decoded_shared_path as None.
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        assert!(
            p.decoded_shared_path.is_none(),
            "FromStr-constructed policy must have decoded_shared_path == None"
        );
    }

    // -----------------------------------------------------------------------
    // EncodeOptions::shared_path override (Phase B tier 0)
    // -----------------------------------------------------------------------

    /// `EncodeOptions::shared_path` must apply to the bytecode's path
    /// declaration when set, replacing the default-fallback BIP 84 path.
    ///
    /// We pick `m/48'/0'/0'/2'` (BIP 48 P2WSH multisig, named indicator
    /// 0x05) so the assertion distinguishes the override from the
    /// default-tier BIP 84 mainnet (indicator 0x03).
    #[test]
    fn to_bytecode_honors_encode_options_shared_path_override() {
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        let custom = DerivationPath::from_str("m/48'/0'/0'/2'").unwrap();
        let opts = EncodeOptions::default().with_shared_path(custom);
        let bytes = p
            .to_bytecode(&opts)
            .expect("to_bytecode with override must succeed");

        // Header byte = 0x00, then SharedPath tag = 0x33, then indicator.
        assert_eq!(bytes[0], 0x00, "header must be 0x00");
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath"
        );
        assert_eq!(
            bytes[2], 0x05,
            "override path m/48'/0'/0'/2' must serialize as named indicator 0x05, not the default 0x03"
        );
    }

    /// Tier-0 (`EncodeOptions::shared_path`) must beat tier-1
    /// (`WalletPolicy.decoded_shared_path`).
    ///
    /// Setup: build a `WalletPolicy` whose `decoded_shared_path` is
    /// `m/84'/0'/0'` (BIP 84 mainnet, indicator 0x03), then encode with an
    /// override pointing to `m/48'/0'/0'/2'` (indicator 0x05). The result
    /// must reflect 0x05 — proving the override wins.
    #[test]
    fn to_bytecode_override_wins_over_decoded_shared_path() {
        use crate::bytecode::Tag;

        // Construct a bytecode whose shared path is m/84'/0'/0' (indicator 0x03),
        // then decode it to populate `decoded_shared_path`.
        let original: Vec<u8> = vec![
            0x00,                       // header v0, no fingerprints
            Tag::SharedPath.as_byte(),  // 0x33
            0x03,                       // BIP 84 mainnet: m/84'/0'/0'
            Tag::Wsh.as_byte(),         // 0x05
            Tag::Check.as_byte(),       // 0x0C
            Tag::PkK.as_byte(),         // 0x1B
            Tag::Placeholder.as_byte(), // 0x32
            0x00,                       // placeholder index 0
        ];
        let policy = WalletPolicy::from_bytecode(&original).expect("decode must succeed");
        // Sanity: the decoded path is populated as tier 1.
        assert_eq!(
            policy.decoded_shared_path.as_ref(),
            Some(&DerivationPath::from_str("m/84'/0'/0'").unwrap()),
            "from_bytecode must populate decoded_shared_path with m/84'/0'/0'"
        );

        // Baseline: re-encode with default options to confirm tier-1 is what
        // produces the m/84'/0'/0' (0x03) byte. This makes the override-wins
        // assertion below catch any future bug where bytes[2] happens to
        // coincide with 0x05 for an unrelated reason.
        let baseline = policy
            .to_bytecode(&EncodeOptions::default())
            .expect("baseline re-encode must succeed");
        assert_eq!(baseline[2], 0x03, "baseline must round-trip to BIP 84");

        // Override (tier 0) selects m/48'/0'/0'/2' (indicator 0x05); it MUST win
        // over the populated decoded_shared_path (tier 1, indicator 0x03).
        let override_path = DerivationPath::from_str("m/48'/0'/0'/2'").unwrap();
        let opts = EncodeOptions::default().with_shared_path(override_path);
        let bytes = policy
            .to_bytecode(&opts)
            .expect("to_bytecode with override must succeed");

        assert_eq!(
            bytes[2], 0x05,
            "tier-0 EncodeOptions::shared_path override must beat tier-1 decoded_shared_path"
        );
        assert_ne!(
            bytes[2], 0x03,
            "the override must NOT be silently ignored in favor of decoded_shared_path"
        );
        assert_ne!(
            bytes, baseline,
            "override-encoded bytes must differ from baseline (no-override) bytes"
        );
    }

    /// When `EncodeOptions::shared_path` is `None`, the precedence chain falls
    /// back to `decoded_shared_path` (tier 1). This is the existing Phase A
    /// invariant; this test pins it after the tier-0 addition so a future
    /// regression in the override branch cannot silently lose tier-1 behavior.
    #[test]
    fn to_bytecode_default_options_still_consult_decoded_shared_path() {
        use crate::bytecode::Tag;

        let original: Vec<u8> = vec![
            0x00,
            Tag::SharedPath.as_byte(),
            0x05, // BIP 48: m/48'/0'/0'/2'
            Tag::Wsh.as_byte(),
            Tag::Check.as_byte(),
            Tag::PkK.as_byte(),
            Tag::Placeholder.as_byte(),
            0x00,
        ];
        let policy = WalletPolicy::from_bytecode(&original).expect("decode must succeed");

        // No override: tier 1 wins.
        let bytes = policy
            .to_bytecode(&EncodeOptions::default())
            .expect("re-encode must succeed");
        assert_eq!(
            bytes, original,
            "with override=None, encode-decode-encode is byte-stable via decoded_shared_path"
        );
    }

    #[test]
    fn md_backup_struct_construction() {
        use crate::BchCode;
        use crate::chunking::EncodedChunk;

        let id = PolicyId::new([0x01u8; 16]);
        let words = id.to_words();
        let chunk = EncodedChunk {
            raw: "md10xsmoke".to_string(),
            chunk_index: 0,
            total_chunks: 1,
            code: BchCode::Regular,
        };
        let backup = MdBackup {
            chunks: vec![chunk],
            policy_id_words: words,
            fingerprints: None,
        };
        assert_eq!(backup.chunks.len(), 1);
        assert_eq!(backup.chunks[0].chunk_index, 0);
        assert!(backup.fingerprints.is_none());
    }

    // -----------------------------------------------------------------------
    // Phase 3 — Default path-tier selector (scoped to v0.4 types)
    //
    // Three default-tier tests verify that wpkh/sh-wpkh/sh-wsh pick the
    // right BIP-standard indicator when no Tier 0 or Tier 1 path is present.
    // Three override tests confirm --path flows through Tier 0 unchanged.
    // One regression test pins wsh-no-origin behavior to preserve v0.3.x
    // wire format (wsh must continue to emit BIP 84 indicator 0x03, not a
    // newly-scoped default, because wsh is explicitly excluded from the
    // v0.4 selector).
    // -----------------------------------------------------------------------

    /// `wpkh(@0/**)` with no path override or key origin must use BIP 84
    /// (`m/84'/0'/0'`, indicator `0x03`) as its default shared-path.
    ///
    /// byte[0] = 0x00 (header), byte[1] = 0x33 (SharedPath tag),
    /// byte[2] = indicator.
    #[test]
    fn wpkh_default_tier_is_bip84() {
        let p: WalletPolicy = "wpkh(@0/**)".parse().expect("wpkh must parse");
        let bytes = p
            .to_bytecode(&EncodeOptions::default())
            .expect("to_bytecode must succeed for wpkh");
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath"
        );
        assert_eq!(
            bytes[2], 0x03,
            "wpkh default path must be BIP 84 indicator 0x03 (m/84'/0'/0'); got 0x{:02x}",
            bytes[2]
        );
    }

    /// `sh(wpkh(@0/**))` with no path override or key origin must use BIP 49
    /// (`m/49'/0'/0'`, indicator `0x02`) as its default shared-path.
    #[test]
    fn sh_wpkh_default_tier_is_bip49() {
        let p: WalletPolicy = "sh(wpkh(@0/**))".parse().expect("sh(wpkh) must parse");
        let bytes = p
            .to_bytecode(&EncodeOptions::default())
            .expect("to_bytecode must succeed for sh(wpkh)");
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath"
        );
        assert_eq!(
            bytes[2], 0x02,
            "sh(wpkh) default path must be BIP 49 indicator 0x02 (m/49'/0'/0'); got 0x{:02x}",
            bytes[2]
        );
    }

    /// `sh(wsh(sortedmulti(2,@0/**,@1/**,@2/**)))` with no path override or
    /// key origin must use BIP 48/1' nested (`m/48'/0'/0'/1'`, indicator
    /// `0x06`) as its default shared-path.
    #[test]
    fn sh_wsh_default_tier_is_bip48_nested() {
        let p: WalletPolicy = "sh(wsh(sortedmulti(2,@0/**,@1/**,@2/**)))"
            .parse()
            .expect("sh(wsh(sortedmulti)) must parse");
        let bytes = p
            .to_bytecode(&EncodeOptions::default())
            .expect("to_bytecode must succeed for sh(wsh(sortedmulti))");
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath"
        );
        assert_eq!(
            bytes[2], 0x06,
            "sh(wsh) default path must be BIP 48/1' indicator 0x06 (m/48'/0'/0'/1'); got 0x{:02x}",
            bytes[2]
        );
    }

    /// Tier 0 (`EncodeOptions::shared_path`) must override the v0.4 default
    /// for `wpkh`. Supplying `m/48'/0'/0'/2'` (indicator 0x05) must beat
    /// the natural BIP 84 default (indicator 0x03).
    #[test]
    fn wpkh_path_override_wins() {
        let p: WalletPolicy = "wpkh(@0/**)".parse().expect("wpkh must parse");
        let custom = DerivationPath::from_str("m/48'/0'/0'/2'").unwrap();
        let opts = EncodeOptions::default().with_shared_path(custom);
        let bytes = p.to_bytecode(&opts).expect("to_bytecode must succeed");
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath"
        );
        assert_eq!(
            bytes[2], 0x05,
            "tier-0 override must win over wpkh default; expected 0x05 (m/48'/0'/0'/2'), \
             got 0x{:02x}",
            bytes[2]
        );
    }

    /// Tier 0 must override the v0.4 default for `sh(wpkh)`. Supplying
    /// `m/84'/0'/0'` (indicator 0x03) must beat the natural BIP 49 default
    /// (indicator 0x02).
    #[test]
    fn sh_wpkh_path_override_wins() {
        let p: WalletPolicy = "sh(wpkh(@0/**))".parse().expect("sh(wpkh) must parse");
        let custom = DerivationPath::from_str("m/84'/0'/0'").unwrap();
        let opts = EncodeOptions::default().with_shared_path(custom);
        let bytes = p.to_bytecode(&opts).expect("to_bytecode must succeed");
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath"
        );
        assert_eq!(
            bytes[2], 0x03,
            "tier-0 override must win over sh(wpkh) default; expected 0x03 (m/84'/0'/0'), \
             got 0x{:02x}",
            bytes[2]
        );
    }

    /// Tier 0 must override the v0.4 default for `sh(wsh)`. Supplying
    /// `m/84'/0'/0'` (indicator 0x03) must beat the natural BIP 48/1'
    /// nested default (indicator 0x06).
    #[test]
    fn sh_wsh_path_override_wins() {
        let p: WalletPolicy = "sh(wsh(sortedmulti(2,@0/**,@1/**,@2/**)))"
            .parse()
            .expect("sh(wsh(sortedmulti)) must parse");
        let custom = DerivationPath::from_str("m/84'/0'/0'").unwrap();
        let opts = EncodeOptions::default().with_shared_path(custom);
        let bytes = p.to_bytecode(&opts).expect("to_bytecode must succeed");
        assert_eq!(
            bytes[1],
            Tag::SharedPath.as_byte(),
            "byte[1] must be Tag::SharedPath"
        );
        assert_eq!(
            bytes[2], 0x03,
            "tier-0 override must win over sh(wsh) default; expected 0x03 (m/84'/0'/0'), \
             got 0x{:02x}",
            bytes[2]
        );
    }

    /// Regression: a template-only `wsh(pk(@0/**))` policy (no origin info,
    /// no Tier 0 override) MUST continue to emit the BIP 84 fallback
    /// indicator `0x03` — identical to v0.3.x behavior.
    ///
    /// The v0.4 default-tier selector is intentionally SCOPED to exclude
    /// `wsh` and `tr` top-level types. This test pins the preserved
    /// no-origin wsh bytecode so any accidental scope creep of the new
    /// selector is caught immediately.
    ///
    /// Expected bytecode:
    ///   [0x00, 0x33, 0x03, 0x05, 0x0C, 0x1B, 0x32, 0x00]
    ///    hdr   SP    BIP84  Wsh  Chk   PkK   Plhd  idx=0
    #[test]
    fn wsh_no_origin_default_unchanged_from_v0_3() {
        use crate::bytecode::Tag;

        let p: WalletPolicy = "wsh(pk(@0/**))".parse().expect("wsh must parse");
        let bytes = p
            .to_bytecode(&EncodeOptions::default())
            .expect("to_bytecode must succeed");

        // Pin the full bytecode — any change breaks v0.3.x wire compatibility.
        let expected: Vec<u8> = vec![
            0x00,                       // header v0, no fingerprints
            Tag::SharedPath.as_byte(),  // 0x33
            0x03,                       // BIP 84 fallback indicator (preserved from v0.3.x)
            Tag::Wsh.as_byte(),         // 0x05
            Tag::Check.as_byte(),       // 0x0C
            Tag::PkK.as_byte(),         // 0x1B
            Tag::Placeholder.as_byte(), // 0x32
            0x00,                       // placeholder index 0
        ];

        assert_eq!(
            bytes, expected,
            "wsh-no-origin bytecode must be byte-identical to v0.3.x baseline; \
             the v0.4 selector must NOT fire for wsh top-level type.\n\
             expected: {expected:?}\n\
             got:      {bytes:?}"
        );
    }
}
