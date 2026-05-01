//! file location /home/user/repo/crates/md-codec/tests/address_derivation.rs


//! Integration tests for `Descriptor::derive_address` (md1 v0.14).
//!
//! Each test follows the same shape: derive an account-level xpub from
//! a known mnemonic via rust-bitcoin's bip32 (trusted), pack the
//! `(chain_code, compressed_pubkey)` bytes into the v0.13 `Pubkeys` TLV,
//! then ask md-codec to derive an address and assert it matches a
//! golden vector from the relevant BIP's published test vectors (or, for
//! multisig, a vector cross-checked against rust-bitcoin's own descriptor
//! derivation done in-test through a second path).

use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use md_codec::{
    Descriptor, OriginPath, PathComponent, PathDecl, PathDeclPaths, Tag, TlvSection,
};
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;
use std::str::FromStr;

/// The "abandon abandon abandon abandon abandon abandon abandon abandon
/// abandon abandon abandon about" mnemonic — used by BIP 84, BIP 86,
/// BIP 49, and BIP 44 published test vectors.
const ABANDON_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// Derive the account-level xpub for the abandon-mnemonic at `path`.
/// Returns a 65-byte `(chain_code || compressed_pubkey)` payload as it
/// would appear in a v0.13 `Pubkeys` TLV entry.
fn account_xpub_bytes(path_str: &str) -> [u8; 65] {
    let mn = bip39::Mnemonic::parse(ABANDON_MNEMONIC).expect("known good mnemonic");
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Bitcoin, &seed).expect("seed → master");
    let path = DerivationPath::from_str(path_str).expect("valid path");
    let account_xpriv = master.derive_priv(&secp, &path).expect("derive priv");
    let account_xpub = Xpub::from_priv(&secp, &account_xpriv);
    let mut out = [0u8; 65];
    out[..32].copy_from_slice(account_xpub.chain_code.as_ref());
    out[32..].copy_from_slice(&account_xpub.public_key.serialize());
    out
}

fn origin(components: &[(bool, u32)]) -> OriginPath {
    OriginPath {
        components: components
            .iter()
            .map(|&(hardened, value)| PathComponent { hardened, value })
            .collect(),
    }
}

fn pkk(index: u8) -> Node {
    Node {
        tag: Tag::PkK,
        body: Body::KeyArg { index },
    }
}

/// BIP 84 test vector — `abandon abandon ... about` at `m/84'/0'/0'/0/0`
/// produces P2WPKH `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu`.
/// Source: <https://github.com/bitcoin/bips/blob/master/bip-0084.mediawiki>
#[test]
fn bip84_wpkh_receive_address_zero() {
    let xpub_bytes = account_xpub_bytes("m/84'/0'/0'");
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(origin(&[(true, 84), (true, 0), (true, 0)])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_bytes)]);
            t
        },
    };
    let addr = d.derive_address(0, 0, Network::Bitcoin).unwrap();
    assert_eq!(
        addr.assume_checked().to_string(),
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
    );
}

/// BIP 84 — second receive address `m/84'/0'/0'/0/1` is
/// `bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g`.
#[test]
fn bip84_wpkh_receive_address_one() {
    let xpub_bytes = account_xpub_bytes("m/84'/0'/0'");
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(origin(&[(true, 84), (true, 0), (true, 0)])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_bytes)]);
            t
        },
    };
    let addr = d.derive_address(0, 1, Network::Bitcoin).unwrap();
    assert_eq!(
        addr.assume_checked().to_string(),
        "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g"
    );
}

/// BIP 84 — first change address `m/84'/0'/0'/1/0` is
/// `bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el`. Confirms `chain=1`
/// selects the change branch of the `<0;1>/*` multipath.
#[test]
fn bip84_wpkh_change_address_zero() {
    let xpub_bytes = account_xpub_bytes("m/84'/0'/0'");
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(origin(&[(true, 84), (true, 0), (true, 0)])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_bytes)]);
            t
        },
    };
    let addr = d.derive_address(1, 0, Network::Bitcoin).unwrap();
    assert_eq!(
        addr.assume_checked().to_string(),
        "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el"
    );
}

