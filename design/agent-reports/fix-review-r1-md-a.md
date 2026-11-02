r1: 0C/0I/3M/2N

# FIX review r1: md FIX-A (`design/FIX_bughunt_md_a.md`)

Reviewer: Opus, effort medium. Target head 1a7a6abf (base b2bfdd63), read-only.
Executed in a scratch detached worktree at 1a7a6abf with FIX-B §2.5 (varint
guard in md-codec `to_miniscript.rs`) and FIX-A §2.1-§2.9 applied literally;
own `CARGO_TARGET_DIR`. First review of this doc: no previous fix commit, so
0 of these findings come from a fold.

## Verdict

The doc is correct and Sonnet-proof for every behaviour it changes. Every
§2 edit could be applied from the doc text alone. Every changed text and
printed command was executed and matched the doc. No C/I.

## Evidence (executed)

- RED on unmodified 1a7a6abf, §6 filter: 12 run, 0 passed, 12 failed. This
  matches the controller's `fixa-red2.log`.
- GREEN with FIX-A §2 + FIX-B §2.5 applied: the full workspace nextest run
  gave 1560 run, 1552 passed, 8 failed. All 8 failures are md-codec FIX-B
  rows (02/05/07/08/08b/09/16/16b) whose FIX-B fixes I did not apply, so they
  are expected and outside FIX-A. All 12 FIX-A md-cli rows pass. The renamed
  `encode_with_policy_id_fingerprint_prints_it_on_stderr` passes. Rows 06/13
  are skipped as ignored.
- BIP: `bip/bip-mnemonic-descriptor.mediawiki` :459 gives the varint max
  `2^29 − 1`, and :750 gives `Error::VarintOverflow { value }`. The ruled Q1
  text matches.
- Manual at mnemonic-toolkit f6355e22 (`git show`): line 40 reads
  `| \`--policy-id-fingerprint\` | print the freshly-computed PolicyId fingerprint after the phrase |`
  and line 43 reads `| \`--json\` | emit JSON output |`. Both match the doc's
  "old" texts exactly, so the replacement anchors are unique.

## Case tables (all executed on the scratch GREEN tree)

### R-1 (§2.1) `--json` checks relative timelocks
| input | decided (doc §2.1) | observed |
|---|---|---|
| `encode --json` with an invalid BIP-68 older() | same refusal as text path, exit 1 | stderr identical to text path, exit 1 |
| same, text path | unchanged refusal | unchanged, exit 1 |
| valid older() `--json` | JSON emitted | emitted, exit 0 |

### R-3 (§2.2) `--json` chunking follows `mint_md1_cards`
| input | decided | observed |
|---|---|---|
| short policy, `--json` | `{"phrase": cards[0], ...}` | `phrase` key, single string |
| `--force-chunked --json` | `chunk_set_id` + `chunks` | as decided |
| policy that text-path chunks, `--json` | chunked JSON | chunked JSON, same cards as text |
| key order | serde_json default (alphabetical) | alphabetical (no preserve_order) |

### R-4 CLI half (§2.3) bare multipath alt >= 2^31
| input | decided | observed |
|---|---|---|
| `wpkh(@0/<2147483648;1>/*)` (address) | hardened text, exit 1 | `md: template parse error: @0 multipath alt \`2147483648\` is hardened; hardened derivation is impossible on a watch-only (xpub) card`, exit 1 |
| `<1;2147483648>` (descriptor), non-first alt | same (Q2: every alt) | same text, exit 1 |
| `<4294967296;1>` | Q3 text "not a bare unsigned integer" | as ruled, exit 1 |
| `<536870912;1>` | Q1 varint text | `md: codec error: varint value 536870912 exceeds single-extension range (max 2^29 - 1)`, exit 1 |
| `<536870911;1>` boundary | accepted | address printed, exit 0 |
| `wsh(multi(2,@0/<2147483648;1>/*,@1/<0;1>/*))` (row 04) | hardened text | as decided |

### R-10 (§2.4) `--policy-id-fingerprint` to stderr
| input | decided | observed |
|---|---|---|
| `encode wpkh --policy-id-fingerprint` | stdout card only; fp on stderr | stdout `md1yqpqqxqq8xtwhw4xwn4qh`; stderr `policy-id-fingerprint: 0x3ace1082` |
| with `--json` | unchanged JSON field (manual-40 Minor already filed) | not re-reported |

### R-11 (§2.5) inspect calls a keyed card watch-only
| input | decided | observed |
|---|---|---|
| keyed (wallet-policy) card, text inspect | WatchOnly note | WatchOnly note |
| keyed card, `--json` inspect | WatchOnly class | WatchOnly |
| keyless template card | Template note unchanged | unchanged |

