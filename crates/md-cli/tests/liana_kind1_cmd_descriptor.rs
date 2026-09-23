//! Stage 1b task 5, CLI-level regression for `cmd/descriptor.rs`'s G-1
//! caller switch (`descriptor.rs:200`/`:201`): `md descriptor --network <N>`
//! must actually THREAD that network into a wire-kind-1 wallet's derived
//! internal-key xpub, not silently keep rendering mainnet regardless of the
//! flag. This is the risk `to_miniscript_descriptor_with_network`'s own doc
//! names directly — a caller passing the wrong network silently renders a
//! mainnet xpub for what the operator believes is a testnet wallet.
//!
//! Builds the phrase directly via `md_codec::encode_md1_string`/`split` (the
//! wire layer already supports kind 1 encode/decode since stage 1b task 3)
//! rather than through `md encode`/`md compose`, which cannot construct a
//! kind-1 descriptor yet (SPEC §4a, a LATER task's job) — this test only
//! needs an already-encoded kind-1 phrase to feed to `md descriptor`'s
//! DECODE path.

use assert_cmd::Command;
use md_codec::tree::{Body, InternalKey, Node};
use md_codec::use_site_path::UseSitePath;
use md_codec::{Descriptor, OriginPath, PathComponent, PathDecl, PathDeclPaths, Tag, TlvSection};

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

/// A real (distinct, valid) 65-byte `chain_code‖compressed pubkey` TLV
/// entry, derived from the standard BIP-39 "abandon..." test mnemonic at
/// `m/86'/0'/{i}'` — the same recipe `md-codec`'s own `tests/common/mod.rs`
/// `test_xpubs()` uses, reproduced narrowly here (a `md-cli` test crate root
/// cannot reach into `md-codec`'s private `tests/common/`).
fn test_xpub_bytes(i: u32) -> [u8; 65] {
    use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
    use bitcoin::secp256k1::Secp256k1;
    use std::str::FromStr;
    let mn = bip39::Mnemonic::parse(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .expect("known-good mnemonic");
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(bitcoin::Network::Bitcoin, &seed).expect("seed gives master");
    let path = DerivationPath::from_str(&format!("m/86'/0'/{i}'")).expect("valid path");
    let xpriv = master.derive_priv(&secp, &path).expect("derive priv");
    let xpub = Xpub::from_priv(&secp, &xpriv);
    let mut out = [0u8; 65];
    out[..32].copy_from_slice(xpub.chain_code.as_ref());
    out[32..].copy_from_slice(&xpub.public_key.serialize());
    out
}

/// A minimal wallet-policy-mode `tr(LianaUnspendable, {pk(@0),pk(@1)})`.
fn kind1_descriptor() -> Descriptor {
    let tree = Node {
        tag: Tag::Tr,
        body: Body::Tr {
            internal_key: InternalKey::LianaUnspendable,
            tree: Some(Box::new(Node {
                tag: Tag::TapTree,
                body: Body::Children(vec![
                    Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 0 },
                    },
                    Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 1 },
                    },
                ]),
            })),
        },
    };
    // A single-component SHARED origin (not per-key Divergent) keeps the
    // payload as small as this shape allows -- `expand_per_at_n` only
    // refuses on an EMPTY path with no canonical default
    // (`canonicalize.rs`'s `MissingExplicitOrigin` gate), so one non-empty
    // shared component is "explicit" regardless of `tr()`'s non-canonical
    // shape here.
    let mut tlv = TlvSection::new_empty();
    tlv.pubkeys = Some(vec![(0u8, test_xpub_bytes(1)), (1u8, test_xpub_bytes(2))]);
    Descriptor {
        n: 2,
        path_decl: PathDecl {
            n: 2,
            paths: PathDeclPaths::Shared(OriginPath {
                components: vec![PathComponent {
                    hardened: true,
                    value: 48,
                }],
            }),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree,
        tlv,
    }
}

/// One or more md1 phrase chunks for `d` -- a keyed `tr()` carrying two
/// 65-byte xpub TLV entries does not fit the ~80-symbol single-string cap
/// (`Error::PayloadTooLongForSingleString`, measured: this descriptor needs
/// 227), matching every `keyed_*` vendored vector's own phrase files (all
/// chunked). `md descriptor` accepts either form as its positional args
/// (`crates/md-cli/tests/cmd_descriptor.rs`'s own `chunks(name)` helper
/// passes multiple chunk args the same way).
fn phrase_chunks(d: &Descriptor) -> Vec<String> {
    match md_codec::encode_md1_string(d) {
        Ok(s) => vec![s],
        Err(md_codec::Error::PayloadTooLongForSingleString { .. }) => {
            md_codec::split(d).expect("chunked encode must succeed when single-string does not")
        }
        Err(e) => panic!("unexpected encode error: {e}"),
    }
}

#[test]
fn md_descriptor_network_flag_reaches_the_derived_internal_key() {
    let chunks = phrase_chunks(&kind1_descriptor());

    let mainnet = md().arg("descriptor").args(&chunks).output().unwrap();
    assert!(
        mainnet.status.success(),
        "mainnet (default) run failed: {}",
        String::from_utf8_lossy(&mainnet.stderr)
    );
    let mainnet_out = String::from_utf8_lossy(&mainnet.stdout).into_owned();
    assert!(
        mainnet_out.contains("xpub"),
        "default network must render an xpub internal key: {mainnet_out}"
    );

    let testnet = md()
        .args(["descriptor", "--network", "testnet"])
        .args(&chunks)
        .output()
        .unwrap();
    assert!(
        testnet.status.success(),
        "testnet run failed: {}",
        String::from_utf8_lossy(&testnet.stderr)
    );
    let testnet_out = String::from_utf8_lossy(&testnet.stdout).into_owned();
    // Only the INTERNAL key (immediately after `tr(`) is network-dependent
    // -- the two leaf keys are ordinary spendable xpubs from the TLV, whose
    // rendering never consults `network` at all (`xpub_from_tlv_bytes`'s own
    // doc: the network byte is a placeholder, unused by CKDpub), so they
    // stay `xpub...` regardless of `--network`. Check the internal key's own
    // prefix specifically, not "no xpub anywhere in the string".
    let internal_key_field = testnet_out
        .strip_prefix("tr(")
        .and_then(|s| s.split(',').next())
        .unwrap_or_default();
    assert!(
        internal_key_field.starts_with("tpub"),
        "--network testnet must actually reach the derived internal key's \
         base58 prefix -- reverting descriptor.rs's switch to \
         _with_network would render xpub here regardless of the flag: \
         internal key field = {internal_key_field:?}, full output = {testnet_out}"
    );
}

/// Same wallet, no `--network` at all: the network-less default is mainnet
/// (unaffected by this task) -- `descriptor.rs` still works exactly as
/// before when the operator does not ask for testnet.
#[test]
fn md_descriptor_with_no_network_flag_still_works_and_defaults_mainnet() {
    let chunks = phrase_chunks(&kind1_descriptor());
    let out = md().arg("descriptor").args(&chunks).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("xpub"), "got {s}");
}
