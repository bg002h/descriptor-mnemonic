//! Printed remedies run as printed: each row runs the remedy md prints and expects it to work (md-cli, C1-C7).
use assert_cmd::Command;
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
    assert_eq!(rc, 2, "{se}");
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
    assert_eq!(rc, 1);
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
    assert_eq!(rc, 1);
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
    assert_eq!(rc, 1);
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

/// C6: policies.
fn pk(i: usize) -> String {
    format!("pk(@{i}/<0;1>/*)")
}
fn chain(alts: Vec<String>) -> String {
    let mut a = alts;
    let mut s = a.pop().unwrap();
    while let Some(x) = a.pop() {
        s = format!("or({x},{s})");
    }
    s
}
/// 2-of-3 primary + `d` single-key timelocked recovery paths: `d` nested `or`s.
fn holder(d: usize) -> String {
    let mut alts = vec![format!("thresh(2,{},{},{})", pk(0), pk(1), pk(2))];
    for i in 0..d {
        alts.push(format!("and({},older({}))", pk(i + 3), 1000 * (i + 1)));
    }
    chain(alts)
}
/// and(pk, or(pk, and(pk, or(...)))): `d` nested `or`s that taproot cannot split into leaves.
fn andor(d: usize) -> String {
    let mut s = pk(2 * d);
    for i in (0..d).rev() {
        s = format!("and({},or({},{s}))", pk(2 * i), pk(2 * i + 1));
    }
    s
}
fn compile(argv: &[&str]) -> (i32, String) {
    let mut c = md();
    c.args(argv).timeout(Duration::from_secs(30));
    let o = c.output().expect("spawn");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stderr).into(),
    )
}
/// RED rows: a verdict within 30 s, and when it is a refusal it names the rewrite.
fn c6_verdict(argv: &[&str]) {
    let (rc, se) = compile(argv);
    assert!(
        rc == 0 || (rc == 1 && se.contains("thresh(1,...)")),
        "no verdict within 30 s (rc {rc}): {se}"
    );
}
#[test]
fn c6_compile_holder13_segwitv0_answers() {
    c6_verdict(&["compile", &holder(13), "--context", "segwitv0"]);
}
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
#[test]
fn c6_compile_andor14_tap_answers() {
    c6_verdict(&["compile", &andor(14), "--context", "tap"]);
}
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
}
/// Controls (pass both ways): depth 8 compiles; a taproot or-spine of 14 compiles; the printed rewrite compiles.
#[test]
fn c6_control_depth_eight_compiles() {
    assert_eq!(
        compile(&["compile", &holder(8), "--context", "segwitv0"]).0,
        0
    );
    assert_eq!(compile(&["compile", &andor(8), "--context", "tap"]).0, 0);
}
#[test]
fn c6_control_tap_or_spine_compiles() {
    assert_eq!(compile(&["compile", &holder(14), "--context", "tap"]).0, 0);
}
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

/// C2 (the SLIP-132 testnet spellings): a `Vpub`/`Upub` token also gets `--network` on both printed lines.
#[test]
fn c2_slip132_testnet_prefix_gets_network() {
    let d = "wsh(pk(Vpub5mtyU6Hx9xrx63Y3W4aGW1LuQkmwrq9xsQNgX7tDAM8DTHhE7vXMZ7Hue2FR8SMAGDW57fy76HFmN1jnckSmeX2cDMWVA1KViot6bLgJZuN/<0;1>/*))";
    let (rc, _, se) = out_of(md().args(["encode", d]));
    assert_eq!(rc, 1);
    let printed: Vec<&str> = se
        .lines()
        .filter(|l| l.trim().starts_with("md decompose"))
        .collect();
    assert_eq!(printed.len(), 2, "{se}");
    for l in printed {
        assert!(l.contains("--network <testnet|signet|regtest>"), "{l}");
    }
}

/// C6: an `or` chain nested under a `thresh` is counted too (andor(14) as one of three thresh children).
#[test]
fn c6_compile_or_under_thresh_answers() {
    let p = format!("thresh(2,{},{},{})", pk(29), pk(30), andor(14));
    c6_verdict(&["compile", &p, "--context", "segwitv0"]);
}
