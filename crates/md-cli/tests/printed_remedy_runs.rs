//! Printed remedies run as printed: each row runs the remedy md prints and expects it to work (md-cli, C1-C7).
use assert_cmd::Command;
#[cfg(feature = "cli-compiler")]
use std::time::Duration;
fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}
const V_B1_WALLET: &str = include_str!("fixtures/seating/v-b1-wallet.txt");
const V_SPENDEQ_KEYED: &str = include_str!("fixtures/seating/v-spendeq-keyed.txt");
fn lines(text: &str, hrp: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with(hrp))
        .map(str::to_string)
        .collect()
}
fn out_of(c: &mut Command) -> (i32, String, String) {
    let o = c.output().unwrap();
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into(),
        String::from_utf8_lossy(&o.stderr).into(),
    )
}

/// C1: `md descriptor CARD --emit md1` with no key cards; follow the remedy the refusal prints.
fn c1_follow(fixture: &str) {
    let phrases = lines(fixture, "md1");
    let (rc, _, se) = out_of(
        md().arg("descriptor")
            .args(&phrases)
            .args(["--emit", "md1"]),
    );
    assert_eq!(rc, 64, "{se}");
    let mut c = md();
    c.arg("descriptor").args(&phrases);
    if se.contains("Drop --emit md1") {
        // as printed: the same run without the flag
    } else if se.contains("add its mk1 key cards with --from-mk1") {
        c.args(["--emit", "md1"]);
        for s in lines(fixture, "mk1") {
            c.args(["--from-mk1", &s]);
        }
    } else {
        panic!("refusal prints no remedy: {se}");
    }
    let (rc2, so2, se2) = out_of(&mut c);
    assert_eq!(rc2, 0, "the printed remedy is refused: {se2}");
    assert!(!so2.trim().is_empty());
}
#[test]
fn c1_keyless_card_printed_remedy_runs() {
    c1_follow(V_B1_WALLET);
}
#[test]
fn c1_keyed_card_printed_remedy_runs_control() {
    c1_follow(V_SPENDEQ_KEYED);
}

const A0: &str = "[73c5da0a/48'/1'/0'/3']tpubDFH9dgzveyD94P86sEzUzWtd2wxFkUoK78rSBqSWyXNuFq46dy4HbPTEZEP4fbSY4L5Vb2LFnm23JeGQppq5SPcPDNuHZU3JQwMSFXLdudh";
const B0: &str = "[b8688df1/48'/1'/0'/3']tpubDEfobrrtptRTd4Qp6K7RtkNC9GZTdyWPwrXBnPEiu9o5gbcFpscfHbhghiNuBBRuq9RfiN3nwNkLs3E2nRwnFmwKq6NPAbVa6btM8iMjsW6";
const MAINNET: &str = "wsh(sortedmulti(2,[73c5da0a/48'/0'/0']xpub6BmeGmRo4LosAcU21HDaGcvtaQ7GrqQcY48nBkE22qM6KVwQUjRJ1BGzk84SFVHgLcd61Vcnhr8petHexjjn5WbQ9PriVrRhphw4oCp2z6a/<0;1>/*,[b8688df1/48'/0'/0']xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj/<0;1>/*))";

/// C2: `md encode <concrete descriptor>` prints `md decompose <DESCRIPTOR> ...` lines; run each as printed,
/// <DESCRIPTOR> filled with the same descriptor, a `<a|b|c>` placeholder filled with `signet`.
fn c2_run(desc: &str) -> usize {
    let (rc, _, se) = out_of(md().args(["encode", desc]));
    assert_eq!(rc, 2);
    let mut n = 0;
    for l in se.lines() {
        let l = l.trim().split("   #").next().unwrap().trim();
        let Some(rest) = l.strip_prefix("md decompose <DESCRIPTOR>") else {
            continue;
        };
        let mut argv = vec!["decompose".to_string(), desc.to_string()];
        for w in rest.split_whitespace() {
            argv.push(if w.starts_with('<') && w.contains('|') {
                "signet".into()
            } else {
                w.into()
            });
        }
        let (rc, _, e) = out_of(md().args(&argv));
        assert_eq!(rc, 0, "printed `md {}` refused: {e}", argv[2..].join(" "));
        n += 1;
    }
    n
}
#[test]
fn c2_testnet_descriptor_printed_lines_run() {
    assert_eq!(
        c2_run(&format!("wsh(sortedmulti(2,{A0}/<0;1>/*,{B0}/<0;1>/*))")),
        2
    );
}
#[test]
fn c2_mainnet_descriptor_printed_lines_run_control() {
    assert_eq!(c2_run(MAINNET), 2);
    let (_, _, se) = out_of(md().args(["encode", MAINNET]));
    assert!(
        !se.contains("--network"),
        "no --network on a mainnet descriptor: {se}"
    );
}