/// BIP 86 test vector — `abandon abandon ... about` at `m/86'/0'/0'/0/0`
/// produces P2TR keypath-only address
/// `bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr`.
/// Confirms BIP 86 NUMS taproot tweak.
/// Source: <https://github.com/bitcoin/bips/blob/master/bip-0086.mediawiki>
#[test]
fn bip86_tr_keypath_only_receive_address_zero() {
    let xpub_bytes = account_xpub_bytes("m/86'/0'/0'");
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(origin(&[(true, 86), (true, 0), (true, 0)])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Tr,
            body: Body::Tr { key_index: 0, tree: None },
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_bytes)]);
            t
        },
    };
    let addr = d.derive_address(0, 0, Network::Bitcoin).unwrap();
    assert_eq!(
        addr.assume_checked().to_string(),
        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
    );
}

/// BIP 44 test vector — `abandon abandon ... about` at `m/44'/0'/0'/0/0`
/// produces P2PKH address `1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA`.
/// Cross-checked against multiple wallet implementations (Electrum,
/// Sparrow, BlueWallet) using the same well-known test mnemonic.
#[test]
fn bip44_pkh_receive_address_zero() {
    let xpub_bytes = account_xpub_bytes("m/44'/0'/0'");
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(origin(&[(true, 44), (true, 0), (true, 0)])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Pkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_bytes)]);
            t
        },
    };
    let addr = d.derive_address(0, 0, Network::Bitcoin).unwrap();
    assert_eq!(
        addr.assume_checked().to_string(),
        "1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA"
    );
}

/// Same wpkh wallet as `bip84_wpkh_receive_address_zero` but on
/// `Network::Testnet` produces a `tb1q…` address. Verifies network
/// parameter end-to-end.
#[test]
fn bip84_wpkh_testnet_address() {
    let xpub_bytes = account_xpub_bytes("m/84'/0'/0'");
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(origin(&[(true, 84), (true, 0), (true, 0)])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_bytes)]);
            t
        },
    };
    let addr = d.derive_address(0, 0, Network::Testnet).unwrap();
    let s = addr.assume_checked().to_string();
    assert!(s.starts_with("tb1q"), "expected testnet bech32, got {s}");
}

/// 2-of-3 wsh-sortedmulti from three independent abandon-mnemonics-like
/// xpubs. Cross-checks our `(classify_derivable_shape +
/// build_multi_script + Address::p2wsh)` chain against rust-bitcoin's
/// own primitives applied independently in-test.
#[test]
fn wsh_sortedmulti_2_of_3_address() {
    use bitcoin::bip32::ChildNumber;

    // Three different account paths under the same abandon-mnemonic
    // master: 0', 1', 2'. Gives three independent xpubs without
    // needing three distinct mnemonics.
    let xpub_a = account_xpub_bytes("m/48'/0'/0'/2'");
    let xpub_b = account_xpub_bytes("m/48'/0'/1'/2'");
    let xpub_c = account_xpub_bytes("m/48'/0'/2'/2'");

    let d = Descriptor {
        n: 3,
        path_decl: PathDecl {
            n: 3,
            paths: PathDeclPaths::Shared(origin(&[
                (true, 48),
                (true, 0),
                (true, 0),
                (true, 2),
            ])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(0), pkk(1), pkk(2)],
                },
            }]),
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_a), (1u8, xpub_b), (2u8, xpub_c)]);
            t
        },
    };
    let addr = d.derive_address(0, 0, Network::Bitcoin).unwrap();
    let got = addr.assume_checked().to_string();

    // Independent verification: do the same math by hand using
    // rust-bitcoin primitives, no md-codec helpers, then assert match.
    let secp = Secp256k1::verification_only();
    let mut pks: Vec<bitcoin::secp256k1::PublicKey> = vec![];
    for bytes in [&xpub_a, &xpub_b, &xpub_c] {
        let mut chain_code = [0u8; 32];
        chain_code.copy_from_slice(&bytes[..32]);
        let pubkey = bitcoin::secp256k1::PublicKey::from_slice(&bytes[32..]).unwrap();
        let xpub = Xpub {
            network: bitcoin::NetworkKind::Main,
            depth: 0,
            parent_fingerprint: Default::default(),
            child_number: ChildNumber::Normal { index: 0 },
            public_key: pubkey,
            chain_code: bitcoin::bip32::ChainCode::from(chain_code),
        };
        let leaf = xpub
            .derive_pub(&secp, &[
                ChildNumber::Normal { index: 0 },
                ChildNumber::Normal { index: 0 },
            ])
            .unwrap();
        pks.push(leaf.public_key);
    }
    pks.sort_by_key(|p| p.serialize());
    let mut b = bitcoin::blockdata::script::Builder::new().push_int(2);
    for p in &pks {
        b = b.push_key(&bitcoin::PublicKey::new(*p));
    }
    let script = b
        .push_int(3)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let expected = bitcoin::Address::p2wsh(&script, Network::Bitcoin).to_string();

    assert_eq!(got, expected);
    assert!(
        got.starts_with("bc1q"),
        "expected mainnet wsh bech32, got {got}"
    );
}

