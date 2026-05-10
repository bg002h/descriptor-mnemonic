# Spike — v0.4.5 BIP-text refresh (bitstream-native rewrite)

Pre-Phase-1 spike persisting source-of-truth bit-layouts for the v0.11+ wire
format, keyed off `crates/md-codec/src/{header,origin_path,use_site_path,tlv,tree,chunk,varint,bitstream,identity}.rs`
and `design/SPEC_v0_11_wire_format.md`. Verified against `md encode` +
`md bytecode` output for six representative templates.

## Top-level payload structure

Order, no byte alignment between sections:

1. Header (5 bits)
2. Origin path declaration (variable)
3. Use-site path declaration (variable, always present)
4. Tree (variable, recursive)
5. TLV section (variable, may be empty)

Bit packing convention: MSB-first. Final byte zero-padded if the total bit
count is not a multiple of 8.

## Header (5 bits)

```
bit 4   bit 3    bits 2..0
[paths] [resv=0] [version (3 bits)]
```

- bits 0..2: version. v0.11 = 0b000.
- bit 3: reserved. Encoders MUST emit 0; decoders MUST reject 1
  (`Error::ReservedHeaderBitSet`).
- bit 4: divergent-paths flag. 0 = shared origin path; 1 = per-`@N`
  divergent paths.

(Reference: `header.rs:24-28, 37-42`.)

Note on bit 3 dual use: the chunk header (used for chunked Template Cards)
places its `chunked` flag at bit 3 and sets it to 1; the payload header
(single-string mode) sets bit 3 to 0. Decoders dispatch chunked-vs-single by
inspecting bit 3 of the first 5-bit symbol after the HRP.

## Origin path declaration

Dispatches on header bit 4:

- Shared mode (bit 4 = 0): `[n: 5 bits, encoded n-1] [origin-path-encoding]`
- Divergent mode (bit 4 = 1): `[n: 5 bits, encoded n-1] [origin-path-encoding × n]`

`n` is the descriptor's key count, range 1..32, encoded as `n-1`. Range
violation: `Error::KeyCountOutOfRange { n }` (`origin_path.rs:111-112`).

origin-path-encoding: `[depth: 4 bits] [component × depth]`. depth range
0..15; max 15 enforced (`MAX_PATH_COMPONENTS = 15` in `origin_path.rs:43`).

component: `[hardened: 1 bit] [value: LP4-ext varint]`. Hardened flag is the
BIP 32 hardened bit; value is the un-hardened-bit u31 (max 2^29 - 1 via
LP4-ext capacity).

## Use-site path declaration

Always present (one shared default applied to all `@N` placeholders unless
overridden by the `UseSitePathOverrides` TLV).

```
[has-multipath: 1 bit]
[if has-multipath:
  [alt-count: 3 bits, encoded count-2; range 2..9]
  [alternative × count]
]
[wildcard-hardened: 1 bit]
```

alternative: `[hardened: 1 bit] [value: LP4-ext varint]`.

(Reference: `use_site_path.rs:1-150`. `MIN_ALT_COUNT = 2`, `MAX_ALT_COUNT = 9`.)

## Tree

Each node: `[primary-tag: 5 bits] [body]`. Primary tags 0x00-0x1E are
operators; 0x1F is an extension prefix introducing a 5-bit extension
subcode (0x00-0x04 currently).

Body classes:

- KeyArg (Wpkh, Pkh, PkK, PkH): `[key_index: kiw bits]` where
  `kiw = ⌈log₂(n + 1)⌉`. `key_index` range 0..n-1 (n is reserved for the
  `tr()` NUMS sentinel, see Tr below).
- Variable-arity (Multi, SortedMulti, MultiA, SortedMultiA, Thresh):
  `[k-1: 5 bits] [n-1: 5 bits] [child × n]`. k, n in 1..32; k ≤ n.
- Tr: `[key_index: kiw bits] [has-tree: 1 bit] [if has-tree: tree-node]`.
  key_index = n signals the BIP 341 NUMS H-point (no key encoded on the
  wire; conforming decoders substitute the canonical x-only NUMS hex).
- Single-child wrappers (Sh, Wsh, Check, Verify, Swap, Alt, DupIf, NonZero,
  ZeroNotEqual): `[child]`.