/// C3: run every branch the compose refusal prints.
#[test]
fn c3_every_printed_branch_runs() {
    let (rc, _, se) = out_of(md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        "2of3",
        "--unspendable",
        "liana",
    ]));
    assert_eq!(rc, 2);
    let mut branches: Vec<Vec<&str>> = vec![];
    if let Some(i) = se.find("Use --wrapper ") {
        let w = se[i + 14..]
            .split(|c: char| !c.is_alphanumeric())
            .next()
            .unwrap();
        branches.push(vec![
            "compose",
            "--wrapper",
            w,
            "--path",
            "2of3",
            "--unspendable",
            "liana",
        ]);
    }
    if se.to_lowercase().contains("omit --unspendable") {
        branches.push(vec!["compose", "--wrapper", "wsh", "--path", "2of3"]);
    }
    assert!(!branches.is_empty(), "no remedy printed: {se}");
    for b in branches {
        let (rc, _, e) = out_of(md().args(&b));
        assert_eq!(
            rc,
            0,
            "printed branch `md {}` refused: {}",
            b.join(" "),
            e.lines().next().unwrap_or("")
        );
    }
}

/// C4: the flag the tr() literal-hex refusal names must be md's.
#[test]
fn c4_printed_flag_runs() {
    let hx = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    let (rc, _, se) = out_of(md().args(["encode", &format!("tr({hx},pk(@0/<0;1>/*))")]));
    assert_eq!(rc, 2);
    let i = se.find("via ").unwrap();
    let flag = se[i + 4..]
        .split(|c: char| c == ')' || c.is_whitespace())
        .next()
        .unwrap()
        .to_string();
    let k0 = A0.split(']').nth(1).unwrap();
    let k1 = B0.split(']').nth(1).unwrap();
    let (rc, _, e) = out_of(md().args([
        "encode",
        "tr(@1/<0;1>/*,pk(@0/<0;1>/*))",
        "--key",
        &format!("@0={k0}"),
        &flag,
        &format!("@1={k1}"),
        "--network",
        "signet",
        "--fingerprint",
        "@0=73c5da0a",
        "--fingerprint",
        "@1=b8688df1",
    ]));
    assert_eq!(rc, 0, "printed flag {flag} refused: {e}");
}