/// `sh(wsh(sortedmulti(2, ...)))` — BIP 48 type 1 (nested-segwit
/// multi). Independent verification through a parallel rust-bitcoin
/// path; asserts a `3...` mainnet P2SH-form address.
#[test]
fn sh_wsh_sortedmulti_2_of_3_address() {
    use bitcoin::bip32::ChildNumber;

    let xpub_a = account_xpub_bytes("m/48'/0'/0'/1'");
    let xpub_b = account_xpub_bytes("m/48'/0'/1'/1'");
    let xpub_c = account_xpub_bytes("m/48'/0'/2'/1'");

    let d = Descriptor {
        n: 3,
        path_decl: PathDecl {
            n: 3,
            paths: PathDeclPaths::Shared(origin(&[
                (true, 48),
                (true, 0),
                (true, 0),
                (true, 1),
            ])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Sh,
            body: Body::Children(vec![Node {
                tag: Tag::Wsh,
                body: Body::Children(vec![Node {
                    tag: Tag::SortedMulti,
                    body: Body::Variable {
                        k: 2,
                        children: vec![pkk(0), pkk(1), pkk(2)],
                    },
                }]),
            }]),
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_a), (1u8, xpub_b), (2u8, xpub_c)]);
            t
        },
    };
    let addr = d.derive_address(0, 0, Network::Bitcoin).unwrap();
    let got = addr.assume_checked().to_string();

    let secp = Secp256k1::verification_only();
    let mut pks: Vec<bitcoin::secp256k1::PublicKey> = vec![];
    for bytes in [&xpub_a, &xpub_b, &xpub_c] {
        let mut chain_code = [0u8; 32];
        chain_code.copy_from_slice(&bytes[..32]);
        let pubkey = bitcoin::secp256k1::PublicKey::from_slice(&bytes[32..]).unwrap();
        let xpub = Xpub {
            network: bitcoin::NetworkKind::Main,
            depth: 0,
            parent_fingerprint: Default::default(),
            child_number: ChildNumber::Normal { index: 0 },
            public_key: pubkey,
            chain_code: bitcoin::bip32::ChainCode::from(chain_code),
        };
        let leaf = xpub
            .derive_pub(&secp, &[
                ChildNumber::Normal { index: 0 },
                ChildNumber::Normal { index: 0 },
            ])
            .unwrap();
        pks.push(leaf.public_key);
    }
    pks.sort_by_key(|p| p.serialize());
    let mut b = bitcoin::blockdata::script::Builder::new().push_int(2);
    for p in &pks {
        b = b.push_key(&bitcoin::PublicKey::new(*p));
    }
    let script = b
        .push_int(3)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let expected = bitcoin::Address::p2shwsh(&script, Network::Bitcoin).to_string();

    assert_eq!(got, expected);
    assert!(got.starts_with('3'), "expected mainnet P2SH-form, got {got}");
}

/// Round-trip: encode → wrap → unwrap → decode → derive_address yields
/// the same address as deriving on the source descriptor. Confirms the
/// derivation API plays well with the v0.13 wire round-trip.
#[test]
fn round_trip_then_derive_address() {
    let xpub_bytes = account_xpub_bytes("m/84'/0'/0'");
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(origin(&[(true, 84), (true, 0), (true, 0)])),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: {
            let mut t = TlvSection::new_empty();
            t.pubkeys = Some(vec![(0u8, xpub_bytes)]);
            t
        },
    };
    let direct = d
        .derive_address(0, 0, Network::Bitcoin)
        .unwrap()
        .assume_checked()
        .to_string();

    let s = md_codec::encode_md1_string(&d).unwrap();
    let decoded = md_codec::decode_md1_string(&s).unwrap();
    let after = decoded
        .derive_address(0, 0, Network::Bitcoin)
        .unwrap()
        .assume_checked()
        .to_string();

    assert_eq!(direct, after);
    assert_eq!(after, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
}
