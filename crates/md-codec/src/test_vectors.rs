//! Canonical `md` test-vector corpus.
//!
//! Used by `md-codec`'s own integration tests, by `md-cli`'s `vectors`
//! subcommand, and by `md-cli`'s `tests/json_snapshots.rs` /
//! `tests/template_roundtrip.rs`. Single source of truth: any vector
//! addition / removal / rename happens here.
//!
//! `Vector` is `#[non_exhaustive]` so future fields can be added without a
//! breaking-change bump: external consumers construct nothing — they only
//! read `MANIFEST` entries.

/// One entry of the canonical test-vector corpus.
#[non_exhaustive]
pub struct Vector {
    /// Vector identifier — used in test failure messages and as a stable
    /// handle for cross-suite filtering. Convention: snake_case mirroring
    /// the wallet-policy template's distinguishing structure.
    pub name: &'static str,
    /// BIP-388 wallet-policy template string the vector encodes. Parsed
    /// by `parse::template`; round-tripped through `encode` and `decode`.
    pub template: &'static str,
    /// `(@N, xpub)` pairs binding each `@N` placeholder in `template`. Empty
    /// when the vector exercises template-only paths (no key binding).
    pub keys: &'static [(u8, &'static str)],
    /// `(@N, 4-byte master fingerprint)` pairs aligned with `keys`. Empty
    /// when the vector does not exercise fingerprint round-tripping.
    pub fingerprints: &'static [(u8, [u8; 4])],
    /// When true, force the encoder onto the chunked wire path even if the
    /// payload would fit in a single chunk. Exercises chunk-boundary logic
    /// without padding the template artificially.
    pub force_chunked: bool,
    /// Explicit shared origin path applied via the encoder's `--path`
    /// override (`m/...` literal, or a named `bip44|48|49|84|86` form).
    /// `None` = elided origin — the encoder infers the canonical origin via
    /// `canonical_origin`. `Some(..)` is REQUIRED for non-canonical shapes
    /// (`tr()` + TapTree, NUMS-taproot) whose `canonical_origin` returns
    /// `None`: without an explicit origin they mint a card the decoder
    /// rejects with `MissingExplicitOrigin`. Carried into the emitted
    /// `.descriptor.json` via `path_decl` (the `.template` file alone does
    /// not determine it), so the BIP §Test Vectors table pins the
    /// template+path pair for path-carrying rows.
    pub path: Option<&'static str>,
}