/// C5: 4 same-origin keyless slots, 74 hashlocks: a 64-chunk card at the cap.
fn c5_template(hashes: usize) -> String {
    use bitcoin::hashes::{Hash, sha256};
    let mut s = String::from("multi_a(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*)");
    for j in 0..hashes {
        let h = sha256::Hash::hash(format!("x-{j}").as_bytes()).to_string();
        s = format!("and_v(v:sha256({h}),{s})");
    }
    format!("tr(@3/<0;1>/*,{s})")
}
fn chunks(so: &str) -> usize {
    so.lines().filter(|l| l.starts_with("md1")).count()
}
#[test]
fn c5_at_cap_printed_remedy_is_not_refused() {
    let t = c5_template(74);
    let base = ["encode", t.as_str(), "--path", "bip86", "--group-size", "0"];
    let (rc, so, se) = out_of(md().args(base));
    assert_eq!((rc, chunks(&so)), (0, 64), "{se}");
    if se.contains("Pass one --fingerprint @N=HEX per slot") {
        let mut argv: Vec<String> = base.iter().map(|s| s.to_string()).collect();
        for n in 0..4 {
            argv.push("--fingerprint".into());
            argv.push(format!("@{n}=0000a{n:03x}"));
        }
        let (rc2, _, e2) = out_of(md().args(&argv));
        assert_eq!(
            rc2,
            0,
            "the printed per-slot fingerprint remedy is refused: {}",
            e2.lines().last().unwrap_or("")
        );
    } else {
        assert!(
            se.contains("--seat '@i=<chunk-set-id>'"),
            "the warning prints no remedy: {se}"
        );
    }
}
/// C5 (the cost claim): below the cap the warning states the card's chunk count with fingerprints; follow it and count.
#[test]
fn c5_below_cap_cost_is_exact() {
    let t = c5_template(73);
    let base = ["encode", t.as_str(), "--path", "bip86", "--group-size", "0"];
    let (rc, so, se) = out_of(md().args(base));
    assert_eq!(rc, 0);
    let i = se
        .find("chunk count becomes ")
        .expect("warning states the exact cost");
    let w: Vec<&str> = se[i + 20..].split_whitespace().collect();
    let (then, now): (usize, usize) = (
        w[0].parse().unwrap(),
        w[2].trim_end_matches("),").parse().unwrap(),
    );
    assert_eq!(now, chunks(&so));
    let mut argv: Vec<String> = base.iter().map(|s| s.to_string()).collect();
    for n in 0..4 {
        argv.push("--fingerprint".into());
        argv.push(format!("@{n}=0000a{n:03x}"));
    }
    let (rc2, so2, se2) = out_of(md().args(&argv));
    assert_eq!(rc2, 0, "{se2}");
    assert_eq!(then, chunks(&so2));
    assert_ne!(
        then, now,
        "this template is chosen so the fingerprints add a chunk"
    );
}

#[cfg(feature = "cli-compiler")]
/// C6: policies.
fn pk(i: usize) -> String {
    format!("pk(@{i}/<0;1>/*)")
}
#[cfg(feature = "cli-compiler")]
fn chain(alts: Vec<String>) -> String {
    let mut a = alts;
    let mut s = a.pop().unwrap();
    while let Some(x) = a.pop() {
        s = format!("or({x},{s})");
    }
    s
}
#[cfg(feature = "cli-compiler")]
/// 2-of-3 primary + `d` single-key timelocked recovery paths: `d` nested `or`s.
fn holder(d: usize) -> String {
    let mut alts = vec![format!("thresh(2,{},{},{})", pk(0), pk(1), pk(2))];
    for i in 0..d {
        alts.push(format!("and({},older({}))", pk(i + 3), 1000 * (i + 1)));
    }
    chain(alts)
}
#[cfg(feature = "cli-compiler")]
/// and(pk, or(pk, and(pk, or(...)))): `d` nested `or`s that taproot cannot split into leaves.
fn andor(d: usize) -> String {
    let mut s = pk(2 * d);
    for i in (0..d).rev() {
        s = format!("and({},or({},{s}))", pk(2 * i), pk(2 * i + 1));
    }
    s
}
#[cfg(feature = "cli-compiler")]
fn compile(argv: &[&str]) -> (i32, String) {
    let mut c = md();
    c.args(argv).timeout(Duration::from_secs(30));
    let o = c.output().expect("spawn");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stderr).into(),
    )
}
#[cfg(feature = "cli-compiler")]
/// RED rows: a verdict within 30 s, and when it is a refusal it names the rewrite.
fn c6_verdict(argv: &[&str]) {
    let (rc, se) = compile(argv);
    assert!(
        rc == 0 || (rc == 1 && se.contains("thresh(1,...)")),
        "no verdict within 30 s (rc {rc}): {se}"
    );
}
#[cfg(feature = "cli-compiler")]
#[test]
fn c6_compile_holder13_segwitv0_answers() {
    c6_verdict(&["compile", &holder(13), "--context", "segwitv0"]);
}
#[cfg(feature = "cli-compiler")]
#[test]
fn c6_encode_from_policy_holder13_answers() {
    c6_verdict(&[
        "encode",
        "--from-policy",
        &holder(13),
        "--context",
        "segwitv0",
    ]);
}
#[cfg(feature = "cli-compiler")]
#[test]
fn c6_compile_andor14_tap_answers() {
    c6_verdict(&["compile", &andor(14), "--context", "tap"]);
}
#[cfg(feature = "cli-compiler")]
/// New behaviour: the bound is exactly 8 (9 refused at once, exit 1), in both contexts.
#[test]
fn c6_bound_nine_refused() {
    let (rc, se) = compile(&[
        "compile",
        &chain((0..10).map(pk).collect()),
        "--context",
        "segwitv0",
    ]);
    assert_eq!(rc, 1, "{se}");
    assert!(
        se.contains("nests 9 `or`s") && se.contains("thresh(1,...)"),
        "{se}"
    );
    let (rc, se) = compile(&["compile", &andor(9), "--context", "tap"]);
    assert_eq!(rc, 1, "{se}");
    assert!(
        se.contains("a taproot leaf of the policy nests 9 `or`s"),
        "{se}"
    );
    assert!(
        se.contains("cannot join them without repeating a key"),
        "{se}"
    );
}
#[cfg(feature = "cli-compiler")]
/// Controls (pass both ways): depth 8 compiles; a taproot or-spine of 14 compiles; the printed rewrite compiles.
#[test]
fn c6_control_depth_eight_compiles() {
    assert_eq!(
        compile(&["compile", &holder(8), "--context", "segwitv0"]).0,
        0
    );
    assert_eq!(compile(&["compile", &andor(8), "--context", "tap"]).0, 0);
}
#[cfg(feature = "cli-compiler")]
#[test]
fn c6_control_tap_or_spine_compiles() {
    assert_eq!(compile(&["compile", &holder(14), "--context", "tap"]).0, 0);
}
#[cfg(feature = "cli-compiler")]
#[test]
fn c6_control_thresh1_rewrite_compiles() {
    let mut alts = vec![format!("thresh(2,{},{},{})", pk(0), pk(1), pk(2))];
    for i in 0..13 {
        alts.push(format!("and({},older({}))", pk(i + 3), 1000 * (i + 1)));
    }
    let p = format!("thresh(1,{})", alts.join(","));
    let (rc, se) = compile(&["compile", &p, "--context", "segwitv0"]);
    assert_eq!(rc, 0, "{se}");
}

