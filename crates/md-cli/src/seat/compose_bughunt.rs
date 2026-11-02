//! Bug-hunt triage row #6 (private item `comparison_form`).
use super::*;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tlv::TlvSection;
use md_codec::use_site_path::UseSitePath;
use std::str::FromStr;

/// #6. A card the mint policy refuses (one key in two slots, DuplicateKeySlots)
/// must still get a comparison form: checking an existing card must not depend
/// on today's mint policy.
#[test]
#[ignore = "deferred-minor md-bughunt-06-comparison-form-mint-policy"]
fn bughunt_md_06_comparison_form_independent_of_mint_policy() {
    let xpub = Xpub::from_str("xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf").unwrap();
    let mut raw = [0u8; 65];
    raw[..32].copy_from_slice(xpub.chain_code.as_bytes());
    raw[32..].copy_from_slice(&xpub.public_key.serialize());
    let mut tlv = TlvSection::new_empty();
    tlv.pubkeys = Some(vec![(0, raw), (1, raw)]);
    let d = Descriptor {
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
        tree: Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::Multi,
                body: Body::MultiKeys {
                    k: 2,
                    indices: vec![0, 1],
                },
            }]),
        },
        tlv,
    };
    // Premise: mint refuses, unadmitted serialisation does not.
    assert!(
        d.canonical_payload_bytes().is_err(),
        "premise: mint policy refuses this card"
    );
    assert!(md_codec::encode::encode_payload_unadmitted(&d).is_ok());
    let r = comparison_form(&d);
    assert!(
        r.is_ok(),
        "comparison_form aborted on a card mint refuses: {:?}",
        r.err()
    );
}