### R-12 (§2.6) fingerprint must be hex digits
| input | decided | observed |
|---|---|---|
| `--fingerprint @0=+fffffff` | `md: --fingerprint @0: invalid hex byte: +f`, exit 1 | exact, exit 1 |
| `deadbeef` | accepted | accepted |
| `DEADBEEF` | accepted (is_ascii_hexdigit) | accepted |
| non-UTF-8 chunk | existing "non-utf8 hex", exit 1 | unchanged (see N1) |

### R-14 (§2.7) no origins note for a single canonical key
| input | decided | observed |
|---|---|---|
| `decode md1yq802gggqpsqwgtua24e7ssf3` (from `encode wpkh --path m/84h/0h/0h`) | stdout `wpkh(@0/<0;1>/*)`, keyless note only | exact |
| single key, non-canonical origin | origins note + `md address` hint kept | kept |
| multi-key | note kept | kept |
| single key with fingerprint | note kept | kept |

### Manual lockstep (§2.8)
Lines 40 and 43 are anchored exactly (verified above). The new texts describe
the observed stdout/stderr split. The `--json` caveat is the filed Minor.

## Create-input table (commands the changed texts print or tell the holder to run)

| printed command | source | flags vs clap | ran |
|---|---|---|---|
| `md address --template <the line above> --key @i=<xpub> --path m/…` (decode hint, unchanged text, now suppressed only in the canonical single-key case) | decode.rs | `--template`, `--key`, `--path` all exist on `address` | yes (rows 04b/04d use the same flags) |
| `md encode wpkh --path m/84h/0h/0h` (doc §2.7 derivation of the row-14 card) | doc §2.7 | `--path` exists on `encode` (clap: `--in --out --from-policy --context --unspendable-key --path --key --fingerprint --network --force-chunked --group-size --separator --force-long-code --policy-id-fingerprint --json --experimental`) | yes, gives `md1yq802gggqpsqwgtua24e7ssf3` |
| `md address --template … --key @0=… --count 1` (row commands) | §3 | `--count` exists | yes |

No printed command fails as printed.

## Deferred-minor re-grade

FIX-A changes the text of no existing `deferred-minor` in `design/FOLLOWUPS.md`.
It only adds the #6 and #13 entries, which are settled. Nothing to re-grade.

## Findings (appended as found)

### M1: §2.9 parenthetical conditions the text on an event the builder cannot observe
`design/FIX_bughunt_md_a.md:180`: "(If Q-B is answered "no", only #13's owner
line becomes "next md bug hunt".)" Q-B belongs to Brian alone
(grade-bughunt-md.md:85), and the architect's 522b83d7 ruling (:140) keeps the
ignore. A Sonnet builder cannot tell whether Q-B has been answered, so it may
STOP. Fails safe, because the default owner line is given. Conditions: (1) the
builder reads the parenthetical as a gate; (2) Q-B is still open at build
time. Fix: replace the line with "Write #13's owner line as given; Q-B is
Brian's and does not block this commit."

### M2: the §3 row table mis-describes three committed rows
§3 lists commands for rows 04, 12 and 14 that differ from the committed
`crates/md-cli/tests/bughunt.rs`:
- Row 04's first assertion uses `wsh(multi(2,@0/<2147483648;1>/*,@1/<0;1>/*))`.
- Row 12 uses keyed wsh-multi args.
- Row 14 encodes and then decodes, rather than decoding the literal card.

The doc's literal commands were run and give the stated outputs, so no printed
command fails. Only the description drifts from the tests. Conditions: (1) a
builder edits a row to match the table; (2) it does so instead of leaving
committed rows alone. Fix: make each of the three §3 cells quote the committed
row's command.

### M3: the R-11 WatchOnly note on text `md inspect` says "public keys only" where stdout prints no keys
This is the grade's ruled class and it fails safe (it under-claims spend
power). Conditions: (1) the holder runs text inspect on a keyed card; (2) the
holder reads "public keys only" as a claim about the printed output. This is
arguably a design line. Fix (optional): none required for FIX-A; file it with
the inspect migration.

### N1: §2.6 does not say whether the hex check goes before or after `from_utf8`
`{s}` in the text implies after. A non-UTF-8 chunk gives "non-utf8 hex" either
way, exit 1.

### N2: §2.6 cites "line 522", but `parse_key_with_origin` has a similar loop (keys.rs:665)
The function name `parse_fingerprint` disambiguates, and the :665 loop is
already hex-checked.

## Design line (ungraded, pre-existing, out of scope)

- `<+5;1>` gives an internal "lexer/substitution divergence" error, and
  `<+2147483648;1>` is reported as hardened alt `+2147483648`. Both fail
  closed. `u32::from_str` accepts a leading `+`.
- `encode --json` silently ignores `--out`.

C / I / M / N: 0 / 0 / 3 / 2
