# Prior Art Survey — Wallet Descriptor Mnemonic Encoding

**Status:** v1, populated 2026-04-26 from primary sources. Update as new
prior art is discovered or as we learn more about the entries below.

**Purpose:** Document existing approaches to encoding wallet descriptors,
miniscript policies, and bitcoin secrets in formats that overlap with the
**Wallet Descriptor Mnemonic (WDM)** format. For each entry: what it does,
what it's good for, what it lacks for our use case, and whether/how we
should build on it.

---

## Summary table

| Format | Compact | Strong ECC | Engravable | Designed for descriptors/policies |
|---|---|---|---|---|
| Plain BIP 388 wallet policy string | ✗ | ✗ (no checksum) | ✓ but verbose | ✓ |
| BIP 380 plain descriptor string | ✗ | partial (hash-checksum suffix only) | ✓ but verbose | ✓ |
| BIP 39 mnemonic + 4-letter prefix | ✓ for seeds | weak (1-word checksum) | ✓ | ✗ (seeds only) |
| SLIP 39 (Shamir mnemonic) | ✓ for seeds | weak | ✓ | ✗ (seeds only) |
| Codex32 / BIP 93 | ✓ for seeds | **strong (BCH)** | ✓ (designed for) | ✗ (HRP fixed to "ms" for seeds) |
| descriptor-codec (joshdoman) | ✓ (30–40% reduction) | ✗ | ✗ (binary, no rendering) | ✓ |
| seedhammer PSBT-flavored binary | ~ | ✗ | ✗ | ✓ |
| Liana `.bed` (encrypted descriptor) | n/a (encrypted) | n/a | ✗ | ✓ |
| Blockchain Commons UR + Bytewords | ~ | weak | ✓ (4-letter words) | ✓ |
| Coldcard multisig file | ✗ | ✗ | ✗ (text+JSON) | partial (multisig only) |

**Conclusion:** No existing format gets all four of [compact, strong ECC,
engravable, descriptor/policy-aware]. Codex32 is the only format with
strong ECC + engravability, but it is locked to seed payloads by its
HRP. descriptor-codec is the only mature compact descriptor binary
encoding, but it has no ECC and no rendering. The contribution gap is
clear.

---

## 1. Codex32 (BIP 93)

**What it is:** A bech32-derived encoding for BIP 32 master seeds and
Shamir shares thereof. Designed explicitly for hand-transcription onto
paper or steel.

**Structure:**

```
ms 1 <threshold> <4-char identifier> <share index> <payload> <checksum>
└─┘ │      │            │                  │            │           │
HRP sep   1 char       4 chars           1 char    up to 74      13 or 15
                                                  (or 103) chars   chars
```

- HRP: fixed as `"ms"` (lowercase) or `"MS"` (uppercase) — no subtypes
- Separator: `"1"` (bech32 convention)
- Threshold: `"0"` = unshared secret, `"2"`–`"9"` = k-of-n threshold
- Identifier: 4 bech32 chars naming this seed/share group
- Share index: `"s"` for unshared secret; other char for Shamir share
- Payload: variable length up to 46 bytes (regular) or ~64 bytes (long)
- Checksum: BCH ECC, 13 chars (regular) or 15 chars (long)

**Alphabet:** `qpzry9x8gf2tvdw0s3jn54khce6mua7l` (32 chars; no
upper/lower mixing within a string).

**Error correction:**
- Regular strings (≤93 data chars): detect all errors affecting ≤8 chars;
  correct up to 4 substitutions, 8 erasures, or 13 consecutive erasures
- Long strings (94–108 data chars): detect ≤8; correct up to 4
  substitutions, 8 erasures, or 15 consecutive erasures
- **Note:** strings of 94–95 data chars are invalid (gap between regular
  and long).

**What's good for us:**
- **Strong BCH ECC** is exactly what engraved-steel backups need
- **Bech32 alphabet** is well-tested for handwriting/engraving (no
  visually confusable characters like `0`/`O` or `1`/`I`)
- **Reference implementations** exist in Rust, Python, JavaScript, C
- **Standardization track** (BIP 93 draft) — adoption story is real