- Binary (AndV, AndB, OrB, OrC, OrD, OrI): `[left] [right]`.
- AndOr (3-ary): `[a] [b] [c]`.
- TapTree (recursive 2-ary inner-node): `[left] [right]`. No bit-count
  prefix; recursion stops at leaf tags. Depth cap `MAX_DECODE_DEPTH = 128`
  enforced per node.
- Timelock (After, Older): `[value: 32 bits, BIP 68 / BIP 113 raw u32]`.
- Hash256 (Sha256, Hash256): `[hash: 256 bits]`.
- Hash160 (Hash160, Ripemd160, RawPkH): `[hash: 160 bits]`.
- Empty (False, True): tag only.

`kiw = ⌈log₂(n + 1)⌉`: the +1 reserves the sentinel value `n` for Tr's NUMS
H-point. v0.17 carried NUMS via `Tag::TrUnspendable` (extension subcode
0x05); v0.18 freed that subcode and switched to the sentinel.
(`encode.rs:40-43`, `decode.rs:20-24`.)

## TLV section

Each entry: `[tlv-tag: 5 bits] [length: LP4-ext varint, in BITS] [payload: length bits]`.
Entries appear in strictly ascending tag-value order; duplicates rejected.
Unknown tags are skipped via length-prefix advancement (forward-compat).

Tag allocations as of v0.13:

| Code | TLV entry | Notes |
|---|---|---|
| 0x00 | UseSitePathOverrides | Sparse `(idx: kiw bits, use-site-path)` list |
| 0x01 | Fingerprints | Sparse `(idx: kiw bits, fingerprint: 32 bits)` list |
| 0x02 | Pubkeys | Sparse `(idx: kiw bits, xpub: 65 bytes)` list (v0.13) |
| 0x03 | OriginPathOverrides | Sparse `(idx: kiw bits, origin-path-encoding)` list |
| 0x04..0x1F | RESERVED | Forward-compat skip via length-prefix advancement |

Sparse-TLV invariants: each `idx < n`; strictly ascending; no duplicates;
each TLV non-empty (encoder omits the entry entirely if empty).

End-of-section detection (rollback-as-padding): TLV section runs until the
codex32 symbol stream is exhausted. The final byte may carry up to 7 bits
of trailing zero padding from the symbol-aligned wire. Decoders save the
BitReader position before each candidate entry; on parse failure with
≤ 7 bits remaining, the trailing bits are tolerated as padding and the
section ends cleanly. With > 7 bits remaining, the failure propagates as a
genuine error.

## LP4-ext varint

```
single-form (L < 15):     [L: 4 bits] [payload: L bits]
extension-form (L = 15):  [L: 4 bits = 15] [L_high: 4 bits]
                          [payload_low: 14 bits]
                          [payload_high: L_high bits]
                          total = (payload_high << 14) | payload_low
```

Capacity: 14 + L_high ≤ 14 + 15 = 29 bits → max value 2^29 - 1.
Overflow rejection: `Error::VarintOverflow { value }` (`varint.rs:31`).

Bit costs: value 0 → 4 bits; value 1 → 5 bits; value 84 → 11 bits;
value 16383 → 18 bits (boundary); value 16384 → 23+ bits (extension form).

## Chunk header (chunked-mode only)

37 bits, replaces the payload header in chunked-mode strings:

```
bits 0..2:  version (3 bits, v0.11 = 0)
bit 3:      chunked flag (= 1 for chunk header, vs 0 for payload header)
bit 4:      reserved (= 0)
bits 5..24: chunk_set_id (20 bits)
bits 25..30: count - 1 (6 bits; total chunks in set, range 1..64)
bits 31..36: index (6 bits; this chunk's zero-based index, < count)
```

(Reference: `chunk.rs:15-67`.)

`chunk_set_id` is derived from the assembled payload's `Md1EncodingId`
(SHA-256 of the canonical bit-packed payload bytes, truncated to 16 bytes;
top 20 bits become the chunk-set id). Every chunk in a set carries the
same chunk_set_id; cross-chunk integrity is the consistency check at
reassembly time (decoder recomputes `Md1EncodingId` from the joined
payload and compares its top 20 bits against the per-chunk-header
`chunk_set_id`). There is no separate 4-byte hash appended to the bytecode
in v0.11+ — the payload-bit count from the bitstream encoder is the only
section-length signal.

## md encode bitstream verification

