**⚠ 1 Critical (#4: `md address` prints a wrong address, exit 0) and 9 Important. #9 is withdrawn because the BIP requires the reported behaviour.**

architect grade-bughunt-md: 7 answered; 3 prior rulings changed (triage grades #3 M→I, #4 N→C, #9 I→withdrawn)

Inputs: triage `bughunt-triage-md.md` (HEAD 05aaa8e8, RED rows f26ab4b5), hunt `bughunt-2026-10-31-md.md`, `/scratch/cache/bughunt/md/fix-NN.patch`.
Binary: `target/debug/md` (`md 0.20.3`) at 05aaa8e8. `X` = signet-test xpub `xpub6DkFAXWQ2dHx…r6KFrf` (the one the triage tests use).
Settled facts were taken as given: 17 rows failed, which is the CONFIRMED set.

## 1. Grades

Verbs that migrate to the toolkit (Brian 2026-11-01): repair, inspect, verify, address/derive.

| # | verb (moves?) | grade | why (likelihood test) | Minor deferred? |
|---|---|---|---|---|
| 1 | encode (stays) | I | `--json` skips `validate_relative_timelocks`, so a check silently does not run and the card records a timelock that consensus does not enforce. | n/a |
| 2 | encode, codec (stays) | I | Same class as #1, reached through `thresh`; the codec walk misses `Body::Variable`. Library reach counts. | n/a |
| 3 | encode (stays) | **I** (was M) | Checked: single-key keyed `wpkh` with `--json` exits 1 with "payload is 115 data symbols; … caps single strings at 80". So every keyed card fails, and the only unusual condition is `--json`. Test (3) fails. | n/a |
| 4 | address/derive (moves) | **C** (was N) | Checked: `md address --template 'wpkh(@0/<2147483648;1>/*)' --key @0=X --count 1` prints `bc1q0qgl7sdpw0vvdn3g0kha052hkjugcm8zujqhea` and exits 0. That is the Normal formula at index 2^31, which BIP-32 defines as hardened, so no signer derives this key. `md descriptor` exits 0 with `…/<2147483648;1>/*)#26hvecv9`. Only `encode` refuses, and with the varint message. Wrong address, so C/I is fixed now. | n/a |
| 5 | shape (codec `policy_shape`; feeds render/coordinator and the engraver's consent screen) | I | Wrong result: `and_v(v:pk(@0),or_d(pk(@1),older(10)))` is summarised as one branch that needs @0, @1 and older(10). That breaks the module's own "honesty contract" (policy_shape.rs:16-25). | n/a |
| 6 | verify (`--verify-against`, seating) (moves) | M | Fails closed: exit 1 with a mint-policy error, and no wrong verdict is given. Conditions: (a) a card from md-codec before F-218 (38cc2fb5, 2026-08-20) that seats one xpub in two slots; (b) the holder runs verify or seating on it. | **yes, to the migration** |
| 7 | codec `canonicalize` (stays) | I | Panics on card-derived input. Library reach. | n/a |
| 8 | encode, codec TLV write (stays) | I | Writes a card that is wrong (tag 0x21 is read back as 0x01 fingerprints) or unreadable (`BitStreamTruncated`), with no error. Library reach. | n/a |
| 9 | decode, codec varint | **withdrawn** (was I) | Not a defect. bip-mnemonic-descriptor.mediawiki:752: "Decoders are '''lenient''': they accept a non-minimal encoding (an over-long L, or extension-form with L_high = 0) with no decode-side minimality check. Enforcing minimality on decode would flip a previously-decodable single-string wire to a reject, violating the recovery-safety invariant." Chunked cards are already caught by `ChunkSetIdMismatch` (same line). Row 09 asserts behaviour that the BIP forbids. | n/a |
| 10 | encode (stays) | M | Fails nothing. Conditions: (a) the opt-in `--policy-id-fingerprint`; (b) stdout piped to a consumer that expects only the artifact. No command md prints uses the flag (mnemonic-gui only declares it, `src/schema/md.rs:204`). Not deferred, because encode stays. | no (fixed in FIX-A) |
| 11 | inspect (moves) | I | A wrong advisory: the note says "keyless (no keys)" while stdout carries xpubs. A wrong result, so it is fixed now. | n/a |
| 12 | encode, parse (stays) | I | Silently accepts `+f+f+f+f` as `0f0f0f0f`, so wrong data goes on the card. | n/a |
| 13 | descriptor seating (`--from-mk1`) | M (provisional) | The dedupe miss is shown, but its effect downstream is not. If the duplicated card seats one key twice, F-218 refuses that at mint (fails closed). Conditions: (a) the same mk1 card supplied twice; (b) once hyphen-grouped and once unbroken. | see Q-B |
| 14 | decode (stays) | N | The note is true. Checked: the printed hint `md address … --path "m/84'/0'/0'"` runs and gives the same address as without `--path` (`bc1qtncy45g8qy8laah6jvxn6ppvj0s9rluxj5uxf6`), exit 0. It is noise only. | no (fixed in FIX-A, trivial) |
| 15 | `gen_ap2_grind` dev tool | no fix | See ruling R-15. | n/a |
| 16 | decode, codec codex32 (stays) | I | A check silently does not run: `md decode 'md1Źq9dsssqrqq20vuhf9agdphs'` exits 0 as if the input were `md1yq9…`. BIP-93: the alphabet is ASCII. | n/a |

## 2. Rulings (exact behaviour)

Every refusal reuses an existing exit code (1) and existing wording wherever possible.

- **R-1 (#1).** The `--json` branch runs `validate_relative_timelocks` before it builds the object. stderr and exit code are byte-identical to the text path (`older(65536) is not what consensus enforces …`, exit 1). No new text.
- **R-2 (#2).** The codec truncation walk descends `Body::Variable` (thresh). The error is the existing `RelativeTimelockTruncated`. No new text.
- **R-3 (#3).** `--json` mints with `mint_md1_cards(&descriptor, args.force_chunked)`, so it chunks exactly when the text path does.
  - The JSON shape is the existing `--force-chunked` shape: `chunk_set_id` (`0x%05x`) + `chunks` when chunked, else `phrase`.
  - Manual 42-md.md:43 becomes `| `--json` | emit JSON output: `phrase` for a single-string card, `chunk_set_id` + `chunks` when the card is chunked (as the text path chooses, or with `--force-chunked`) |`.
- **R-4 (#4).**
  - **CLI:** in `lex_placeholders` (template.rs:294), a bare alt whose value is >= 2^31 is refused with the existing hardened message.
  - Expected for `<2147483648;1>`: `md: template parse error: @0 multipath alt `2147483648` is hardened; hardened derivation is impossible on a watch-only (xpub) card`, exit 1. The same text and exit code were seen today for `<1h;1>` on `address` and `descriptor`.
  - This applies to encode, address and descriptor (they share the lexer). Values in [2^29, 2^31) pass the lexer. The codec refuses them on every path (encode, address, descriptor, and the library entries `to_miniscript_descriptor[_with_network]`, `to_miniscript_descriptor_multipath[_with_network]` and `Descriptor::derive_address`) with the existing `Error::VarintOverflow { value }` (BIP mediawiki:459), exit 1.
  - **Codec:** `to_miniscript` stops falling back to `ChildNumber::Normal/Hardened { index }`. Any origin, use-site or multipath value > 2^29-1 returns the existing `Error::VarintOverflow { value }`, because the BIP says so (mediawiki:459: "Component values up to 2^29 − 1 are representable; larger values are rejected with Error::VarintOverflow { value }"). `origin_path_to_derivation` becomes fallible.
  - New RED rows: `md address` and `md descriptor` with `<2147483648;1>` (expect the text above, exit 1); a codec row that runs `to_descriptor` on a library-built `Descriptor` with alt 2^31 (expect `VarintOverflow`).
  - Triage row 04 keeps its name. Its expected text is the hardened message.
- **R-5 (#5, open Q answered).** No cross product in this fix: the cross product is a feature, and a branch-count cap is a product number that nothing has asked for.
  - The rule: when `branch_of`'s `collect` meets a disjunction tag (`or_b`, `or_c`, `or_d`, `or_i`, `andor`) inside a branch, it returns `false`, so the shape is `complete = false`. This is the module's existing honesty path: no part of the summary is shown.
  - Expected for the hunt's input: `complete == false`. Row 05 changes to assert that, and its name becomes `bughunt_md_05_nested_or_under_and_marks_incomplete`.
  - Top-level `or_*`/`andor` splitting is unchanged.
  - Tie-break: the cross product is also correct but needs a cap value and new branch semantics. The incomplete rule is the smaller spec, so I picked it. Recorded as a tie on correctness.
  - Follow-up (owner: the engraver bug hunt): the Go port `mnemonic-engrave/third_party/seedhammer/gui/composer_*.go` (`md/policy_shape.go`) probably has the same merge. Dispatching that is the coordinator's call.
- **R-7 (#7).** `canonicalize_placeholder_indices` checks every TLV idx < n and returns the existing `PlaceholderIndexOutOfRange { idx, n }` (fix-07 as written).
- **R-8 (#8, open Q: refuse).** `TlvSection::write` refuses before writing:
  - (a) An unknown tag outside 4..=31 gets a **new variant** `Error::TlvUnknownTagOutOfRange { tag: u8 }`, text `unknown TLV tag 0x{tag:02x} is outside the reserved range 0x04..=0x1f`. fix-08's reuse of `TlvOrderingViolation{prev:3}` misstates the problem.
  - (b) Two unknown entries with the same tag get the existing `TlvOrderingViolation { prev: tag, current: tag }`.
  - The source is BIP mediawiki:639 and :654 (reserved 0x04-0x1F; strictly ascending; duplicates rejected).
  - An error-enum variant is not a new type, so this stays small-track (my ruling).
  - Row 08b keeps tag 1 as its input. Add row `bughunt_md_08c_duplicate_unknown_tlv_tag_refused`.
- **R-9 (#9, open Q: neither).** Keep the decoder lenient, as BIP:752 requires. Drop fix-09.
  - Rename and invert row 09 to `bughunt_md_09_varint_noncanonical_extension_accepted_per_bip` with `assert_eq!(v, Ok(5))`. This pins the spec rule.
  - The hunt's "one value, one wire form" premise is wrong for single strings, and the BIP says so deliberately.
- **R-10 (#10, open Q: stderr).** Change `println!` to `eprintln!` at encode.rs:230. The text is unchanged: `policy-id-fingerprint: 0x%08x`.
  - Manual 42-md.md:40 becomes `| `--policy-id-fingerprint` | print the freshly-computed PolicyId fingerprint on stderr after the card (stdout carries only the md1 artifact) |`.
  - The test that changes is cmd_encode.rs:151 `encode_with_policy_id_fingerprint_prints_two_lines`. Its stdout assertion moves to stderr, and its name becomes `encode_with_policy_id_fingerprint_prints_it_on_stderr`.
- **R-11 (#11).** Both inspect sites (`--json` at inspect.rs:85 and the text path) choose WatchOnly when `is_wallet_policy()`, as encode and repair do. Add a RED row for the text path: `bughunt_md_11b_inspect_text_keyed_card_not_called_keyless`. No new text: it reuses encode's WatchOnly note.
- **R-12 (#12).** Every fingerprint character must be an ASCII hex digit. Expected: `md: --fingerprint @0: invalid hex byte: +f`, exit 1. This is the existing per-byte wording; today's `@0=zzzzzzzz` gives `invalid hex byte: zz`, exit 1.
- **R-14 (#14, open Q: canonical).** "Canonical" means exactly `md_codec::canonical_origin::canonical_origin(&d.tree) == Some(origin)`, with one key and no fingerprint TLV. In that case the note and the `--path` hint are suppressed. Every other case is unchanged. No manual row shows the note, so the manual does not change.
- **R-16 (#16).** Non-ASCII characters reach the existing invalid-character error in both the single-string and chunk paths. Expected: `md: codec error: codex32 decode error: character 'Ź' not in codex32 alphabet`, exit 1. That is the same text seen today for `'b'`. fix-16's new chunk wording is replaced by this existing one.
- **R-15 (#15, does it need fixing?).** No. `gen_ap2_grind` is a developer tool that is not shipped, and fixture determinism matters only when someone regenerates a fixture. File it as a FOLLOWUPS `deferred-minor` Nit owned by "next AP2 fixture regeneration", with fix text "`fetch_min` on the winning counter; threads stop once past the best". No row.
- **#6 and #13.** Minor. #6 defers to the toolkit migration of verify and seating, and the migration's spec owes a row that asserts a verdict on a pre-F-218 duplicate-slot card. For #13 see Q-B.

## 3. FIX documents

| FIX | contents | repo / lockstep | track |
|---|---|---|---|
| FIX-A md-cli intake and output | #1 #3 #4 (CLI half) #10 #11 #12 #14 | descriptor-mnemonic `md-cli`; manual 42-md.md:40 and :43 | small: no new flag, type or command, about four spec sentences (R-3, R-10, R-14, R-4) |
| FIX-B md-codec | #2 #4 (codec guard) #5 #7 #8 #16; #9 row inversion | descriptor-mnemonic `md-codec`; no manual change | small: one error variant (R-8), spec sentences for R-5 and R-8; the rest are refusals the BIP already states |

Review class: both are B (address and descriptor derivation, codec), and A for FIX-A (printed text and manual).

## 4. Open questions (not mine)

- **Q-B (Brian's alone).** Does `md descriptor` seating (`--from-mk1`) count as "address/derive" for verb placement? If yes, #13 defers to the migration. If no, it is owned by the next md hunt and needs a downstream row first (does a duplicate seat refuse, as I expect, or mis-seat?). If a row shows a mis-seat, #13 becomes I.
- Whether the engraver Go port (R-5 follow-up) is hunted now or later is the coordinator's scheduling call.

Counts: C 1, I 9, M 3 (#6 #10 #13), N 1 (#14); withdrawn 1 (#9); no-fix 1 (#15); FIX documents 2 (both small track); open questions 2.