/// The canonical 15-entry corpus.
///
/// Part-3 additions (BIP-alignment cycle): `sh_wpkh`, `tr_with_leaf`,
/// `nums_taproot`, `wsh_sortedmulti_2chunk`, and `single_string_boundary`.
///
/// * `sh_wpkh` — un-omitted: since F-A1, `sh(wpkh)` round-trips symmetrically
///   in ELIDED form (`canonical_origin(sh(wpkh))` = `m/49'/0'/0'`), so it is a
///   corpus ADDITION (`path: None`), not an asymmetric omission.
/// * `tr_with_leaf` / `nums_taproot` — non-canonical `tr()` shapes
///   (`canonical_origin` = `None`); expressible now via the `path` field,
///   which supplies the explicit origin the decoder requires.
/// * `wsh_sortedmulti_2chunk` — a genuine 2-member chunk set: a 2-of-8
///   sortedmulti with a master fingerprint on every cosigner makes a 376-bit
///   payload that still FITS a single string (below the 400-bit / 80-symbol
///   regular-code cap), so it is `force_chunked` to route it through `split`,
///   whose 320-bit per-chunk budget then yields two chunks. (Contrast
///   `wsh_multi_chunked`, also `force_chunked` but a chunk-set-of-one.)
/// * `single_string_boundary` (F-V2) — a 2-of-9 sortedmulti with fingerprints
///   on 8 of the 9 cosigners, sized so its single-string regular-code emit
///   lands at 79 of the 80 max data symbols (95 chars total = `md1` plus 79
///   data plus 13 checksum). Proves the regular-only single-string boundary
///   holds right at the codex32 BCH(93,80) cap — NOT chunked, NOT long-code.
#[rustfmt::skip]
pub const MANIFEST: &[Vector] = &[
    // ── KEYED CONFORMANCE VECTORS (R3, 2026-08-20) ─────────────────────────────
    //
    // Every entry above is KEYLESS, and `Vector::keys` was read by no code at all
    // -- `cmd/vectors.rs` passed `&[]` unconditionally -- so the corpus pinned
    // template bytes and nothing else. A Go port could agree with Rust about every
    // byte on the wire and still derive a different ADDRESS, and no vector would
    // say so. These entries close that: each one carries real xpubs, so the export
    // can emit descriptor strings, both wallet ids and per-chain addresses.
    //
    // THE KEYS ARE PUBLIC BY CONSTRUCTION and reproducible in one command. They are
    // BIP-39's own published test mnemonic --
    // "abandon abandon ... about" -- at `bip48-p2wsh` accounts 0..3:
    //
    // printf 'abandon abandon abandon abandon abandon abandon abandon \
    // abandon abandon abandon abandon about' \
    // | ms derive --phrase - --template bip48-p2wsh --account N
    //
    // Master fingerprint 73c5da0a. NEVER put funds behind them.
    //
    // ALL KEYED ENTRIES ARE `force_chunked`, and not as a style choice: real
    // xpubs push the payload past the codex32 regular code's 80-data-symbol cap
    // for a single string. A keyed 2-of-3 measures 474 symbols. Chunking is what a
    // full-policy card actually is.
    //
    // The keyless entries above are deliberately NOT converted: they pin the
    // template-mode wire bytes, which is a different contract, and rewriting them
    // would churn the whole committed corpus to test one more thing.
    Vector { name: "keyed_wpkh",          template: "wpkh(@0/<0;1>/*)",
        keys: &[(0, "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf")],
        fingerprints: &[(0, [0x73, 0xc5, 0xda, 0x0a])], force_chunked: true, path: None },
    Vector { name: "keyed_wsh_multi_2of3", template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",
        keys: &[(0, "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf"), (1, "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk"), (2, "xpub6EGx8sPr9FxPPE1rbZazhqWwpMXA3Hf5DYKtZbL7c4BSddzmQktp96UaTvecEkoCZysuaj79GMCFZYT1KKk7Ph2M3Kf5g8B82KZ8TZ9SKQR")],
        fingerprints: &[(0, [0x73, 0xc5, 0xda, 0x0a]), (1, [0x73, 0xc5, 0xda, 0x0a]), (2, [0x73, 0xc5, 0xda, 0x0a])], force_chunked: true, path: None },
    Vector { name: "keyed_wsh_sortedmulti_2of3", template: "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",
        keys: &[(0, "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf"), (1, "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk"), (2, "xpub6EGx8sPr9FxPPE1rbZazhqWwpMXA3Hf5DYKtZbL7c4BSddzmQktp96UaTvecEkoCZysuaj79GMCFZYT1KKk7Ph2M3Kf5g8B82KZ8TZ9SKQR")],
        fingerprints: &[(0, [0x73, 0xc5, 0xda, 0x0a]), (1, [0x73, 0xc5, 0xda, 0x0a]), (2, [0x73, 0xc5, 0xda, 0x0a])], force_chunked: true, path: None },
    Vector { name: "keyed_tr_keyonly",     template: "tr(@0/<0;1>/*)",
        keys: &[(0, "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf")],
        fingerprints: &[(0, [0x73, 0xc5, 0xda, 0x0a])], force_chunked: true, path: None },
    // THE SHAPE THE WHOLE CYCLE IS ABOUT, and the one no keyless vector could
    // price: a taproot script tree. It needs an explicit `path` because its
    // `canonical_origin` is None -- exactly the case R4's `--path` on
    // address/verify exists to reach.
    Vector { name: "keyed_tr_with_leaf",   template: "tr(@0/<0;1>/*,pk(@1/<0;1>/*))",
        keys: &[(0, "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf"), (1, "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk")],
        fingerprints: &[(0, [0x73, 0xc5, 0xda, 0x0a]), (1, [0x73, 0xc5, 0xda, 0x0a])], force_chunked: true, path: Some("48'/0'/0'/2'") },
    // DEPTH-2, unbalanced: leaf depths (2,2,1). This is the shape the pre-#953
    // renderer flattened, so before the ff4732e pin it could not have had a
    // conformance record at all -- the descriptor string would have been
    // unparseable.
    Vector { name: "keyed_tr_depth2",      template: "tr(@0/<0;1>/*,{{pk(@1/<0;1>/*),pk(@2/<0;1>/*)},pk(@3/<0;1>/*)})",
        keys: &[(0, "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf"), (1, "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk"), (2, "xpub6EGx8sPr9FxPPE1rbZazhqWwpMXA3Hf5DYKtZbL7c4BSddzmQktp96UaTvecEkoCZysuaj79GMCFZYT1KKk7Ph2M3Kf5g8B82KZ8TZ9SKQR"), (3, "xpub6E6Z3Ss5TXJYNJp4U1q3NZ3pCn82i7KXQAKUtNnzLJ3cCdchQeSdFvXemizaHUF7wNwRQAB8mPdoZhGHLiv49cWPtCnoJY3Az3E8JKxH9Mq")],
        fingerprints: &[(0, [0x73, 0xc5, 0xda, 0x0a]), (1, [0x73, 0xc5, 0xda, 0x0a]), (2, [0x73, 0xc5, 0xda, 0x0a]), (3, [0x73, 0xc5, 0xda, 0x0a])], force_chunked: true, path: Some("48'/0'/0'/2'") },
    Vector { name: "wpkh_basic",         template: "wpkh(@0/<0;1>/*)",                                   keys: &[], fingerprints: &[], force_chunked: false, path: None },
    Vector { name: "pkh_basic",          template: "pkh(@0/<0;1>/*)",                                    keys: &[], fingerprints: &[], force_chunked: false, path: None },
    Vector { name: "wsh_multi_2of2",     template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",                keys: &[], fingerprints: &[], force_chunked: false, path: None },
    Vector { name: "wsh_multi_2of3",     template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",     keys: &[], fingerprints: &[], force_chunked: false, path: None },
    Vector { name: "wsh_sortedmulti",    template: "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))", keys: &[], fingerprints: &[], force_chunked: false, path: None },
    Vector { name: "tr_keyonly",         template: "tr(@0/<0;1>/*)",                                     keys: &[], fingerprints: &[], force_chunked: false, path: None },
    Vector { name: "sh_wsh_multi",       template: "sh(wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*)))",            keys: &[], fingerprints: &[], force_chunked: false, path: None },
    Vector { name: "wsh_divergent_paths", template: "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))",               keys: &[], fingerprints: &[], force_chunked: false, path: None },
    Vector { name: "wsh_with_fingerprints", template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
        keys: &[],
        fingerprints: &[(0, [0xDE,0xAD,0xBE,0xEF]), (1, [0xCA,0xFE,0xBA,0xBE])],
        force_chunked: false, path: None },
    Vector { name: "wsh_multi_chunked",  template: "wsh(multi(3,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",     keys: &[], fingerprints: &[], force_chunked: true, path: None },
    // --- Part-3 additions ---
    // F-A1: elided sh(wpkh) now round-trips (canonical origin m/49'/0'/0').
    Vector { name: "sh_wpkh",            template: "sh(wpkh(@0/<0;1>/*))",                               keys: &[], fingerprints: &[], force_chunked: false, path: None },
    // Non-canonical tr()+leaf — explicit origin via the new `path` field.
    Vector { name: "tr_with_leaf",       template: "tr(@0/<0;1>/*,pk(@1/<0;1>/*))",                      keys: &[], fingerprints: &[], force_chunked: false, path: Some("48'/0'/0'/2'") },
    // NUMS-taproot (`is_nums = 1` wire path) — script-path-only tr, explicit origin.
    Vector { name: "nums_taproot",       template: "tr(50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0,multi_a(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",
        keys: &[], fingerprints: &[], force_chunked: false, path: Some("48'/0'/0'/2'") },
    // 2-of-8 sortedmulti + fingerprint on every cosigner: a 376-bit payload
    // that FITS a single string (< the 400-bit regular-code cap), so
    // `force_chunked` routes it through `split`, whose 320-bit per-chunk budget
    // yields a genuine 2-member chunk set.
    Vector { name: "wsh_sortedmulti_2chunk",
        template: "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*,@3/<0;1>/*,@4/<0;1>/*,@5/<0;1>/*,@6/<0;1>/*,@7/<0;1>/*))",
        keys: &[],
        fingerprints: &[
            (0, [0x01,0x02,0x03,0x04]), (1, [0x02,0x03,0x04,0x05]),
            (2, [0x03,0x04,0x05,0x06]), (3, [0x04,0x05,0x06,0x07]),
            (4, [0x05,0x06,0x07,0x08]), (5, [0x06,0x07,0x08,0x09]),
            (6, [0x07,0x08,0x09,0x0A]), (7, [0x08,0x09,0x0A,0x0B]),
        ],
        force_chunked: true, path: None },
    // F-V2: single-string regular-code boundary. 2-of-9 sortedmulti with a
    // fingerprint on 8 of the 9 cosigners sizes the payload to 79 of the 80
    // max data symbols → a single 95-char md1 string (NOT chunked, NOT long).
    Vector { name: "single_string_boundary",
        template: "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*,@3/<0;1>/*,@4/<0;1>/*,@5/<0;1>/*,@6/<0;1>/*,@7/<0;1>/*,@8/<0;1>/*))",
        keys: &[],
        fingerprints: &[
            (0, [0x01,0x02,0x03,0x04]), (1, [0x02,0x03,0x04,0x05]),
            (2, [0x03,0x04,0x05,0x06]), (3, [0x04,0x05,0x06,0x07]),
            (4, [0x05,0x06,0x07,0x08]), (5, [0x06,0x07,0x08,0x09]),
            (6, [0x07,0x08,0x09,0x0A]), (7, [0x08,0x09,0x0A,0x0B]),
        ],
        force_chunked: false, path: None },
];
