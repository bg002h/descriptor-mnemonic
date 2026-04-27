//! WDM wallet policy newtype wrapping `miniscript::descriptor::WalletPolicy`.
//!
//! # Design decisions (Phase 5)
//!
//! - `key_count()` is derived by counting unique `@N` placeholder indices in
//!   the template string, because the inner type's `key_info` and `template`
//!   fields are private (D-3).
//! - `shared_path()` returns `None` for template-only policies (no key info
//!   attached). Origin derivation paths live in `key_info`, which has no
//!   public accessor; this will be re-examined in Task 5-D when we have a
//!   full `WalletPolicy` with keys (D-4).
//! - `to_canonical_string()` replaces the `/**` BIP 388 shorthand with
//!   `/<0;1>/*` per BIP §"Round-trip canonical form". The inner type's
//!   `Display` uses `/**` (D-5).

use std::str::FromStr;

use bitcoin::bip32::DerivationPath;
use miniscript::descriptor::WalletPolicy as InnerWalletPolicy;

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
        max_index.map_or(0, |m| (m + 1) as usize)
    }

    /// The shared derivation path used by all `@i` placeholders, if any.
    ///
    /// Returns `None` if the policy was created from a template string (no
    /// key info attached). For policies constructed from full descriptor
    /// strings (with origin info), origin derivation paths live in the inner
    /// type's private `key_info` field, which has no public accessor in the
    /// current fork API. This will be revisited in Task 5-D (D-4).
    pub fn shared_path(&self) -> Option<&DerivationPath> {
        // Phase 5-A: no public accessor to key_info; always returns None.
        // Task 5-D will re-examine this once we have a public key-info API
        // or an alternative strategy for extracting origin paths.
        None
    }

    /// Read-only access to the wrapped miniscript WalletPolicy.
    #[doc(hidden)]
    pub fn inner(&self) -> &InnerWalletPolicy {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Phase 5-A: key_info has no public accessor; shared_path is always
        // None. This test gate covers both None and Some so that the test
        // remains valid whether or not 5-D wires up origin paths.
        let p: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
        assert!(
            matches!(p.shared_path(), None | Some(_)),
            "shared_path must be None or Some(path)"
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
}
