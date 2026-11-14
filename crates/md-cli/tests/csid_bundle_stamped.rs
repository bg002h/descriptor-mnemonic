//! FOLLOWUPS `md-chunk-set-id-bundle-stamped`: `md descriptor` / `md address`
//! with `--from-mk1` stay silent about the chunk-set id of a card made by
//! `mnemonic bundle` (mk BIP :327, id = first policy_id_stub leading 20 bits
//! XOR cosigner slot 0..=31). Cards: the 2-of-2 signet bundle (cosigners 0, 1).
use std::process::Command;

const POLICY: &str = "md1yp80tcuyqqcy8ppgtcgu79mg9dcdzxlzhdgm3ludvwrqr";
const CARDS: &[&str] = &[
    "mk1qpmfampqqspd57an0td8hvm6w0za5zshqs6c0ncc9wvaf5dxxvd5nulu0wmuqswqfxhp60gu782rszqekjqawuq8w9cmualmr3dawx4g2xx5",
    "mk1qpmfamppu9asynaxppp0uslaga3fzup656vhfg3j96p47d7x4n20yt2tz5h32qumgluzlec8x6tcx52mqg6c",
    "mk1qpmfa6pqqspd57an0td8hvm6hp5gmughqs6c0ncrum7pwruvf52der5dqacn6cr6q75j7myc33vauqn492273ntu57puxundl6n2wydhmvkv",
    "mk1qpmfa6pp2ekqyzxyy66kcdj5qqnzvlehu09qzyn2e9k6rck8qjpk5a48k7xujdvssc06flc7pyw66x3chdks",
];

fn run(verb: &str) -> (i32, String) {
    let mut a = vec![
        verb.to_string(),
        POLICY.to_string(),
        "--from-mk1".to_string(),
    ];
    a.extend(CARDS.iter().map(|s| s.to_string()));
    a.extend(["--network".to_string(), "signet".to_string()]);
    let o = Command::new(env!("CARGO_BIN_EXE_md"))
        .args(&a)
        .output()
        .unwrap();
    (
        o.status.code().unwrap(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn descriptor_bundle_stamped_card_is_silent() {
    let (rc, err) = run("descriptor");
    assert_eq!(rc, 0, "{err}");
    assert!(!err.contains("was not derived from its content"), "{err}");
}

#[test]
fn address_bundle_stamped_card_is_silent() {
    let (rc, err) = run("address");
    assert_eq!(rc, 0, "{err}");
    assert!(!err.contains("was not derived from its content"), "{err}");
}
