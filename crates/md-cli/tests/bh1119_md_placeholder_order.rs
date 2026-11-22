//! bughunt-1119 row #5 (`design/FIX_bh1119_md_placeholder_order.md`): CLI rows.
//! A template that numbers its slots out of first-occurrence order mints the card of its
//! first-occurrence spelling (C2), and `--verify-against` compares canonical numberings (I2).

#![allow(missing_docs)]

use assert_cmd::Command;

const KA: &str = "tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";
const KB: &str = "tpubDEYM1BmQ5rp2PWKvCgvQxNeUrEv8gu5819xRdmu6S23fYpS8x2icwAeoVaBTLyN3fGWJQcWoaiKMduTXWKtG9bXNpVrZPRF7XVxrANtAEcR";
const KC: &str = "tpubDEPGZXGPF2v3uNGTWHteRSQnZdEFfMvh6Dcp3DykkBE48hTA6pe3Es6nWH5dAThPyvqmZGmZqQhTw5kxqgR5uJW6Ei2VWyAAN92yLcYBvzV";

fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.code().unwrap(),
    )
}

/// Encode `template`, assert rc 0 and the expected card, then decode it (rc 0).
fn card_row(template: &str, expected: &str, expected_decode: Option<&str>) {
    let (card, err, rc) = run(&["encode", template]);
    assert_eq!(rc, 0, "encode failed: {err}");
    assert_eq!(card.trim(), expected);
    let (dec, derr, drc) = run(&["decode", card.trim()]);
    assert_eq!(drc, 0, "decode failed: {derr}");
    if let Some(t) = expected_decode {
        assert!(dec.lines().any(|l| l == t), "decode stdout: {dec}");
    }
}

#[test]
fn e1_c2_two_slots() {
    card_row(
        "wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))",
        "md1yppq2zcyxppgzcuqqcr8qpnypdll2zu",
        Some("wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))"),
    );
}

#[test]
fn e2_c2_force_chunked() {
    let (a, ea, rca) = run(&[
        "encode",
        "--force-chunked",
        "wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))",
    ]);
    let (b, eb, rcb) = run(&[
        "encode",
        "--force-chunked",
        "wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))",
    ]);
    assert_eq!((rca, rcb), (0, 0), "{ea} {eb}");
    assert_eq!(a, b);
    assert_eq!(
        a.lines().next().unwrap(),
        "md1f5dpaqqpqggzskppsg2qk8qqxw2kucj9lvgv06"
    );
    let chunks: Vec<&str> = a.split_whitespace().collect();
    let mut args = vec!["decode"];
    args.extend(chunks);
    let (_, derr, drc) = run(&args);
    assert_eq!(drc, 0, "decode failed: {derr}");
}

#[test]
fn e3_c2_three_slots() {
    card_row(
        "wsh(multi(2,@1/<2;3>/*,@2/<2;3>/*,@0/<0;1>/*))",
        "md1yzpq2zcyxpzrq9j5qqcsv998hnqcexfa",
        Some("wsh(multi(2,@0/<2;3>/*,@1/<2;3>/*,@2/<0;1>/*))"),
    );
}

#[test]
fn e4_c2_baseline_moves() {
    card_row(
        "wsh(multi(2,@1/<0;1>/*,@0/<2;3>/*))",
        "md1yppqqxppsg2qknq2zc2ktzhwekmddzh",
        Some("wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))"),
    );
}

