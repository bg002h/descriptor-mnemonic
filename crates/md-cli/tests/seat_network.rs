//! C8b rows: seating (`md address` / `md descriptor` with `--from-mk1`,
//! `--from-mk1-file`, `--emit md1`) must refuse an mk1 card whose xpub network
//! kind differs from `--network`. Every row runs the public CLI binary.
//! Cards: one keyless md1 policy card, two mk1 key cards minted from tpubs
//! (TEST), and the same two keys minted as mainnet xpubs (MAIN).
use std::process::Command;

const POLICY: &str = "md1ypfds3cfwqqvzrss59uyw0za5zkux3r034tgl3mtwh2rwk";
const TEST: &[&str] = &[
    "mk1qpw6lfzqqsq6458qupeutks2lcztpqyqsqygrqyqsqygpqyqsqyg8qyqsqyqgdv8e7avzjpe5atps4cfpcj6s2kca4q7gqqtqkhwt8yyclw5",
    "mk1qpw6lfzplesunlrnjptll5nqh65de99r8uezm0qrnv8x7mq3tu7rcgtc58kcfymtdkgfdkfztzyakqdlvrkhdmcg3gchvfk6vl67cxca96p3",
    "mk1qpw6lfzzh62zyyks8vy89nm5y6",
    "mk1qp7m9zzqqsq6458quzux3r03lcztpqyqsqygrqyqsqygpqyqsqyg8qyqsqyqgdv8eanas4md2c7u94j8ljjlpjz9mta0sacl8f8960vjj9qs",
    "mk1qp7m9zzp7cf8yshs9snlhuctudr5y66qhkuf27granw7r5vp9plqehm88gr537y0t6xx44ksmgnrccnuf49rvsvyfpmlvutzvfyg6q4w3xp9",
    "mk1qp7m9zzzegkhj7xxfljcfccg4u",
];
const MAIN: &[&str] = &[
    "mk1qpvxz5zqqsq6458qupeutks2lcztpqyqsqygrqyqsqygpqyqsqyg8qyqsqyqfz9jr6avzjpe5atps4cfpcj6s2kca4q7g74httk7yjq5jx9q",
    "mk1qpvxz5zplesunlrnjptll5nqh65de99r8uezm0qrnv8x7mq3tu7rcgtc58kcfymtdkgfdkfztzyakqdlvrkhdmcg3gckz48tht364ezmewxa",
    "mk1qpvxz5zzs49dwuzu0k30lvxtnz",
    "mk1qp9ydqzqqsq6458quzux3r03lcztpqyqsqygrqyqsqygpqyqsqyg8qyqsqyqfz9jrenas4md2c7u94j8ljjlpjz9mta0sej5fxwzd0vzywrp",
    "mk1qp9ydqzp7cf8yshs9snlhuctudr5y66qhkuf27granw7r5vp9plqehm88gr537y0t6xx44ksmgnrccnuf49rvsvyfpmjjjcutcflff0f3ky9",
    "mk1qp9ydqzzrgp3vszgqlatwzzzmc",
];
const KEYED_MD1: &[&str] = &[
    "md1fl2mxpspq2tvywz2sqrqsuy9p0prnchdq4hp5gmug4cyjn4vxzhpy8zt2p2mrk5re87v8y7eq5u75su40ql",
    "md1fl2mxpstlrnjptll5nqh65de99r8uezm0qrnv8x7mq3tu7rcgtc58kcfymtdkgfdkfztzywjhph02drc0u7",
    "md1fl2mxpsnkcphasw6ah0pz9rr2c7u94j8ljjlpjz9mta03asjwfp0qtp8l0eshc68gf45pwkpj5f5w3za8n5",
    "md1fl2mxpsahz2hjqlvmhsarqfg0cxd7ee6qay03r67334dd5x6yc7xylzdfgmyrpzgwuq3gdq22spkga0g",
];