/// BIP-32 test vector 1, chain m: `xprv` and its public key `xpub`, whose fingerprint is `3442193e` (the
/// parent fingerprint of chain m/0H).
const V1_XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
const V1_XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";

/// The same extended key under another version (SLIP-132 and BIP-32 differ only there).
fn reversion(k: &str, version: [u8; 4]) -> String {
    let mut b = bitcoin::base58::decode_check(k).unwrap();
    b[..4].copy_from_slice(&version);
    bitcoin::base58::encode_check(&b)
}

/// C2, every other key spelling the referral matches (SLIP-132 testnet `Vpub`, SLIP-132 mainnet `zpub` from
/// the BIP-84 vector, a private `xprv`, an xpub beside a tpub): decompose refuses each under every flag, so
/// no `md decompose` line is printed, the text names the spelling, and the remedy it names runs (the same
/// descriptor with xpub or tpub keys prints two lines, and each runs; one line, the template, where a key is
/// the master key with no origin path, as in the xprv and xpub-beside-tpub rows).
#[test]
fn c2_other_key_spellings_printed_lines_run() {
    let (a0_origin, a0_tpub) = A0.split_once(']').unwrap();
    let a0_vpub = reversion(a0_tpub, [0x02, 0x57, 0x54, 0x83]);
    let zpub = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
    let zpub_as_xpub = reversion(zpub, [0x04, 0x88, 0xb2, 0x1e]);
    let cases = [
        (
            format!("wsh(pk({a0_origin}]{a0_vpub}/<0;1>/*))"),
            "Vpub",
            format!("wsh(pk({A0}/<0;1>/*))"),
            2,
        ),
        (
            format!("wpkh([73c5da0a/84'/0'/0']{zpub}/<0;1>/*)"),
            "zpub",
            format!("wpkh([73c5da0a/84'/0'/0']{zpub_as_xpub}/<0;1>/*)"),
            2,
        ),
        (
            format!("wpkh({V1_XPRV}/<0;1>/*)"),
            "xprv",
            format!("wpkh([3442193e]{V1_XPUB}/<0;1>/*)"),
            1,
        ),
        (
            format!("wsh(multi(1,[3442193e]{V1_XPUB}/<0;1>/*,{A0}/<0;1>/*))"),
            "xpub, tpub",
            format!(
                "wsh(multi(1,[3442193e]{V1_XPUB}/<0;1>/*,[73c5da0a/84'/0'/0']{zpub_as_xpub}/<0;1>/*))"
            ),
            1,
        ),
    ];
    for (d, has, again, lines) in cases {
        assert_eq!(c2_run(&d), 0, "a decompose line printed for {d}");
        let (_, _, se) = out_of(md().args(["encode", &d]));
        assert!(
            se.contains(&format!("this one has {has}.")) && se.contains("`md decompose`"),
            "{se}"
        );
        assert_eq!(c2_run(&again), lines, "{again}");
    }
}