**What's missing for us:**
- **HRP is fixed as `"ms"`.** Codex32 has no extensibility mechanism for
  non-seed payloads. The threshold field encodes Shamir parameters, not a
  payload type. There are no reserved bits, version field, or subtype
  marker.
- **A new payload type for wallet policies requires a new BIP** with a
  distinct HRP (e.g., `"wdm"` for "wallet descriptor mnemonic" or `"wp"`
  for "wallet policy").
- **Payload size cap.** 46 bytes (regular) covers our v0 estimates for
  typical policies, but complex policies with master fingerprints can
  reach ~60 bytes — into the "long string" range. We need a clear
  multi-string story for policies that exceed even the long limit.

**How we use it:**

- **Adopt:** the bech32 alphabet (32 chars, no confusables)
- **Adopt:** the BCH polynomial(s) for ECC — both regular and long
- **Replace:** the HRP and framing — define a new HRP (proposed
  `"wdm"`) with our own header semantics
- **Define:** a multi-string concatenation scheme for payloads that exceed
  the long-string limit (probably needed only for v1+ wallet policies
  with foreign xpubs)

This is fork-and-extend, not direct reuse. The cryptographic primitives
(alphabet, BCH polynomial) are reusable; the framing must be redesigned.

---

## 2. descriptor-codec (joshdoman, Rust, CC0)

**Repo:** https://github.com/joshdoman/descriptor-codec

**What it is:** A Rust library that encodes BIP 380 output descriptors
(including miniscript) as compact binary using single-byte operator tags
and LEB128 varints. Decodes back to descriptor strings. Claims 30–40%
size reduction vs. plaintext.

**Tag table** (50 tags, 0x00–0x31):

| Category | Tags |
|---|---|
| Boolean | False (0x00), True (0x01) |
| Top-level wrappers | Pkh, Sh, Wpkh, Wsh, Tr, Bare (0x02–0x07) |
| Taproot | TapTree (0x08) |
| Multisig | SortedMulti (0x09), Multi (0x19), MultiA (0x1A) |
| Miniscript ops | Alt, Swap, Check, DupIf, Verify, NonZero, ZeroNotEqual (0x0A–0x10), AndV, AndB, AndOr, OrB, OrC, OrD, OrI, Thresh (0x11–0x18) |
| Key scripts | PkK (0x1B), PkH (0x1C), RawPkH (0x1D) |
| Timelocks | After (0x1E), Older (0x1F) |
| Hashes | Sha256, Hash256, Ripemd160, Hash160 (0x20–0x23) |
| Key origins | Origin (0x24), NoOrigin (0x25) |
| Public keys | UncompressedFullKey, CompressedFullKey, XOnly, XPub, MultiXPub (0x26–0x2A) |
| Private keys | UncompressedSinglePriv, CompressedSinglePriv, XPriv, MultiXPriv (0x2B–0x2E) |
| Wildcards | NoWildcard, UnhardenedWildcard, HardenedWildcard (0x2F–0x31) |

**Path encoding:** LEB128 varint for length and child numbers. Hardened
encoded as `2c+1`, unhardened as `2c`.

**API:**
- `encode(descriptor: &str) -> Result<Vec<u8>>`
- `decode(data: &[u8]) -> Result<String>`

**License:** CC0-1.0 (public domain equivalent). Directly forkable.

**What's good for us:**
- **Most of the canonical-template-bytecode work is already done.** The
  tag table covers every miniscript fragment we need plus all top-level
  wrappers and key types.
- **CC0 license** — no friction to fork, extend, or relicense.
- **Complete miniscript coverage** — including the operators we'd
  otherwise have to enumerate ourselves.
- **Good encoding choices** — LEB128 for varints, separate tags for
  hardened vs. unhardened wildcards, distinct key-type tags.

**What's missing for us:**
- **No BIP 388 wallet policy support.** The format encodes raw descriptors
  with embedded keys; there's no `@i` placeholder concept. We'd need to
  add new tags or a new framing layer for the wallet-policy abstraction.
- **No ECC.** Output is raw bytes, intended for QR/NFC, not for
  hand-transcription.