#[test]
fn e5_c2_keyed() {
    let fp = [
        "--fingerprint",
        "@0=73c5da0a",
        "--fingerprint",
        "@1=73c5da0a",
        "--network",
        "signet",
    ];
    let ka = format!("@0={KA}");
    let kb = format!("@1={KB}");
    let kb0 = format!("@0={KB}");
    let ka1 = format!("@1={KA}");
    let mut a_args = vec![
        "encode",
        "wsh(multi(2,@1/48'/1'/1'/2'/<2;3>/*,@0/48'/1'/0'/2'/<0;1>/*))",
        "--key",
        &ka,
        "--key",
        &kb,
    ];
    a_args.extend(fp);
    let (a, ea, rca) = run(&a_args);
    let mut b_args = vec![
        "encode",
        "wsh(multi(2,@0/48'/1'/1'/2'/<2;3>/*,@1/48'/1'/0'/2'/<0;1>/*))",
        "--key",
        &ka1,
        "--key",
        &kb0,
    ];
    b_args.extend(fp);
    let (b, eb, rcb) = run(&b_args);
    assert_eq!((rca, rcb), (0, 0), "{ea} {eb}");
    assert_eq!(a, b);
    assert_eq!(
        a.lines().next().unwrap(),
        "md1fr6mpzq9q2tvyw89ykcguy4q2zcyxppgzcuqqc9uyw0za5z4eutks2zhqjvuqxvmnuzkpw9lvu"
    );
    let chunks: Vec<&str> = a.split_whitespace().collect();
    let mut dargs = vec!["decode"];
    dargs.extend(chunks.iter().copied());
    let (_, derr, drc) = run(&dargs);
    assert_eq!(drc, 0, "decode failed: {derr}");
    let mut aargs = vec!["address", "--network", "signet"];
    aargs.extend(chunks.iter().copied());
    let (addr, aerr, arc) = run(&aargs);
    assert_eq!(arc, 0, "{aerr}");
    assert_eq!(
        addr.lines().next().unwrap(),
        "tb1q0v66c8vdpjv67fx77penkcx6lealfln60ey7vjte5evmdtc378uq44nq3u"
    );
}

const SPEND_EQUAL: &str = "md: --verify-against: SPEND-EQUAL — same template structure, per-slot key values and per-slot use-site paths. Origin metadata (fingerprints, key origins, path declaration) is excluded from this comparison: it is seating/signing guidance, not script content.";

/// Mint `mint_template`'s card with KA/KB, then verify `verify_template` against it with `k1`
/// as `@1`; return (rc, stderr).
fn verify(mint_template: &str, verify_template: &str, k1: &str) -> (i32, String) {
    let dir = tempfile::tempdir().unwrap();
    let own = dir.path().join("own.md1");
    let own = own.to_str().unwrap();
    let ka = format!("@0={KA}");
    let kb = format!("@1={KB}");
    let (_, e, rc) = run(&[
        "encode",
        mint_template,
        "--key",
        &ka,
        "--key",
        &kb,
        "--network",
        "signet",
        "--out",
        own,
    ]);
    assert_eq!(rc, 0, "fixture encode failed: {e}");
    let k1 = format!("@1={k1}");
    let (_, err, rc) = run(&[
        "descriptor",
        "--template",
        verify_template,
        "--key",
        &ka,
        "--key",
        &k1,
        "--network",
        "signet",
        "--verify-against",
        own,
    ]);
    (rc, err)
}

fn assert_spend_equal(mint: &str, ver: &str) {
    let (rc, err) = verify(mint, ver, KB);
    assert_eq!(rc, 0, "{err}");
    assert!(err.contains(SPEND_EQUAL), "{err}");
}

#[test]
fn e6_i2_multi_noncanonical() {
    let t = "wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))";
    assert_spend_equal(t, t);
}

#[test]
fn e7_i2_sortedmulti_noncanonical() {
    let t = "wsh(sortedmulti(2,@1/<0;1>/*,@0/<0;1>/*))";
    assert_spend_equal(t, t);
}

#[test]
fn e8_i2_control_canonical() {
    let t = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    assert_spend_equal(t, t);
}

#[test]
fn e9_control_canonical() {
    card_row(
        "wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))",
        "md1yppq2zcyxppgzcuqqcr8qpnypdll2zu",
        None,
    );
}

#[test]
fn e10_control_noncanonical_same_use_site() {
    card_row(
        "wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))",
        "md1yppqqxppsg2vlumagltz27le",
        None,
    );
}

#[test]
fn e11_i2_divergent_use_sites() {
    let t = "wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))";
    assert_spend_equal(t, t);
}

#[test]
fn e16_i2_sortedmulti_respelled_same_wallet() {
    assert_spend_equal(
        "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))",
        "wsh(sortedmulti(2,@1/<0;1>/*,@0/<0;1>/*))",
    );
}

#[test]
fn e17_i2_noncanonical_replaced_key() {
    let t = "wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))";
    let (rc, err) = verify(t, t, KC);
    assert_eq!(rc, 5, "{err}");
    assert!(err.contains("NOT spend-equal"), "{err}");
}