/// C2, a key with no `[fingerprint/path]` origin (xpub, tpub beside an originated one, and a master xpub
/// with its fingerprint but no path, which decompose's two mint routes both refuse): decompose's
/// `--emit commands` refuses it, so only the `--emit template` line is printed and it runs; the text says
/// why, and with the origin path stated both lines are printed and run.
#[test]
fn c2_origin_less_key_printed_line_runs() {
    let bare_tpub = A0.split_once(']').unwrap().1;
    for d in [
        format!("wpkh({V1_XPUB}/<0;1>/*)"),
        format!("wsh(sortedmulti(2,{bare_tpub}/<0;1>/*,{B0}/<0;1>/*))"),
        format!("wpkh([3442193e]{V1_XPUB}/<0;1>/*)"),
    ] {
        assert_eq!(c2_run(&d), 1, "{d}");
        let (_, _, se) = out_of(md().args(["encode", &d]));
        assert!(
            se.contains(
                "Its mint commands (--emit commands) also need each key at account level with its origin, [fingerprint/path] before the key; this descriptor has a key without one."
            ),
            "{se}"
        );
        assert!(!se.contains("knows the path"), "{se}");
    }
    assert_eq!(c2_run(&format!("wsh(pk({A0}/<0;1>/*))")), 2);
}

/// Signet keys from `bh1119_md_decompose_routes.rs`: `KA` at depth 4 under `[73c5da0a/48'/1'/0'/2']`, `K1` at
/// depth 1 under `[73c5da0a/84']`, `K2` at depth 2 under `[73c5da0a/84'/1']`.
const KA: &str = "tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";
const K1: &str = "tpubD97UxEEVXiRvic42NcHLjLAWv5L3CEkuj587dZtEmb4T4Da74DpjbC5tjm7qBq3G4LxjU9SYuTv7edTvTefctXWPkvgSn7MHUmiPCsSyzyJ";
const K2: &str = "tpubDB5GSFrKWykZsJgwdd9ViGUECU8AQvTxig4iZonrS5BJ2k9V4ucfHifnXTNfRExUaDsLvZedPfbEN2iphbq11CMDe4WJQAvQRqXaGJuwhwk";
const C2_HEAD: &str = "md: template parse error: this is a concrete wallet descriptor (it carries a real extended key), not an md1 template — `md encode` takes a template whose keys are `@i` placeholders. md reads descriptors with `md decompose`";
const C2_NO_PATH: &str = "Its mint commands (--emit commands) also need each key at account level with its origin, [fingerprint/path] before the key; this descriptor has a key without one.";
const TPUB_NET: &str = " --network <testnet|signet|regtest>";
fn c2_stderr(desc: &str) -> String {
    out_of(md().args(["encode", desc])).2
}
fn c2_both_lines(net: &str) -> String {
    format!(
        "{C2_HEAD}:\n    md decompose <DESCRIPTOR> --emit commands{net}   # the mint commands, ready to run\n    md decompose <DESCRIPTOR> --emit template{net}   # or just the @i template for --template\n"
    )
}
fn c2_fixed_refusal() -> String {
    format!(
        "{C2_HEAD}, which prints no template for this one: this descriptor has no md1 template: key @0 has use-site `/0/*`, which an md1 card cannot hold (after the origin it holds only a wildcard, optionally after one multipath step such as `/<0;1>/*`), so a template, and any card minted from it, would describe a different wallet. If this is the receive (`/0/*`) or change (`/1/*`) half of a wallet, give the descriptor with `/<0;1>/*` in its place. The emissions that need no template still work: `--emit keys`, `--emit fingerprints`, `--emit descriptor`."
    )
}

