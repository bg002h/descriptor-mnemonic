// Vectors corpus source-of-truth. Used both by `md vectors` and by
// `tests/template_roundtrip.rs`.

pub struct Vector {
    pub name: &'static str,
    pub template: &'static str,
    pub keys: &'static [(u8, &'static str)],
    pub fingerprints: &'static [(u8, [u8; 4])],
    pub force_chunked: bool,
}

pub const MANIFEST: &[Vector] = &[
    Vector { name: "wpkh_basic",         template: "wpkh(@0/<0;1>/*)",                                   keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "pkh_basic",          template: "pkh(@0/<0;1>/*)",                                    keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_multi_2of2",     template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",                keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_multi_2of3",     template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",     keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_sortedmulti",    template: "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))", keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "tr_keyonly",         template: "tr(@0/<0;1>/*)",                                     keys: &[], fingerprints: &[], force_chunked: false },
    // tr_with_leaf and sh_wpkh intentionally omitted: their round-trip via the v0.14 codec
    // is asymmetric (encode requires explicit origin; decode strips canonical 86'/0'/0'
    // resp. 49'/0'/0'). Coverage for these wrappers is preserved by parse::template
    // unit tests (tr_with_one_leaf, sh_wpkh_nested).
    Vector { name: "sh_wsh_multi",       template: "sh(wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*)))",            keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_divergent_paths", template: "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))",               keys: &[], fingerprints: &[], force_chunked: false },
    Vector { name: "wsh_with_fingerprints", template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
        keys: &[],
        fingerprints: &[(0, [0xDE,0xAD,0xBE,0xEF]), (1, [0xCA,0xFE,0xBA,0xBE])],
        force_chunked: false },
    Vector { name: "wsh_multi_chunked",  template: "wsh(multi(3,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",     keys: &[], fingerprints: &[], force_chunked: true },
];
