//! Shared test fixtures for md-codec and downstream crates.
//!
//! Available only under `#[cfg(test)]` builds of md-codec itself, or
//! when downstream consumers enable the `test-helpers` cargo feature
//! in their `[dev-dependencies]` block.
//!
//! These keys are deterministic-looking compressed secp256k1 pubkeys
//! used purely as placeholder values in unit/integration tests.
//! They are NOT intended for any production purpose.

use std::str::FromStr;

use miniscript::DescriptorPublicKey;

/// First fixture key. Compressed secp256k1 pubkey (no origin metadata).
pub fn dummy_key_a() -> DescriptorPublicKey {
    DescriptorPublicKey::from_str(
        "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
    )
    .unwrap()
}

/// Second fixture key, distinct from `dummy_key_a()`.
pub fn dummy_key_b() -> DescriptorPublicKey {
    DescriptorPublicKey::from_str(
        "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
    )
    .unwrap()
}

/// Third fixture key, distinct from `dummy_key_a()` and `dummy_key_b()`.
pub fn dummy_key_c() -> DescriptorPublicKey {
    DescriptorPublicKey::from_str(
        "02e493dbf1c10d80f3581e4904930b1404cc6c13900ee0758474fa94abe8c4cd13",
    )
    .unwrap()
}