/// C2 (review md60 I1.1): a key with a fixed use-site `/0/*` has no md1 template, so `--emit template` and
/// `--emit commands` both refuse it: no `md decompose` line is printed, the text is decompose's own refusal
/// (it names `/<0;1>/*`), plus the no-path sentence for an origin-less key; with `/<0;1>/*` in its place the
/// lines are printed and run.
#[test]
fn c2_fixed_use_site_prints_no_decompose_line() {
    let d = format!("wpkh([73c5da0a/48'/1'/0'/2']{KA}/0/*)");
    assert_eq!(c2_run(&d), 0, "{d}");
    assert_eq!(c2_stderr(&d), format!("{}\n", c2_fixed_refusal()));
    assert_eq!(
        c2_run(&format!("wpkh([73c5da0a/48'/1'/0'/2']{KA}/<0;1>/*)")),
        2
    );

    let d = format!("wpkh({KA}/0/*)");
    assert_eq!(c2_run(&d), 0, "{d}");
    assert_eq!(
        c2_stderr(&d),
        format!("{}\n{C2_NO_PATH}\n", c2_fixed_refusal())
    );
    assert_eq!(c2_run(&format!("wpkh({KA}/<0;1>/*)")), 1);
}

/// C2 (review md60 I1.2): a depth-1 key whose origin fingerprint is not its parent's: decompose proves no
/// mint route, so only the `--emit template` line is printed (it runs), and the text is decompose's refusal.
#[test]
fn c2_no_mint_route_prints_template_line_only() {
    let d = format!("wpkh([deadbeef/84']{K1}/<0;1>/*)");
    assert_eq!(c2_run(&d), 1, "{d}");
    assert_eq!(
        c2_stderr(&d),
        format!(
            "{C2_HEAD}:\n    md decompose <DESCRIPTOR> --emit template{TPUB_NET}   # the @i template for --template\nIts mint commands are not printed: --emit commands cannot be produced: no mint route reproduces this descriptor. route 1: md encode refuses it: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally uses 3), got 1. route 2: mk encode refuses it: origin fingerprint deadbeef does not match the xpub: at depth 1 the xpub's parent is the master key, so the fingerprint must be 73c5da0a. The record pairs a key with an origin that is not its own.. The emissions that mint nothing still work: `--emit template`, `--emit keys`, `--emit descriptor`.\n"
        )
    );
}

/// C2 (review md60 I1.3): a depth-0 key under a 3-step origin path: decompose refuses the descriptor, so no
/// `md decompose` line is printed and the text is decompose's refusal; the same key with its true origin
/// (fingerprint, no path) prints the template line, which runs.
#[test]
fn c2_depth_inconsistent_prints_no_decompose_line() {
    let d = format!("wpkh([3442193e/84'/0'/0']{V1_XPUB}/<0;1>/*)");
    assert_eq!(c2_run(&d), 0, "{d}");
    assert_eq!(
        c2_stderr(&d),
        format!(
            "{C2_HEAD}, which refuses this one: key @0 is depth-inconsistent IN THE INPUT: the extended key states depth 0, but its origin path `[3442193e/84'/0'/0']` has 3 component(s) — depth 3. An mk1 key card drops depth and child number from the wire and reconstructs BOTH from the origin path, so `mk encode --keys` refuses such a record outright (\"xpub origin-path mismatch: xpub depth 0 … vs origin_path depth 3 …\"). decompose will not emit a key line that could not be minted: correct the origin in the descriptor so it states the path this key was actually derived at.\n"
        )
    );
    assert_eq!(c2_run(&format!("wpkh([3442193e]{V1_XPUB}/<0;1>/*)")), 1);
}

