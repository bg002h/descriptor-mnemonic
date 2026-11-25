//! Bug hunt 2026-11-25 md#1, per Brian's ruling of 2026-11-25: a KEYED card
//! carries every key's origin explicitly, and a reader without the seed never
//! guesses a missing one. Keyless templates keep eliding a canonical origin,
//! and the wallet-policy id of an elided origin is unchanged.

#![allow(missing_docs)]

use std::process::Command;
use std::str::FromStr as _;

/// m/84'/0'/0' account key (depth 3, child 0'), mainnet.
const K3: &str = "xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj";
/// m/48'/0'/0'/2' key (depth 4, child 2'), mainnet.
const K4: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
/// m/84'/1'/0' account key (depth 3, child 0'), testnet.
const TPUB: &str = "tpubDC9Go1KDateW3gS8VXZ6DD1Xu7PgoTdPcf1MX9Z6qVLiHbaeDJ78swPyuQ8YQY19QjtrzkfkZSXwqCcb7XArtid1iLq8Vy55Ydfm4giZh6X";
/// `wpkh(@0/<0;1>/*)`, keyless, origin elided.
const KEYLESS_WPKH: &str = "md1yqpqqxqq8xtwhw4xwn4qh";

const MISSING: &str =
    "warning: the origin path of @0 is NOT on this card, and md does not guess it";

fn run(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        o.status.code().expect("normal exit"),
        String::from_utf8(o.stdout).unwrap(),
        String::from_utf8(o.stderr).unwrap(),
    )
}

fn cards(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(str::to_string)
        .collect()
}

/// A keyed `wpkh` card with an EMPTY origin, the shape `md encode` minted
/// before this fix: the keyless elided card plus K3 and a fingerprint, encoded
/// through the codec directly.
fn legacy_keyed_elided_card() -> Vec<String> {
    let mut d = md_codec::decode::decode_md1_string(KEYLESS_WPKH).expect("decodes");
    let x = bitcoin::bip32::Xpub::from_str(K3).unwrap();
    let mut payload = [0u8; 65];
    payload[..32].copy_from_slice(x.chain_code.as_bytes());
    payload[32..].copy_from_slice(&x.public_key.serialize());
    d.tlv.pubkeys = Some(vec![(0, payload)]);
    d.tlv.fingerprints = Some(vec![(0, [0xaa, 0xbb, 0xcc, 0xdd])]);
    md_codec::chunk::split(&d).expect("mints")
}

// ─── minting ────────────────────────────────────────────────────────────────

#[test]
fn keyed_mint_writes_the_canonical_origin() {
    let k = format!("@0={K3}");
    let (code, elided, err) = run(&["encode", "wpkh(@0/<0;1>/*)", "--key", &k]);
    assert_eq!(code, 0, "{err}");
    let (_, explicit, _) = run(&["encode", "wpkh(@0/<0;1>/*)", "--key", &k, "--path", "bip84"]);
    // The same card as writing the origin out, so the same wallet-policy id.
    assert_eq!(elided, explicit);
    let mut args = vec!["inspect"];
    let c = cards(&elided);
    args.extend(c.iter().map(String::as_str));
    let (code, out, err) = run(&args);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("  @0: m/84'/0'/0'\n"), "{out}");
    assert!(!err.contains("NOT on this card"), "{err}");
}

#[test]
fn keyed_mint_writes_the_canonical_origin_for_the_network() {
    let k = format!("@0={TPUB}");
    let (code, elided, err) = run(&[
        "encode",
        "wpkh(@0/<0;1>/*)",
        "--key",
        &k,
        "--network",
        "testnet",
    ]);
    assert_eq!(code, 0, "{err}");
    let (_, explicit, _) = run(&[
        "encode",
        "wpkh(@0/<0;1>/*)",
        "--key",
        &k,
        "--network",
        "testnet",
        "--path",
        "m/84'/1'/0'",
    ]);
    assert_eq!(elided, explicit);
}

