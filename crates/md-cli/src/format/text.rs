use crate::error::CliError;
use md_codec::encode::Descriptor;
use std::fmt::Write as _;

/// Render a `Descriptor` back to a BIP 388 template string with `@i` placeholders.
///
/// Thin delegating wrapper over the canonical renderer in `md_codec` (the
/// single source of truth since md-codec 0.40.0). The ~500-line renderer
/// cluster (`render_node`, `render_wrapper`, `render_multi`, …) used to live
/// here in md-cli; it was lifted verbatim into `md_codec::render` so the
/// `mnemonic` toolkit's `inspect` can emit a byte-identical `template:` line.
/// `md_codec::RenderError` maps into [`CliError::Render`] via the `?` operator
/// (the `From` impl lives in [`crate::error`]).
pub fn descriptor_to_template(d: &Descriptor) -> Result<String, CliError> {
    Ok(md_codec::descriptor_to_template(d)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::template::parse_template;

    #[test]
    fn roundtrip_wpkh_singlepath() {
        let t = "wpkh(@0/*)";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_wsh_multi_2of2() {
        let t = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_sh_wpkh() {
        let t = "sh(wpkh(@0/<0;1>/*))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_tr_keyonly() {
        let t = "tr(@0/<0;1>/*)";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.17 Phase 2 — round-trip of the inheritance pattern through the
    /// text renderer (decode-side path). Ensures `Tag::AndV`, `Tag::Verify`,
    /// `Tag::Older` render correctly when descriptors are reconstructed from
    /// md1 bytecode.
    #[test]
    fn roundtrip_tr_and_v_verify_older_inheritance() {
        let t = "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),older(144)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4a — or_d recovery pattern: `or_d(pk(K1), and_v(v:pk(K2),
    /// older(N)))`. Common BOLT-3-style hot-cold split: hot key spends
    /// immediately; cold key spends after timelock.
    #[test]
    fn roundtrip_tr_or_d_recovery_pattern() {
        let t = "tr(@0/<0;1>/*,or_d(pk(@1/<0;1>/*),and_v(v:pk(@2/<0;1>/*),older(144))))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4a — or_i disjunction round-trip in tap-leaf context.
    #[test]
    fn roundtrip_tr_or_i_disjunction() {
        let t = "tr(@0/<0;1>/*,or_i(pk(@1/<0;1>/*),pk(@2/<0;1>/*)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    // or_c top-level test deferred to Phase 4b: or_c is V-typed at the
    // top level which miniscript rejects as a non-T fragment. Phase 4b's
    // wrapper coverage will enable wrapping or_c with `t:` (or using it
    // inside another fragment) so a valid testable expression exists.

    /// v0.18 Phase 4a — andor ternary: `andor(a, b, c)` is "if a then b else
    /// c". The only ternary fragment in miniscript; exercises Body::Children
    /// with length 3.
    #[test]
    fn roundtrip_tr_and_or_ternary() {
        let t = "tr(@0/<0;1>/*,andor(pk(@1/<0;1>/*),pk(@2/<0;1>/*),pk(@3/<0;1>/*)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4a — `after()` absolute timelock (BIP-65). Distinct from
    /// `older()` (relative timelock, BIP-112). Pinned at a value > 500_000_000
    /// would be a Unix-timestamp; here the height-form is exercised.
    #[test]
    fn roundtrip_tr_and_v_after_absolute_timelock() {
        let t = "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),after(700000)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4b — sha256 hash preimage lock. Tests the 32-byte hash body
    /// path through both walker and renderer. Hash literal is the canonical
    /// "single-bit-set" SHA256 input (1 in binary).
    #[test]
    fn roundtrip_tr_and_v_sha256_hash_lock() {
        let t = "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),sha256(0000000000000000000000000000000000000000000000000000000000000001)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4b — hash160 preimage lock. Tests the 20-byte hash body path.
    #[test]
    fn roundtrip_tr_and_v_hash160_hash_lock() {
        let t = "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),hash160(0000000000000000000000000000000000000001)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4b — segwitv0 thresh with non-key fragment children.
    /// Architect's I3 finding from round 1: Thresh accepts arbitrary fragments
    /// (distinct from Multi/MultiA which take only keys). Phase 4a deferred
    /// this end-to-end test because the second-and-later children require
    /// `s:` (Swap) wrapping per miniscript's typecheck. Phase 4b's wrappers
    /// unblock it.
    ///
    /// Note: segwitv0 desugars bare `pk(K)` to `c:pk_k(K)` at typed-miniscript
    /// parse time. v0.30 SPEC §5.1 makes the walker emit bare `Tag::PkK` for
    /// the key-leaf (no enclosing Tag::Check); the renderer's bare-PkK arm
    /// emits `pk(K)` directly. Round-trip target uses the shorthand.
    #[test]
    fn roundtrip_wsh_thresh_with_non_key_fragment_child() {
        let t =
            "wsh(thresh(2,pk(@0/<0;1>/*),s:pk(@1/<0;1>/*),snj:and_v(v:pk(@2/<0;1>/*),older(144))))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4b — and_b with `s:` wrapper on the second child.
    /// Deferred from Phase 4a (needs Swap).
    #[test]
    fn roundtrip_wsh_and_b_with_swap_wrapper() {
        let t = "wsh(and_b(pk(@0/<0;1>/*),s:pk(@1/<0;1>/*)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4b — or_b with `s:` wrapper. Deferred from Phase 4a.
    #[test]
    fn roundtrip_wsh_or_b_with_swap_wrapper() {
        let t = "wsh(or_b(pk(@0/<0;1>/*),s:pk(@1/<0;1>/*)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4b — or_c at top level via `t:` (or_c is V-typed; `t:X`
    /// = `and_v(X, 1)` which produces a T-typed expression at the root).
    /// Deferred from Phase 4a. Note: rendering of `t:or_c(...)` desugars to
    /// `and_v(or_c(...),1)`, so the round-trip target string differs from
    /// the input — we parse `t:or_c(...)`, walk it (which sees the desugared
    /// form), and the renderer emits the desugared form. This is a parse-
    /// then-render round-trip on the desugared canonical form.
    #[test]
    fn roundtrip_tr_t_or_c_desugars_to_and_v_with_true() {
        // Input: t:or_c(pk(@1), v:pk(@2)) — miniscript's t: prefix
        // Rendered (canonical): and_v(or_c(pk(@1),v:pk(@2)),1)
        let input = "tr(@0/<0;1>/*,t:or_c(pk(@1/<0;1>/*),v:pk(@2/<0;1>/*)))";
        let canonical = "tr(@0/<0;1>/*,and_v(or_c(pk(@1/<0;1>/*),v:pk(@2/<0;1>/*)),1))";
        let d = parse_template(input, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), canonical);
    }

    // NOTE: `render_bare_rawpkh_emits_expr_raw_pkh` (which constructed a `Node`
    // and called the local `render_node` directly) moved to md-codec
    // `src/render.rs` when the renderer cluster was lifted there — the local
    // `render_node` no longer exists in md-cli.
}

use md_codec::chunk::ChunkHeader;
use md_codec::identity::{Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId};

pub fn fmt_md1_id(id: &Md1EncodingId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(64);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}
pub fn fmt_template_id(id: &WalletDescriptorTemplateId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(64);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}
pub fn fmt_policy_id(id: &WalletPolicyId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(32);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}
/// 4-byte fingerprint of a `WalletPolicyId`. v0.14's `WalletPolicyId` has
/// no `fingerprint()` method; we slice the first 4 bytes directly.
pub fn fmt_policy_id_fingerprint(id: &WalletPolicyId) -> String {
    let b = id.as_bytes();
    format!("0x{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3])
}
#[allow(dead_code)] // declared for future chunked-display callers; no current usage
pub fn fmt_chunk_header(h: &ChunkHeader) -> String {
    format!(
        "chunk-set-id=0x{:05x}, count={}, index={}",
        h.chunk_set_id, h.count, h.index
    )
}

#[cfg(test)]
mod hash_tests {
    use super::*;

    #[test]
    fn policy_id_fingerprint_format() {
        let bytes = [
            0x9E, 0x1D, 0x72, 0xB6, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let id = WalletPolicyId::new(bytes);
        assert_eq!(fmt_policy_id_fingerprint(&id), "0x9e1d72b6");
    }
}