Six templates verified end-to-end. Each row is `(template, path) → md1
phrase, payload bits`:

| # | Template | Path | Phrase | Bits |
|---|---|---|---|---|
| 1 | `wpkh(@0/<0;1>/*)` | `84'/0'/0'` | `md1qq802gggqpsqjphwttu9xhh9p` | 58 |
| 2 | `wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))` | `48'/0'/0'/2'` | `md1qpfdsssj5qqcgcy9g2gesk5yjw8lgj2y` | 92 |
| 3 | `tr(@0/<0;1>/*)` | `86'/0'/0'` | `md1qq80tgggqpsg88lzxawsqrlpe` | 59 |
| 4 | `tr(@0/<0;1>/*,pk(@1/<0;1>/*))` | `48'/0'/0'/2'` | `md1qpfdsssj5qqcy4yq50p2saz68dhr` | 73 |
| 5 | `tr(@0/<0;1>/*,multi_a(2,@1/<0;1>/*,@2/<0;1>/*,@3/<0;1>/*))` | `48'/0'/0'/2'` | `md1qrfdsssj5qqcy2qgj32ffsg50cx8xxj7max` | 106 |
| 6 | `tr(@0/<0;1>/*,{pk(@1/<0;1>/*),pk(@2/<0;1>/*)})` | `48'/0'/0'/2'` | `md1qzfdsssj5qqcyj492s3w9eelx8vqrn` | 85 |

Template 1 hand-verified bit-by-bit against the formulas above:

```
hex 00 0E F5 21 08 00 60 00      (8 bytes; 58 bits used + 6 padding)
58 bits = 5 (header)
       + 31 (path-decl: 5 n=1 + 4 depth=3 + 12 c[0]=84' + 5 c[1]=0' + 5 c[2]=0')
       + 16 (use-site: 1 + 3 + 5 + 6 + 1)
       + 6 (tree: 5 wpkh + 1 key_index)
       + 0 (TLV section empty)
```

The 12-bit cost of `84'` decomposes as 1 (hardened) + 4 (L=7) + 7
(payload `1010100` = 84). The 5-bit cost of `0'` decomposes as 1 (hardened)
+ 4 (L=0, no payload).

## Critical implementation invariants

- `kiw = ⌈log₂(n + 1)⌉` formula MUST be identical encoder-side and
  decoder-side; diverges between n=1 (1 bit) and n=2 (2 bits).
- Sparse-TLV `idx` field width is uniformly `kiw` bits, NOT `⌈log₂(n)⌉`.
  (The kiw formula is reused so the bit-width matches the tree's
  key_index field.)
- Rollback-as-padding tolerance (≤ 7 bits) at TLV-section end is
  load-bearing for symbol-aligned codex32; without it, the wire would
  need an explicit terminator.
- Empty TLV omission is enforced both encoder-side (reject) and
  decoder-side (reject or treat as padding).
- The header's bit-3 dual-use convention (payload header bit 3 = 0,
  chunk header bit 3 = 1) is the chunked-vs-single dispatcher signal.

## BIP-side gaps surfaced by the spike

- BIP currently lacks any "Use-site path declaration" section. Needs to
  be added (analogous to the SPEC's §3.5).
- BIP's "General structure" prose says "bytecode is followed by a 4-byte
  cross-chunk integrity hash" — wrong for v0.11+. Cross-chunk integrity
  is via the 20-bit chunk_set_id in each chunk header; no separate hash.
- BIP's Path declaration prose was written against an 8-bit-tag namespace
  with `Tag::SharedPath = 0x34` / `Tag::OriginPaths = 0x36`. Those tags
  do not exist in v0.11+; the dispatch is via header bit 4 directly.
- BIP's MAX_PATH_COMPONENTS = 10 (pre-v0.11 cap) is incorrect. Current
  cap is 15.
- BIP's Bytecode header table (8-bit, fingerprints flag + OriginPaths
  flag at bits 2-3) is incorrect. v0.11+ header is 5 bits, fingerprints
  are now a TLV entry, and bit 4 (not bit 3) holds the divergent-paths
  flag.
- BIP's byte-layout examples (wsh-multi at L598-628 and 5 taproot shapes
  at L668-729) cannot be produced by a v0.11+ encoder. Need to be
  re-rendered as bit-layout examples sourced from `md encode`.
