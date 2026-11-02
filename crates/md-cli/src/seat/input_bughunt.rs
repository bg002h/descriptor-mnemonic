//! Bug-hunt triage row #13 (private-path `dedupe_strings`).
use super::*;

/// #13. A hyphen-grouped paste and the same card pasted unbroken name the same
/// card and must dedupe to one string (the decoder strips display separators).
#[test]
#[ignore = "deferred-minor md-bughunt-13-dedupe-display-separators"]
fn bughunt_md_13_dedupe_strings_strips_display_separators() {
    let unbroken = "mk1qqqqqqqqqqqqqqqqqqqqqq".to_string();
    let grouped = "mk1qqqq-qqqq-qqqq-qqqq-qqqqqq".to_string();
    assert_eq!(
        md_codec::encode::strip_display_separators(&grouped),
        unbroken,
        "premise: these are the same card"
    );
    let out = dedupe_strings(&[unbroken, grouped]);
    assert_eq!(
        out.len(),
        1,
        "same card pasted two ways was not deduped: {out:?}"
    );
}