- **No rendering layer.** No bech32, no codex32, no word list. We'd add
  this on top.
- **No version byte.** First byte is whatever opcode the encoded
  descriptor starts with; no explicit version/format header.

**How we use it:**

- **Adopt:** the tag table as the basis for our miniscript fragment
  encoding
- **Adopt:** the LEB128 path encoding scheme
- **Add:** new tags for `@i` placeholders (probably 0x32–0x3F or
  similar) replacing the key tags for our wallet-policy framing
- **Add:** a single-byte path-dictionary tag for shared-path encoding
  (v0 case)
- **Add:** a version/header byte before the encoded template
- **Wrap:** the output bytes in our codex32-derived framing for ECC and
  rendering

descriptor-codec becomes the bytecode layer (§6.1 of POLICY_BACKUP.md);
codex32-derived framing becomes the rendering layer (§5).

---

## 3. Blockchain Commons UR + Bytewords

**Specs:**
- UR: https://developer.blockchaincommons.com/ur/
- Bytewords: https://developer.blockchaincommons.com/bytewords/
- Crypto-output (descriptor type): well-documented; used by Sparrow,
  Foundation Passport, Keystone, others

**What it is:** A two-layer encoding scheme:
- **UR** (Uniform Resources): wraps any binary data — including bitcoin
  descriptors — in a CBOR envelope with a type tag, optionally chunked
  for animated QR codes. Result is binary or bech32-rendered.
- **Bytewords:** encodes binary data as a sequence of 4-letter English
  words, with each word's first and last letter forming a unique
  "minimal" 2-letter code.

**Wallet descriptor support:** UR has dedicated types for output
descriptors (`crypto-output`) and HD keys (`crypto-hdkey`) with
established CBOR schemas.

**What's good for us:**
- **Existing word-based descriptor encoding** — closest existing analog
  to what we want
- **Real adoption** — Sparrow, Passport, Keystone, etc. ship this
- **Standardized CBOR schemas** for descriptors and HD keys
- **Bytewords is engraving-friendly** — fixed-length 4-letter words

**What's missing for us:**
- **Weak error correction.** Bytewords adds a 4-byte checksum (CRC32);
  no BCH-style error *correction*, only *detection*. For an engraved
  steel artifact subject to scratches and partial obliteration, this is
  insufficient.
- **Verbose for our use case.** Each byte → ~2 bytewords characters
  (worst case), full 4-letter form → 4 chars per byte. A 30-byte template
  encodes to ~120 chars — comparable to codex32 but with much weaker ECC.
- **CBOR overhead.** Adds a few bytes of envelope per backup;
  meaningful at our small payload sizes.
- **Not designed for engraving specifically.** Designed for QR
  transmission; engraving is a possible-but-unintended use.

**How we use it:**

- **Reference:** as the existing "descriptor as words" prior art —
  document what's good and bad in PRIOR_ART
- **Possibly fallback:** if codex32-extension fails, Bytewords + a
  stronger external ECC layer is a fallback path
- **Not adopted as primary.** Weak ECC disqualifies it for the engraving
  use case.

---

## 4. Liana `.bed` (Bitcoin Encrypted Descriptor)

**Repo / docs:**
- https://wizardsardine.com/blog/liana-13.0-release/
- https://wizardsardine.com/blog/liana-10.0-release/

**What it is:** Liana wallet's production backup format (default in v13+).
A `.bed` file containing the wallet's descriptor and metadata
(aliases, labels, transactions), encrypted at rest. Intended for digital
storage with redundancy across cloud, USB, etc.

**What's good for us:**
- **Production-shipping miniscript wallet backup format** — proves the
  problem is real and Liana ships a solution
- **Encryption-at-rest** is a thoughtful default for a digital backup
- **Round-trips with Liana's wallet state** including non-policy data
  (labels, aliases)

**What's missing for us:**
- **Not human-transcribable.** Encrypted binary file.
- **Not engravable.** Same.
- **Not recoverable from steel alone.** A user with only their seed +
  steel-stamped artifacts cannot use a `.bed` file — they need the
  digital file too.