/// C2 (review md60 M2): a depth-1 and a depth-2 key with their true origins: route 1 is withheld and route 2
/// proven, so `--emit commands` runs; both lines are printed and run.
#[test]
fn c2_depth_1_2_key_printed_lines_run() {
    for d in [
        format!("wpkh([73c5da0a/84']{K1}/<0;1>/*)"),
        format!("wpkh([73c5da0a/84'/1']{K2}/<0;1>/*)"),
    ] {
        assert_eq!(c2_run(&d), 2, "{d}");
        assert_eq!(c2_stderr(&d), c2_both_lines(TPUB_NET));
        let (rc, so, se) =
            out_of(md().args(["decompose", &d, "--network", "signet", "--emit", "commands"]));
        assert_eq!(rc, 0, "{se}");
        assert!(so.starts_with("# ── route 1 withheld: "), "{so}");
        assert!(
            so.contains(
                "# ── route 2: the SPLIT set — a keyless policy card + one mk1 card per key ──"
            ),
            "{so}"
        );
    }
}

#[cfg(feature = "cli-compiler")]
/// C6: an `or` chain nested under a `thresh` is counted too (andor(14) as one of three thresh children).
#[test]
fn c6_compile_or_under_thresh_answers() {
    let p = format!("thresh(2,{},{},{})", pk(29), pk(30), andor(14));
    c6_verdict(&["compile", &p, "--context", "segwitv0"]);
}

#[cfg(feature = "cli-compiler")]
/// C6 control: side-by-side `or` chains are not bounded, only nesting (three 8-deep key chains under one
/// `thresh` compile, in both contexts).
#[test]
fn c6_control_width_not_bounded() {
    let p = format!(
        "thresh(2,{},{},{})",
        chain((0..9).map(pk).collect()),
        chain((9..18).map(pk).collect()),
        chain((18..27).map(pk).collect())
    );
    for ctx in ["segwitv0", "tap"] {
        let (rc, se) = compile(&["compile", &p, "--context", ctx]);
        assert_eq!(rc, 0, "{ctx}: {se}");
    }
}
/// C5 at the single-string edge: the card fits one md1 string now and needs chunks with fingerprints; the
/// stated counts are what `md encode` prints.
#[test]
fn c5_single_string_card_cost_is_exact() {
    let t = c5_template(1);
    let base = ["encode", t.as_str(), "--path", "bip86", "--group-size", "0"];
    let (rc, so, se) = out_of(md().args(base));
    assert_eq!(rc, 0);
    let i = se
        .find("chunk count becomes ")
        .expect("warning states the exact cost");
    let w: Vec<&str> = se[i + 20..].split_whitespace().collect();
    let (then, now): (usize, usize) = (
        w[0].parse().unwrap(),
        w[2].trim_end_matches("),").parse().unwrap(),
    );
    assert_eq!(now, chunks(&so), "{se}");
    let mut argv: Vec<String> = base.iter().map(|s| s.to_string()).collect();
    for n in 0..4 {
        argv.push("--fingerprint".into());
        argv.push(format!("@{n}=0000a{n:03x}"));
    }
    let (rc2, so2, se2) = out_of(md().args(&argv));
    assert_eq!(rc2, 0, "{se2}");
    assert_eq!(then, chunks(&so2), "{se}");
}
#[cfg(feature = "cli-compiler")]
/// C6 on tap: a root `thresh(1,...)` is split into leaves as compile_tr splits it, a root `thresh(2,...)` is not.
#[test]
fn c6_tap_root_thresh_split_matches_compile_tr() {
    let one = format!("thresh(1,{},{})", pk(20), chain((0..10).map(pk).collect()));
    let (rc, se) = compile(&["compile", &one, "--context", "tap"]);
    assert_eq!(rc, 0, "{se}");
    let two = format!(
        "thresh(2,{},{},{})",
        pk(20),
        pk(21),
        chain((0..10).map(pk).collect())
    );
    let (rc, se) = compile(&["compile", &two, "--context", "tap"]);
    assert_eq!(rc, 1, "{se}");
    assert!(
        se.contains("a taproot leaf of the policy nests 9 `or`s"),
        "{se}"
    );
}