#[derive(Clone, Copy, Debug)]
enum Verb {
    AddressCards,
    AddressFile,
    DescriptorCards,
    DescriptorFile,
    DescriptorEmitMd1,
}
const VERBS: [Verb; 5] = [
    Verb::AddressCards,
    Verb::AddressFile,
    Verb::DescriptorCards,
    Verb::DescriptorFile,
    Verb::DescriptorEmitMd1,
];

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn md(args: &[String]) -> Out {
    let o = Command::new(env!("CARGO_BIN_EXE_md"))
        .args(args)
        .output()
        .unwrap();
    Out {
        code: o.status.code().unwrap(),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn run(verb: Verb, cards: &[&str], net: Option<&str>, tag: &str) -> Out {
    let mut a: Vec<String> = Vec::new();
    let file = std::env::temp_dir().join(format!("c8b-{}-{tag}.txt", std::process::id()));
    match verb {
        Verb::AddressCards => {
            a.extend(["address".into(), POLICY.into(), "--from-mk1".into()]);
            a.extend(cards.iter().map(|s| s.to_string()));
        }
        Verb::DescriptorCards | Verb::DescriptorEmitMd1 => {
            a.extend(["descriptor".into(), POLICY.into(), "--from-mk1".into()]);
            a.extend(cards.iter().map(|s| s.to_string()));
            if matches!(verb, Verb::DescriptorEmitMd1) {
                a.extend(["--emit".into(), "md1".into()]);
            }
        }
        Verb::AddressFile | Verb::DescriptorFile => {
            std::fs::write(&file, cards.join("\n")).unwrap();
            let v = if matches!(verb, Verb::AddressFile) {
                "address"
            } else {
                "descriptor"
            };
            a.extend([
                v.into(),
                POLICY.into(),
                "--from-mk1-file".into(),
                file.display().to_string(),
            ]);
        }
    }
    if let Some(n) = net {
        a.extend(["--network".into(), n.into()]);
    }
    let o = md(&a);
    let _ = std::fs::remove_file(&file);
    o
}

fn refused(o: &Out, what: &str, key_kind: &str, net: &str, remedy: &str) {
    assert_eq!(
        o.code, 1,
        "{what}: want exit 1, got {} / stdout {:?}",
        o.code, o.stdout
    );
    assert!(
        o.stdout.is_empty(),
        "{what}: stdout must be empty, got {:?}",
        o.stdout
    );
    let want =
        format!("its xpub is a {key_kind} key, but --network says {net}. Re-run with {remedy}");
    assert!(
        o.stderr.contains(&want),
        "{what}: stderr {:?} lacks {want:?}",
        o.stderr
    );
    assert!(
        o.stderr.contains("--from-mk1 card "),
        "{what}: {:?}",
        o.stderr
    );
}

const TESTNET_REMEDY: &str = "`--network testnet`, `--network signet` or `--network regtest`.";
const MAINNET_REMEDY: &str = "`--network mainnet`.";

fn t_no_network(v: Verb) {
    let o = run(v, TEST, None, "t-none");
    refused(
        &o,
        &format!("{v:?} no --network"),
        "testnet",
        "mainnet",
        TESTNET_REMEDY,
    );
}

fn t_network_mainnet(v: Verb) {
    let o = run(v, TEST, Some("mainnet"), "t-main");
    refused(
        &o,
        &format!("{v:?} --network mainnet"),
        "testnet",
        "mainnet",
        TESTNET_REMEDY,
    );
}

fn m_network(v: Verb, net: &str) {
    let o = run(v, MAIN, Some(net), "m-test");
    refused(
        &o,
        &format!("{v:?} --network {net}"),
        "mainnet",
        net,
        MAINNET_REMEDY,
    );
}

/// One row per verb spelling and per case, so each wrong case of the survey
/// is its own row.
macro_rules! per_verb {
    ($($name:ident => $verb:expr),* $(,)?) => {
        mod no_network { use super::*; $(#[test] fn $name() { t_no_network($verb); })* }
        mod testnet_cards_network_mainnet { use super::*; $(#[test] fn $name() { t_network_mainnet($verb); })* }
        mod mainnet_cards_network_testnet { use super::*; $(#[test] fn $name() { m_network($verb, "testnet"); })* }
        mod mainnet_cards_network_signet { use super::*; $(#[test] fn $name() { m_network($verb, "signet"); })* }
        mod mainnet_cards_network_regtest { use super::*; $(#[test] fn $name() { m_network($verb, "regtest"); })* }
    };
}
per_verb! {
    address_from_mk1 => Verb::AddressCards,
    address_from_mk1_file => Verb::AddressFile,
    descriptor_from_mk1 => Verb::DescriptorCards,
    descriptor_from_mk1_file => Verb::DescriptorFile,
    descriptor_emit_md1 => Verb::DescriptorEmitMd1,
}

/// The printed remedy runs: a refused testnet set seats under each network the
/// message names, and a refused mainnet set seats under `--network mainnet`.
#[test]
fn the_printed_remedies_seat() {
    for v in VERBS {
        for net in ["testnet", "signet", "regtest"] {
            let o = run(v, TEST, Some(net), "r-t");
            assert_eq!(
                o.code, 0,
                "{v:?} testnet cards --network {net}: {}",
                o.stderr
            );
        }
        let o = run(v, MAIN, Some("mainnet"), "r-m");
        assert_eq!(
            o.code, 0,
            "{v:?} mainnet cards --network mainnet: {}",
            o.stderr
        );
    }
}

/// Control (passes both ways): matching kinds, and the mainnet default with
/// mainnet cards.
#[test]
fn control_matching_network_kinds_seat() {
    for v in VERBS {
        let o = run(v, MAIN, None, "c-m-none");
        assert_eq!(o.code, 0, "{v:?} mainnet cards, no --network: {}", o.stderr);
        for net in ["testnet", "signet", "regtest"] {
            let o = run(v, TEST, Some(net), "c-t");
            assert_eq!(o.code, 0, "{v:?}: {}", o.stderr);
        }
    }
}

/// Control (passes both ways): a keyed md1 card (no mk1 cards) takes no
/// network check (C8 ruled it not a defect).
#[test]
fn control_a_keyed_card_with_no_network_flag_still_renders() {
    let mut a = vec!["address".to_string()];
    a.extend(KEYED_MD1.iter().map(|s| s.to_string()));
    let o = md(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stdout.starts_with("bc1"), "{}", o.stdout);
}

/// Control (the network surface the refusal must not disturb): each network
/// still renders its own address prefix for the matching card set.
#[test]
fn control_address_prefix_follows_the_network() {
    for (net, pfx) in [("testnet", "tb1"), ("signet", "tb1"), ("regtest", "bcrt1")] {
        let o = run(Verb::AddressCards, TEST, Some(net), "p");
        assert!(o.stdout.starts_with(pfx), "{net}: {}", o.stdout);
    }
    let o = run(Verb::AddressCards, MAIN, None, "p");
    assert!(o.stdout.starts_with("bc1"), "{}", o.stdout);
}

/// Mixed set (one testnet card, one mainnet card): refused under either
/// network, naming the card whose kind differs.
#[test]
fn mixed_kind_set_is_refused_under_either_network() {
    let mixed = [TEST[0], TEST[1], TEST[2], MAIN[3], MAIN[4], MAIN[5]];
    let o = run(Verb::AddressCards, &mixed, Some("mainnet"), "mix1");
    assert_eq!(o.code, 1, "{}", o.stdout);
    assert!(
        o.stderr.contains("its xpub is a testnet key"),
        "{}",
        o.stderr
    );
    let o = run(Verb::AddressCards, &mixed, Some("signet"), "mix2");
    assert_eq!(o.code, 1, "{}", o.stdout);
    assert!(
        o.stderr.contains("its xpub is a mainnet key"),
        "{}",
        o.stderr
    );
}

/// The ruled texts, whole: `<id>` is the card's label (chunk-set id and
/// stub, the id `--seat` takes), the file spelling prints the same text, and
/// a mixed set names its first card (ascending chunk-set id) of the other
/// kind. stderr is that one line and nothing else.
#[test]
fn the_refusal_lines_are_the_ruled_texts() {
    const T_LINE: &str = "md: seating refused: --from-mk1 card 76be9 (stub aad0e0e0): \
its xpub is a testnet key, but --network says mainnet. \
Re-run with `--network testnet`, `--network signet` or `--network regtest`.\n";
    const M_LINE: &str = "md: seating refused: --from-mk1 card 291a0 (stub aad0e0e0): \
its xpub is a mainnet key, but --network says regtest. Re-run with `--network mainnet`.\n";
    for v in [Verb::AddressCards, Verb::AddressFile] {
        assert_eq!(run(v, TEST, None, "x-t").stderr, T_LINE, "{v:?}");
    }
    assert_eq!(
        run(Verb::DescriptorFile, MAIN, Some("regtest"), "x-m").stderr,
        M_LINE
    );
    let mixed = [MAIN[3], MAIN[4], MAIN[5], TEST[0], TEST[1], TEST[2]];
    assert_eq!(
        run(Verb::AddressCards, &mixed, Some("mainnet"), "x-mix").stderr,
        T_LINE
    );
    assert_eq!(
        run(Verb::AddressCards, &mixed, Some("regtest"), "x-mix2").stderr,
        M_LINE
    );
}