**How we use it:**

- **Reference:** as the "competent wallet's bulk-digital backup story" —
  any wallet adopting WDM should consider an analogous *secondary*
  backup mechanism alongside the steel-stamped Template Card
- **Not adopted.** Solves a different problem (digital bulk backup, not
  engravable backup).

---

## 5. seedhammer PSBT-based descriptor encoding (BC Research #135)

**Issue:** https://github.com/BlockchainCommons/Research/issues/135 (Oct 2023)

**What it is:** A proposal for a binary encoding of wallet descriptors
patterned on PSBT structure (magic header, key-value maps). Motivated by
Liana. Targets QR transmission. Go reference implementation exists.

**Status:** Sketch / proof-of-concept. Has not progressed to a finalized
spec or BIP.

**What's good for us:**
- **PSBT-flavored binary** is familiar to bitcoin tooling
- **Includes BIP 388 wallet-policy framing** with `@<key-index>`
  references — relevant prior art for how to encode placeholders
- **Wallet name + birthdate** in the global map — useful precedent

**What's missing for us:**
- **No human-transcribable form** discussed
- **No ECC**
- **PSBT-style key-value framing is heavier than tag-based bytecode**
  for our payload sizes

**How we use it:**

- **Reference:** for how to encode `@i` placeholders in a binary format
  (we'd compare with descriptor-codec's tag-based approach and pick the
  smaller one)
- **Not adopted as primary encoding.** Tag-based (descriptor-codec) is
  more compact for our use case.

---

## 6. Coldcard multisig file format

**Docs:** https://coldcard.com/docs/multisig (and related Coldcard docs)

**What it is:** A plain-text file format for sharing multisig wallet
configurations across cosigners and onto Coldcard for registration.
Contains: wallet name, policy (M-of-N), derivation path, list of
xpubs with origin info. Header-commented, human-readable.

**What's good for us:**
- **Human-readable** — a user can inspect the file and understand it
- **Production-tested** — Coldcard, Sparrow, Specter all consume it
- **Self-describing** — header lines name the wallet and policy

**What's missing for us:**
- **Multisig-only.** Cannot represent arbitrary miniscript.
- **No ECC.**
- **Verbose.** A typical 2-of-3 file is 500–1500 bytes — much more than a
  steel plate can hold.
- **Not engraving-grade.** Intended for SD-card transport, not
  hand-transcription.

**How we use it:**

- **Reference:** for what *information* a backup needs to convey
  (wallet name, threshold, derivation path, xpubs/keys)
- **Not adopted.** Wrong scope and wrong density.

---

## 7. BIP 39 mnemonic with 4-letter prefix

**Standard:** BIP 39

**What it is:** The seed phrase standard. 2048-word English wordlist
where the first 4 letters of each word are unique, allowing
abbreviation. Conventionally used for steel engraving by stamping the
4-letter prefixes (e.g., "abandon" → `aban`).

**What's good for us:**
- **Universally understood by bitcoin users**
- **Robust 4-letter prefix convention** for engraving
- **Mature wordlist** — distinctness audited, multiple language variants
- **Fits commercial steel backup products** — every steel plate vendor
  accommodates 24 × 4-letter words

**What's missing for us:**
- **Designed for fixed-entropy seeds**, not variable-length policies
- **Weak error correction** (single-word checksum at end of mnemonic)
- **2048 words = 11 bits per word** — less character-economical than
  bech32 alphabet's 5 bits per char (4-letter prefix = 20 bits but only
  encodes 11 bits of data)
- **No standardized way to encode anything other than a seed**

**How we use it:**

- **Conservative fallback** for the WDM encoding layer if codex32
  extension proves unworkable
- **UX inspiration** — the 4-letter-prefix convention sets user
  expectations for engraving
- **Not adopted as primary** — weaker ECC and lower character density
  than codex32-derived encoding

---

## 8. SLIP 39 (Shamir mnemonic)

**Standard:** https://github.com/satoshilabs/slips/blob/master/slip-0039.md

**What it is:** Trezor's BIP 39 extension to Shamir's secret sharing.
Splits a seed into N shares with k-of-n recovery threshold; each share
is encoded as a separate mnemonic from a custom 1024-word list.

