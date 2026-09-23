//! Coordinator-compatibility plan 1b: verdicts, the rule/evidence build, and
//! the conformance gate over EVIDENCE rows.
//!
//! Every test names the mutation that reds it.

use md_codec::coordinator::REGISTRY;

/// Every verified version has exactly one rule set. Mutation: shrink Core's
/// 29.4-31.1 span to 29.4-30.3 -> 31.1 has none and this reds.
#[test]
fn every_verified_version_has_exactly_one_rule_set() {
    for c in REGISTRY {
        for v in c.verified {
            let at = c.position(v).unwrap();
            let n = c
                .rules
                .iter()
                .filter(|r| {
                    let (a, b) = (c.position(r.span.since.0), c.position(r.span.until.0));
                    matches!((a, b), (Some(a), Some(b)) if a <= at && at <= b)
                })
                .count();
            assert_eq!(n, 1, "{} {v}: {n} rule sets", c.name);
        }
    }
}