#[test]
fn keyed_mint_fills_only_the_slots_with_no_origin() {
    // X1: K4's chain code and point under a depth-4, child-2' header.
    const X1: &str = "xpub6E6MpT6SMBy1WFsHq7TjScR2sKMSXB3HPPxQ26t3F8WCTbr1mSW1xUGLrT2VZXvV4hj1quWxm15Bkfr1GF47JBnT7WEBMHGSFrQv1Gr6hq2";
    let (k0, k1) = (format!("@0={K4}"), format!("@1={X1}"));
    let (code, out, err) = run(&[
        "encode",
        "wsh(sortedmulti(2,@0/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))",
        "--key",
        &k0,
        "--key",
        &k1,
    ]);
    assert_eq!(code, 0, "{err}");
    let mut args = vec!["inspect"];
    let c = cards(&out);
    args.extend(c.iter().map(String::as_str));
    let (_, out, _) = run(&args);
    assert!(
        out.contains("  @0: m/48'/0'/0'/2'\n  @1: m/48'/0'/1'/2'\n"),
        "{out}"
    );
}

#[test]
fn keyed_mint_refuses_a_key_the_canonical_origin_contradicts() {
    let k = format!("@0={K4}");
    let (code, out, err) = run(&["encode", "wpkh(@0/<0;1>/*)", "--key", &k]);
    assert_eq!((code, out.as_str()), (1, ""), "{err}");
    assert!(
        err.starts_with(
            "md: --key @0: no origin was given for @0, so md would write this shape's standard \
             origin `m/84'/0'/0'` on the card"
        ),
        "{err}"
    );
    assert!(err.contains("Pass --path"), "{err}");
}

#[test]
fn keyed_mint_refuses_an_empty_path() {
    let k = format!("@0={K3}");
    let (code, out, err) = run(&["encode", "wpkh(@0/<0;1>/*)", "--key", &k, "--path", "m"]);
    assert_eq!((code, out.as_str()), (1, ""), "{err}");
    assert!(
        err.starts_with("md: unsupported: @0 would have no origin path on this keyed card"),
        "{err}"
    );
}

#[test]
fn keyed_mint_refuses_a_non_canonical_wrapper_with_no_origin() {
    let k = format!("@0={K3}");
    let (code, out, err) = run(&["encode", "wsh(pk(@0/<0;1>/*))", "--key", &k]);
    assert_eq!((code, out.as_str()), (1, ""), "{err}");
    assert!(
        err.starts_with("md: unsupported: @0 would have no origin path on this keyed card"),
        "{err}"
    );
}

#[test]
fn keyless_mint_still_elides() {
    let (code, out, err) = run(&["encode", "wpkh(@0/<0;1>/*)"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, format!("{KEYLESS_WPKH}\n"));
}

// ─── reading a keyed card that has no origin ───────────────────────────────

#[test]
fn decode_shows_no_master_path_and_does_not_advise_path_m() {
    let c = legacy_keyed_elided_card();
    let mut args = vec!["decode"];
    args.extend(c.iter().map(String::as_str));
    let (code, _, err) = run(&args);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains(MISSING), "{err}");
    assert!(
        err.contains("  @0: [aabbccdd] (origin path not on the card)"),
        "{err}"
    );
    assert!(!err.contains("--path m"), "{err}");
    assert!(!err.contains("@0: m\n"), "{err}");
}

#[test]
fn inspect_shows_no_master_path() {
    let c = legacy_keyed_elided_card();
    let mut args = vec!["inspect"];
    args.extend(c.iter().map(String::as_str));
    let (code, out, err) = run(&args);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("  @0: [aabbccdd] (origin path not on the card)\n"),
        "{out}"
    );
    assert!(!out.contains("aabbccdd/m"), "{out}");
    assert!(err.contains(MISSING), "{err}");
}

#[test]
fn descriptor_and_address_warn() {
    let c = legacy_keyed_elided_card();
    for verb in ["descriptor", "address"] {
        let mut args = vec![verb];
        args.extend(c.iter().map(String::as_str));
        let (code, _, err) = run(&args);
        assert_eq!(code, 0, "{verb}: {err}");
        assert!(err.contains(MISSING), "{verb}: {err}");
    }
}

#[test]
fn keyless_elided_card_draws_no_warning_and_no_path_m_advice() {
    let (code, _, err) = run(&["decode", KEYLESS_WPKH]);
    assert_eq!(code, 0, "{err}");
    assert!(!err.contains("NOT on this card"), "{err}");
    assert!(!err.contains("--path m"), "{err}");
}