**What's good for us:**
- **Demonstrates word-list encoding for variable-length structured data**
  (each share has more structure than BIP 39)
- **Includes group/share metadata** in the encoded payload — precedent
  for header bytes in a word-encoded format
- **Strong checksum** (RS1024 — Reed-Solomon over GF(1024))

**What's missing for us:**
- **Designed for seed shares**, not arbitrary policies
- **1024-word custom list** is a fragmentation cost; codex32's bech32
  alphabet is more universal

**How we use it:**

- **Reference:** for how to add metadata/headers to a word-encoded
  format
- **Reference:** Reed-Solomon over GF(1024) is an alternative ECC scheme
  (vs. codex32's BCH). Probably not adopted but worth knowing.

---

## Decisions implied by this survey

Three concrete decisions emerge from the prior-art landscape:

### Decision A: Build the bytecode layer on descriptor-codec
- CC0 license, Rust, miniscript-complete tag table
- Extend with `@i` placeholder tags (proposed: 0x32–0x3F range)
- Extend with a path-dictionary tag for shared-path encoding (v0 case)
- Add a version/header byte at the start of every encoded payload

### Decision B: Build the encoding layer as codex32-derived
- Reuse codex32's bech32 alphabet
- Reuse codex32's BCH polynomial (regular and long variants)
- Define a new HRP — proposed: `"wdm"` (wallet descriptor mnemonic);
  alternatives `"wp"` or `"dm"` — and seek BIP-equivalent registration
  when the spec stabilizes
- Define a WDM-specific header structure (no Shamir threshold;
  instead a payload-version byte and possibly a wallet-name field)
- Define a multi-string concatenation scheme for payloads that exceed
  codex32's long-string limit (deferred until needed)

### Decision C: Position Bytewords as fallback, not primary
- Bytewords is the only existing word-based descriptor encoding in
  production, but its weak ECC disqualifies it for engraving-grade backup
- If codex32 extension proves unworkable for any reason, Bytewords +
  external ECC is the fallback path

These three decisions together close the gap identified in the summary
table: compact (descriptor-codec) + strong ECC (codex32) + engravable
(codex32 alphabet) + descriptor/policy-aware (descriptor-codec extended
for BIP 388 placeholders).

---

## Open prior-art questions to investigate later

- **Sparrow's wallet export format** — JSON-based; how does it compare to
  Coldcard's multisig file format? Any policy-aware extensions?
- **Specter's wallet config** — similar JSON-based; same comparison.
- **Border Wallets** — visual-grid seed encoding; tangentially relevant
  but worth a brief look for completeness.
- **CryptoCurrency Security Standard (CCSS)** and related guidance on
  wallet backup hygiene — does any standard explicitly call for policy
  backup beyond seed?
- **Nunchuk's backup format** — libnunchuk has its own approach; should
  be reviewed.
- **Active BIP discussions on bitcoin-dev** about descriptor encoding
  (post-2024) — is there a current proposal we should align with?

---

## References

- [BIP 93 (codex32) spec](https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki)
- [BIP 388 (wallet policies)](https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki)
- [BIP 380 (output descriptors)](https://github.com/bitcoin/bips/blob/master/bip-0380.mediawiki)
- [BIP 39 (mnemonic seed phrases)](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [SLIP 39 (Shamir mnemonic)](https://github.com/satoshilabs/slips/blob/master/slip-0039.md)
- [descriptor-codec by joshdoman](https://github.com/joshdoman/descriptor-codec)
- [Blockchain Commons Research #135 (PSBT-based descriptor encoding)](https://github.com/BlockchainCommons/Research/issues/135)
- [Blockchain Commons UR developer docs](https://developer.blockchaincommons.com/ur/)
- [Blockchain Commons Bytewords](https://developer.blockchaincommons.com/bytewords/)
- [Liana 13.0 release notes](https://wizardsardine.com/blog/liana-13.0-release/)
- [Liana 10.0 release notes](https://wizardsardine.com/blog/liana-10.0-release/)
