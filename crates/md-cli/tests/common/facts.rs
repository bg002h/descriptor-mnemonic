//! The two equality relations of SPEC Acceptance 1, computed INDEPENDENTLY of
//! `src/seat` and `src/decompose`.
//!
//! **SPEND-EQUALITY** (cross-form) — canonicalised template structures equal
//! AND per-slot xpub VALUES and use-site paths equal; origin metadata
//! EXCLUDED, because it is seating/signing guidance rather than script
//! content.
//! **ROUND-TRIP-EQUALITY** (decompose∘compose) — spend-equality AND origin
//! metadata preserved exactly.
//!
//! "xpub VALUE" is the chain code and the public point, NOT the 111-character
//! serialisation. That distinction is the whole of r1 C3: md's `Pubkeys` TLV
//! carries 65 bytes (chain code ‖ compressed point) and md-codec reconstructs
//! a DEPTH-0 xpub from them, so a card-composed descriptor renders
//! `xpub661MyMwAqRbc…` where the input had `xpub6DkFAXWQ2dHxq…` — measured
//! 2026-08-30. Same spending key, same wallet, different string. An assertion
//! on the strings would fail on a wallet that is provably identical, which is
//! why the spec's relation is over values.
//!
//! **Why it is computed here and not called out of `src/`.** The acceptance
//! walks (`tests/acceptance_walks.rs`) and the decompose round trip
//! (`tests/cmd_decompose_roundtrip.rs`) both grade the seating engine, and
//! `seat::compose::spend_equal` is part of what they grade — an acceptance
//! that asked the code under test whether it had succeeded would agree with
//! itself by construction. This module reads the emitted descriptor STRINGS
//! back through rust-miniscript and derives the relation from the parse, so
//! the only thing shared with the implementation is the descriptor grammar.
//!
//! Included with `#[path = "common/facts.rs"] mod facts;` — `tests/common/`
//! holds no `main.rs`, so cargo does not build it as a test target of its own.

#![allow(dead_code)]

use miniscript::ForEachKey;
use miniscript::descriptor::{Descriptor, DescriptorPublicKey};
use std::str::FromStr;

/// Everything the two equality relations are defined over.
#[derive(Debug, PartialEq, Eq)]
pub struct Facts {
    /// The descriptor with every key expression replaced by `@n`, n in
    /// textual order — "canonicalised template structure". Rendered with
    /// `{:#}`, so no checksum takes part: the same wallet spelled with and
    /// without origins carries two different BIP-380 checksums (measured
    /// 2026-08-30 on the pathological wallet: `#s5a2k003` vs `#xn3k4jmt`),
    /// and a checksum in the structure would make two spend-equal forms
    /// compare unequal.
    pub structure: String,
    /// Per slot: chain code ‖ compressed point, hex. The xpub VALUE, which is
    /// what md's wire format actually carries.
    pub values: Vec<String>,
    /// Per slot: everything after the xpub, e.g. `/<0;1>/*`.
    pub use_sites: Vec<String>,
    /// Per slot: `[fingerprint/path]`, or `-` when the key states no origin.
    /// EXCLUDED from spend-equality, INCLUDED in round-trip-equality.
    pub origins: Vec<String>,
}

/// Parse `desc_str` and read the relation's inputs off the parse.
pub fn facts(desc_str: &str) -> Facts {
    let d = Descriptor::<DescriptorPublicKey>::from_str(desc_str)
        .unwrap_or_else(|e| panic!("descriptor must parse ({desc_str}): {e}"));
    let rendered = format!("{d:#}");
    let mut keys: Vec<DescriptorPublicKey> = Vec::new();
    d.for_each_key(|k| {
        keys.push(k.clone());
        true
    });
    keys.sort_by_key(|k| rendered.find(&k.to_string()).unwrap_or(usize::MAX));
    keys.dedup_by(|a, b| a.to_string() == b.to_string());

    let mut structure = rendered.clone();
    let (mut values, mut use_sites, mut origins) = (Vec::new(), Vec::new(), Vec::new());
    for (n, k) in keys.iter().enumerate() {
        let shown = k.to_string();
        structure = structure.replace(&shown, &format!("@{n}"));
        let (origin, xkey) = match k {
            DescriptorPublicKey::XPub(x) => (x.origin.clone(), x.xkey),
            DescriptorPublicKey::MultiXPub(m) => (m.origin.clone(), m.xkey),
            DescriptorPublicKey::Single(_) => panic!("this fixture has no raw keys"),
        };
        values.push(format!(
            "{}{}",
            hex(xkey.chain_code.as_ref()),
            hex(&xkey.public_key.serialize())
        ));
        // The use-site suffix: strip `[origin]` then the base58 xpub.
        let after = match shown.find(']') {
            Some(i) if shown.starts_with('[') => shown[i + 1..].to_string(),
            _ => shown.clone(),
        };
        use_sites.push(match after.find('/') {
            Some(i) => after[i..].to_string(),
            None => String::new(),
        });
        origins.push(match origin {
            Some((f, p)) if p.as_ref().is_empty() => format!("[{f}]"),
            Some((f, p)) => format!("[{f}/{p}]"),
            None => "-".to_string(),
        });
    }
    Facts {
        structure,
        values,
        use_sites,
        origins,
    }
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// SPEC Acceptance 1's SPEND-EQUALITY: structures equal AND per-slot xpub
/// values and use-site paths equal — origin metadata EXCLUDED.
pub fn spend_equal(a: &Facts, b: &Facts) -> bool {
    a.structure == b.structure && a.values == b.values && a.use_sites == b.use_sites
}

/// A spend-equality failure, rendered so the failing HALF is visible.
pub fn spend_equal_report(a: &Facts, b: &Facts) -> String {
    format!(
        "structure: {:?}\n       vs: {:?}\nvalues: {:?}\n    vs: {:?}\nuse-sites: {:?}\n       vs: {:?}",
        a.structure, b.structure, a.values, b.values, a.use_sites, b.use_sites
    )
}
