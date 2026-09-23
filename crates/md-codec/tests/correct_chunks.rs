//! F-449 stage 2 Task 2c (SPEC §8.9's stage-2 row): `correct_chunks` is the
//! BCH-correction half of `decode_with_correction`, exposed so `md repair`
//! can keep a correction on a card whose wire version this build cannot
//! decode. It corrects and decodes NOTHING, so an unsupported version is not
//! its concern.
//!
//! FIXTURES, committed so nobody re-derives the recipe (plan Task 2c Step 1).
//! Each was built from a real v4 md1 string by `codex32::unwrap_string`,
//! rewriting the version in the first 5-bit symbol, and `codex32::wrap_payload`
//! -- so the BCH checksum is valid -- then (for the `*_ONE_ERROR` strings)
//! corrupting data position 0.

#![allow(missing_docs)]

use md_codec::{CorrectionDetail, Error, correct_chunks, decode_with_correction};

/// Wire version 12 (outside the accepted set {4, 8}), one correctable error
/// at data position 0.
pub const V12_ONE_ERROR: &str = "md1qzfdsssjjtvyyw2fdssj54qqxppcgscu5e7m9jgawlhg";
/// The same card corrected: BCH-valid, zero errors, still version 12.
pub const V12_CLEAN: &str = "md1uzfdsssjjtvyyw2fdssj54qqxppcgscu5e7m9jgawlhg";
/// The v4 control: same payload at version 4, the same one error.
pub const V4_ONE_ERROR: &str = "md1qzfdsssjjtvyyw2fdssj54qqxppcgsc276kwwfnzntuh";

#[test]
fn correct_chunks_refuses_an_empty_set() {
    assert!(matches!(correct_chunks(&[]), Err(Error::ChunkSetEmpty)));
}

#[test]
fn correct_chunks_keeps_a_correction_whatever_the_wire_version() {
    let (strings, details) = correct_chunks(&[V12_ONE_ERROR]).expect("one error is correctable");
    assert_eq!(strings, vec![V12_CLEAN.to_string()]);
    assert_eq!(
        details,
        vec![CorrectionDetail {
            chunk_index: 0,
            position: 0,
            was: 'q',
            now: 'u'
        }]
    );
    // ...while the decoding entry point still refuses the version, unchanged.
    assert!(matches!(
        decode_with_correction(&[V12_ONE_ERROR]),
        Err(Error::WireVersionMismatch { got: 12 })
    ));
}

#[test]
fn the_clean_v12_fixture_is_bch_valid_and_version_12() {
    let (strings, details) = correct_chunks(&[V12_CLEAN]).expect("clean");
    assert!(details.is_empty(), "{details:?}");
    assert_eq!(strings, vec![V12_CLEAN.to_string()]);
    assert!(matches!(
        md_codec::decode_md1_string(V12_CLEAN),
        Err(Error::WireVersionMismatch { got: 12 })
    ));
}

/// `decode_with_correction` is now `correct_chunks` + decode, and must return
/// exactly what it returned before for a supported card.
#[test]
fn decode_with_correction_is_unchanged_on_a_supported_card() {
    let (_, details) = decode_with_correction(&[V4_ONE_ERROR]).expect("v4 decodes");
    assert_eq!(details.len(), 1);
    assert_eq!(
        (details[0].position, details[0].was, details[0].now),
        (0, 'q', '5')
    );
}

#[test]
fn correct_chunks_is_atomic_on_an_uncorrectable_chunk() {
    // Five substitutions exceed t = 4.
    let mut s: Vec<char> = V4_ONE_ERROR.chars().collect();
    for (i, pos) in [5usize, 9, 14, 20, 27].iter().enumerate() {
        s[*pos] = if s[*pos] == 'q' {
            'p'
        } else {
            ['q', 'z', 'r', 'y', 'x'][i]
        };
    }
    let bad: String = s.into_iter().collect();
    assert!(matches!(
        correct_chunks(&[&bad]),
        Err(Error::TooManyErrors { chunk_index: 0, .. })
    ));
}
