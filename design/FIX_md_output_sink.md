# FIX-md-output-sink-2026-11: a stdout that cannot take md's output (crash sweep F1 family, Important)

Small-fix track. Repo: descriptor-mnemonic (branch `fix/md-output-sink`, from
main `6b70d5d964d9dafcae0c1cfef926a0a7887ec6a9`). Crate: `md-cli` only. One
section of the toolkit manual, `docs/manual/src/40-cli-reference/42-md.md`,
changes in lockstep, but in a separate toolkit change and only at the pin bump
(section 8, S1 ruled); the md build does not edit the toolkit.

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (the repo's
`rust-toolchain.toml`); `--locked`; nextest `-j 8`; never `--release`.

Inputs: coordinator `agent-reports/grade-ms-md-output-errors.md` (sections 3 to 5
bind: the md rulings, S1, the fix, the rows and the texts), its files
`/scratch/cache/grade-ms-md-output-errors/md/{fix.diff,rows.diff,plants.txt}`
(copied byte for byte into section 3 and section 4 below), the rule it applies
(`agent-reports/grade-mk-crash-sweep-1.md` Q4) and the md triage
(`agent-reports/triage-md-output-errors.md`). The same fix in mk, already
written: mnemonic-key `design/FIX_mk_output_sink.md`.

Everything below is transcribed from the grade. The grade built this exact
diff and ran it both ways: RED on the base `10 tests run: 2 passed, 8 failed`
(the 2 passes are guards); GREEN `-p md-cli --features cli-compiler`
`942 tests run: 942 passed, 3 skipped` and default features
`904 tests run: 904 passed, 3 skipped`. Where the grade did not decide a point
that a holder can see, it is a STOP in section 10, not a choice.

**One STOP is open (S2, a proposal: the `<COMMIT>` in the FOLLOWUPS entry).**
S1 (the manual's timing) is ruled and is section 8.

## 1. Spec delta (exact texts)

No change to the wire spec: this is CLI behaviour only. No flag, no help text
(so the GUI mirror of md's schema is untouched). The holder-visible texts, all
from the grade:

1. Stderr line, printed once, at exit, with a newline:

   > md: cannot write the output: <error>

   `<error>` is the `io::Error`'s Display, e.g.
   `No space left on device (os error 28)`. The prefix is `md:` (the prefix of
   md's other stderr lines, and mr's form).

2. CHANGELOG, `CHANGELOG.md`, under `## md-cli [Unreleased]`, a new `### Fixed`
   (it appears verbatim in the diff of section 3.1).

3. Manual section `## When stdout fails` (section 8), plain text.

No version bump (grade section 5): no test or CI rule forces one (md has no
`version_is_honest` test and no `Unreleased` check under `tests`, `ci` or
`.github`), and md bumps in `release:` commits. The next release is a patch,
md-cli 0.20.6 ("Fixed" only). Do not touch `Cargo.toml` or `Cargo.lock`.

## 2. Behaviour, refusals, exit codes, error variants

The rule (grade-mk-crash-sweep-1 Q4, applied to md by the grade). One module,
`src/out.rs`, is the crate's only stdout path: its `print!`/`println!`
(and `eprint!`/`eprintln!`) macros shadow std's in every module, and the
second binary `gen_ap2_grind` includes the same file. `--help` and `--version`
go through it too (`main` uses `Cli::try_parse()` and prints clap's text by
the sink).

- **Closed reader** (`ErrorKind::BrokenPipe`, `md address ... --count 1000 | head -3`):
  the output stops, nothing is printed about it, the exit code is the verb's
  own (0.20.5 panicked with exit 101).
- **Any other stdout write error** (a full disk, `> /dev/full`), from a verb or
  `--help`/`--version`: the output stops; at exit `md: cannot write the
  output: <error>` is printed on stderr. A code that means "the run succeeded
  and stdout is the result" becomes **1** (md's existing I/O-error code). These
  codes, each named from the grade's rulings, are: **0**; **4** (a partial
  decode: `decode` or `inspect` of a card whose shape has no origin to
  resolve); **5** (`repair` that corrected the card, and `descriptor
  --verify-against` when the composition is NOT spend-equal: its stdout is the
  composition, and its verdict line goes to stderr, still printed). A failing
  code (**1**, **2**, anything else) is kept. The test is on the number (0, 4,
  5) because md's `Ok(2)` from `repair` (an uncorrectable card) is a failure,
  so an Ok/Err test would be wrong.
- **Stderr write errors** (`2>/dev/full`, a closed `2>&1 | head`) are
  ignored: the text is dropped and the result is unchanged.
- No new exit code, no new error variant, no flag change. A clap parse error is
  printed on stderr by clap's `print()`; a stderr failure there is ignored and
  its exit code (2) is unchanged.

| # | situation | stdout | stderr | exit | row |
|---|---|---|---|---|---|
| 1 | closed reader, verb's code 0 (`gui-schema`, `encode`, `decode`, `address --count 1000` text and `--json`) | stops, no text | no `cannot write`, no panic | 0 | `closed_reader_exits_with_the_verbs_code` |
| 1b | closed reader, `repair` that repaired (code 5) | stops | none | 5 (not 1, not 101) | `closed_reader_exits_with_the_verbs_code` |
| 2 | other stdout error (`/dev/full`, ENOSPC), verb's code 0: 23 invocations, 24 with `--features cli-compiler` (adds `compile`): `gui-schema`; `encode` (text, `--json`); `decode`, `inspect`, `repair` and `compose` and `address` (each text and `--json`); `bytecode`; `verify`; `shape-key`; `descriptor`; `decompose` (default and `--emit all`); `--help`, `--version`, `decode --help`, `help encode`; `compile` with the feature | stops | `md: cannot write the output: No space left on device (os error 28)` last line | 1 | `full_stdout_exits_1_with_the_line` |
| 2b | the same, `repair` that repaired (code 5; text and `--json`) | stops | the line | 1 (not 5) | `full_stdout_after_a_repair_exits_1_not_5` |
| 2c | the same, partial decode (code 4: `decode`, `inspect`) | stops | the line | 1 (not 4) | `full_stdout_after_a_partial_decode_exits_1_not_4` |
| 2d | the same, `descriptor --verify-against` with a NOT spend-equal verdict (code 5) | stops | the verdict line (still printed), then the line | 1 (not 5) | `r3_a_not_equal_verdict_whose_stdout_is_lost_exits_1_and_keeps_the_verdict_line` |
| 3 | verb already failing (codes 1, 2), stdout full | nothing was printed | the verb's own text; no `cannot write` line (it never touched stdout) | 1 or 2 (kept) | `full_stdout_with_nothing_to_print_keeps_the_verbs_code` (public path) and the unit test `out::tests::a_closed_pipe_keeps_every_code_and_a_write_error_turns_success_into_1` (codes 1, 2, 64 kept after a write error; no md verb writes stdout and then fails, so no public path exists) |
| 4 | `--help` / `--version` / `decode --help` / `help encode`, closed reader | stops | none | 0 | `closed_reader_exits_with_the_verbs_code` |
| 5 | the same four, other stdout error | stops | the line | 1 (0.20.5: exit 0 with no text) | `full_stdout_exits_1_with_the_line` |
| 6 | stderr write error: stderr is a closed pipe or `/dev/full` (`compose` notes), and stdout and stderr both closed (`encode --policy-id-fingerprint`, `compose`, `address`, `decode`, `decode not-a-card`, `repair` of a bad card) | as case 1 | ignored | the verb's own (0, 1 or 2) | `a_stderr_that_cannot_take_text_never_changes_the_result`, `closed_reader_on_stdout_and_stderr_exits_0` |
| 7 | clap parse error (printed on stderr) | none | clap's text; a stderr failure is ignored | 2 (unchanged) | none new (no change in behaviour) |
| 8 | the second binary `gen_ap2_grind` (a maintainer tool that installs next to `md`): its one `print!` goes through `out.rs`, `main` returns `ExitCode::from(out::exit_code(0))` | same rule | same rule | 0, or 1 with the line | `every_binary_includes_the_out_module` (a source scan: the binary is a ~2^32 SHA-256 grind, too slow to run in a row) |
| 9 | no module but `out.rs` names stdout | n/a | n/a | n/a | `only_out_rs_names_stdout` (a source scan) |

`--out FILE` and the reserved-tag line of the ms document do not exist in md.
A short write (`ulimit -f`) is not a row.

## 3. Production changes (exact)

The five blocks 3.1 to 3.5 are `fix.diff` split by file, in the diff's own
order, byte for byte: concatenated they equal the grade's file (251
lines, sha256 `d380793acd7710e2394d0798da3654b1c101146a41e4c0c3e9de7459d94405af`, checked by the gate). Apply them with `git apply`.
They touch `CHANGELOG.md`, `crates/md-cli/src/bin/gen_ap2_grind.rs`,
`crates/md-cli/src/main.rs`, `crates/md-cli/src/out.rs` (new, 99 lines) and
`design/FOLLOWUPS.md` (two entries: `md-output-sink` and
`md-gen-ap2-grind-installed`, the latter Minor, open, owned by the next md-cli
release). Block 3.5 is the FOLLOWUPS text, from the grade's diff; its literal
`resolved <COMMIT>` is S2.

### 3.1 `CHANGELOG.md`

```diff
diff --git c/CHANGELOG.md i/CHANGELOG.md
index c5b084c..90ea393 100644
--- c/CHANGELOG.md
+++ i/CHANGELOG.md
@@ -6,6 +6,22 @@ The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
 
 ## md-cli [Unreleased]
 
+### Fixed
+- **A stdout that cannot take the output no longer panics or reports success.**
+  Every verb wrote with `println!`, which panicked (exit 101, "failed printing
+  to stdout") on a full disk or `> /dev/full`, and `--help` and `--version`
+  exited 0 with no text. Now any stdout write error other than a closed pipe
+  prints `md: cannot write the output: <error>` on stderr and exits 1; a verb
+  that was already failing keeps its own code (1, 2). A code that carries a
+  result becomes 1 when that result never reached stdout: a partial decode's
+  4, `repair`'s 5, and `descriptor --verify-against`'s 5 (its verdict line on
+  stderr is still printed). A reader that closes the pipe early (`md address …
+  --count 1000 | head -3`) only stops the output, and the exit code is the
+  verb's own, where 0.20.5 panicked with exit 101. A stderr that cannot take
+  text (`2>/dev/full`, or `2>&1 | head` once the reader exits) no longer panics:
+  the text is dropped and the result is unchanged. The maintainer binary
+  `gen_ap2_grind` takes the same path. `tests/output_errors.rs`.
+
 ## md-cli [0.20.5] — 2026-11-04
 
 No functional change since 0.20.4. Only CI workflow files, root `Cargo.toml`
```

### 3.2 `crates/md-cli/src/bin/gen_ap2_grind.rs`

```diff
diff --git c/crates/md-cli/src/bin/gen_ap2_grind.rs i/crates/md-cli/src/bin/gen_ap2_grind.rs
index a976f28..252d061 100644
--- c/crates/md-cli/src/bin/gen_ap2_grind.rs
+++ i/crates/md-cli/src/bin/gen_ap2_grind.rs
@@ -52,6 +52,12 @@
 //! Measured runtime and the resulting fixture's SHA-256 are recorded in
 //! `design/agent-reports/impl-seat-ap-p0.md`.
 
+// The crate's one stdout path (src/out.rs): its print macros shadow std's, so a
+// full stdout exits 1 with a line instead of panicking.
+#[path = "../out.rs"]
+#[macro_use]
+mod out;
+
 use bitcoin::NetworkKind;
 use bitcoin::bip32::{ChainCode, ChildNumber, DerivationPath, Fingerprint, Xpub};
 use bitcoin::hashes::{Hash, sha256};
@@ -59,6 +65,7 @@ use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
 use mk_codec::bytecode::encode_bytecode;
 use mk_codec::string_layer::{StringLayerHeader, bytes_to_5bit, encode_5bit_to_string};
 use mk_codec::{CHUNKED_FRAGMENT_LONG_BYTES, KeyCard};
+use std::process::ExitCode;
 use std::str::FromStr;
 use std::time::Instant;
 
@@ -146,7 +153,7 @@ fn stub_list(base_tag: u8, counter: u32) -> Vec<[u8; 4]> {
     stubs
 }
 
-fn main() {
+fn main() -> ExitCode {
     // Card A and B: IDENTICAL 13-stub list (shared chunk 0), distinct fp + xpub.
     let shared_stubs: Vec<[u8; 4]> = stub_list(0xAB, 0);
     let card_a = make_card(shared_stubs.clone(), 0x11, 0xA1);
@@ -317,4 +324,5 @@ fn main() {
         out.push('\n');
     }
     print!("{out}");
+    ExitCode::from(out::exit_code(0))
 }
```

### 3.3 `crates/md-cli/src/main.rs`

```diff
diff --git c/crates/md-cli/src/main.rs i/crates/md-cli/src/main.rs
index 0052ac3..7d2749d 100644
--- c/crates/md-cli/src/main.rs
+++ i/crates/md-cli/src/main.rs
@@ -1,5 +1,9 @@
 #![allow(missing_docs)]
 
+// First, so its print macros shadow std's in every module below.
+#[macro_use]
+mod out;
+
 mod bip388;
 mod cmd;
 #[cfg(feature = "cli-compiler")]
@@ -891,18 +895,32 @@ fn collect_mk1(inline: &[String], file: Option<&std::path::Path>) -> Result<Vec<
 fn main() -> ExitCode {
     // v0.6.1: deny other-UID /proc/$PID/cmdline reads + core dumps.
     process_hardening::set_non_dumpable();
-    let cli = Cli::parse();
-    match dispatch(cli.command) {
-        Ok(code) => ExitCode::from(code),
+    let cli = match Cli::try_parse() {
+        Ok(cli) => cli,
+        Err(e) => {
+            // --help and --version go to stdout under the same write-error
+            // rule as the verbs (`out`); a parse error goes to stderr.
+            if let Err(w) = e.print() {
+                if !e.use_stderr() {
+                    out::failed(&w);
+                }
+            }
+            let code = u8::try_from(e.exit_code()).unwrap_or(2);
+            return ExitCode::from(out::exit_code(code));
+        }
+    };
+    let code = match dispatch(cli.command) {
+        Ok(code) => code,
         Err(CliError::BadArg(m)) => {
             eprintln!("md: {m}");
-            ExitCode::from(2)
+            2
         }
         Err(e) => {
             eprintln!("md: {e}");
-            ExitCode::from(1)
+            1
         }
-    }
+    };
+    ExitCode::from(out::exit_code(code))
 }
 
 /// v0.18 Item G — reject `--unspendable-key` values that aren't the BIP-341
```

### 3.4 `crates/md-cli/src/out.rs` (new)

```diff
diff --git c/crates/md-cli/src/out.rs i/crates/md-cli/src/out.rs
new file mode 100644
index 0000000..20e8a96
--- /dev/null
+++ i/crates/md-cli/src/out.rs
@@ -0,0 +1,99 @@
+//! The one stdout path. Every `print!`/`println!` in md-cli resolves to the
+//! macros below (declared `#[macro_use]` first in `main.rs`, so they shadow
+//! std's in every module), and so do `eprint!`/`eprintln!`.
+//!
+//! std's macros panic (exit 101) when a write fails. Here a write error stops
+//! the output instead. A reader that closed the pipe (`md … | head -3`,
+//! `ErrorKind::BrokenPipe`) only stops it: the exit code is the verb's own.
+//! Any other stdout error (a full disk, `> /dev/full`) is kept, and `main`
+//! prints "md: cannot write the output: <error>" and exits 1 (crash sweep,
+//! grade-mk-crash-sweep-1 Q4). A stderr write error is ignored.
+//! `tests/output_errors.rs` pins both cases and that no other file of the
+//! crate names stdout.
+
+use std::io::Write;
+use std::sync::OnceLock;
+use std::sync::atomic::{AtomicBool, Ordering};
+
+static CLOSED: AtomicBool = AtomicBool::new(false);
+static WRITE_ERROR: OnceLock<String> = OnceLock::new();
+
+/// Writes to stdout; after the first failure, writes nothing.
+pub(crate) fn stdout_write(args: std::fmt::Arguments<'_>) {
+    if CLOSED.load(Ordering::Relaxed) {
+        return;
+    }
+    let mut o = std::io::stdout().lock();
+    if let Err(e) = o.write_fmt(args).and_then(|()| o.flush()) {
+        failed(&e);
+    }
+}
+
+/// Records a failed stdout write (also used for clap's `--help`/`--version`).
+pub(crate) fn failed(e: &std::io::Error) {
+    CLOSED.store(true, Ordering::Relaxed);
+    if e.kind() != std::io::ErrorKind::BrokenPipe {
+        let _ = WRITE_ERROR.set(e.to_string());
+    }
+}
+
+/// Writes to stderr, ignoring a failure.
+pub(crate) fn stderr_write(args: std::fmt::Arguments<'_>) {
+    let _ = std::io::stderr().lock().write_fmt(args);
+}
+
+/// The process exit code: `code`, unless a stdout write failed for a reason
+/// other than a closed pipe. Then the error is printed, and a code that says
+/// the output is the result (0, partial decode's 4, `repair`'s 5 and
+/// `descriptor --verify-against`'s 5) becomes 1; a failing code (1, 2) is kept.
+pub(crate) fn exit_code(code: u8) -> u8 {
+    match WRITE_ERROR.get() {
+        None => code,
+        Some(e) => {
+            stderr_write(format_args!("md: cannot write the output: {e}\n"));
+            if matches!(code, 0 | 4 | 5) { 1 } else { code }
+        }
+    }
+}
+
+macro_rules! print {
+    ($($a:tt)*) => { $crate::out::stdout_write(format_args!($($a)*)) };
+}
+// Unused in `src/bin/gen_ap2_grind.rs`, which includes this file.
+#[allow(unused_macros)]
+macro_rules! println {
+    () => { $crate::out::stdout_write(format_args!("\n")) };
+    ($($a:tt)*) => { $crate::out::stdout_write(format_args!("{}\n", format_args!($($a)*))) };
+}
+// Unused today; defined so a future `eprint!` cannot reach std's.
+#[allow(unused_macros)]
+macro_rules! eprint {
+    ($($a:tt)*) => { $crate::out::stderr_write(format_args!($($a)*)) };
+}
+macro_rules! eprintln {
+    () => { $crate::out::stderr_write(format_args!("\n")) };
+    ($($a:tt)*) => { $crate::out::stderr_write(format_args!("{}\n", format_args!($($a)*))) };
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    // One test, because the statics cannot be reset: a closed pipe first
+    // (every code kept), then a write error (success codes become 1, failing
+    // codes kept). Two tests would share the statics under `cargo test`.
+    #[test]
+    fn a_closed_pipe_keeps_every_code_and_a_write_error_turns_success_into_1() {
+        failed(&std::io::Error::from(std::io::ErrorKind::BrokenPipe));
+        for c in [0u8, 1, 2, 4, 5] {
+            assert_eq!(exit_code(c), c, "closed pipe, code {c}");
+        }
+        failed(&std::io::Error::from_raw_os_error(28));
+        for ok in [0u8, 4, 5] {
+            assert_eq!(exit_code(ok), 1, "success-with-output code {ok}");
+        }
+        for bad in [1u8, 2, 64] {
+            assert_eq!(exit_code(bad), bad, "failing code {bad}");
+        }
+    }
+}
```

### 3.5 `design/FOLLOWUPS.md`

```diff
diff --git c/design/FOLLOWUPS.md i/design/FOLLOWUPS.md
index ba9694d..f65d947 100644
--- c/design/FOLLOWUPS.md
+++ i/design/FOLLOWUPS.md
@@ -3304,3 +3304,21 @@ exceeds a resource limit) or record that this one is intentionally latent.
 - **Spec:** mnemonic-toolkit `design/SPEC_vendor_freshness_shared.md`.
 - **Status:** resolved -- shipped by this PR
 - Companion: mnemonic-toolkit design/FOLLOWUPS.md::vendor-freshness-shared
+
+### `md-output-sink` — a stdout or stderr write error panicked (exit 101); `--help`/`--version` exited 0 with the text lost (repo: **descriptor-mnemonic**; owning phase: this PR)
+
+- **Surfaced:** 2026-11-13, the crash sweep's F1 family rule (coordinator `agent-reports/grade-mk-crash-sweep-1.md` Q4) and the md triage (`agent-reports/triage-md-output-errors.md`); graded in `agent-reports/grade-ms-md-output-errors.md`: full stdout, closed reader (`md address … --count 1000 | head -3` panicked 10 of 10), stderr write error and `--help`/`--version` all Important.
+- **Where:** every `print!`/`println!`/`eprintln!` of `crates/md-cli/src` and `src/bin/gen_ap2_grind.rs`; clap's `Cli::parse()` in `main.rs`.
+- **What:** One stdout path, `src/out.rs` (macros that shadow std's; `gen_ap2_grind` includes the same file): a closed pipe stops the output and keeps the verb's code; any other stdout error prints `md: cannot write the output: <error>` and turns 0, 4 and 5 into 1, keeping 1 and 2; a stderr error is ignored. Rows: `tests/output_errors.rs` and `out::tests`.
+- **Companion:** mnemonic-toolkit `design/FOLLOWUPS.md::md-manual-when-stdout-fails` (the `42-md.md` section; it lands with the toolkit PR that pins the first md-cli release carrying this fix).
+- **Status:** `resolved <COMMIT>`
+- **Tier:** `cross-repo`
+
+### `md-gen-ap2-grind-installed` — the fixture grinder `gen_ap2_grind` installs with `md` (repo: **descriptor-mnemonic**; owning phase: next md-cli release)
+
+- **Surfaced:** 2026-11-13, md output-error triage section 5 (`agent-reports/triage-md-output-errors.md`).
+- **Where:** `crates/md-cli/src/bin/gen_ap2_grind.rs` (auto-discovered: `crates/md-cli/Cargo.toml` declares only `[[bin]] md` and no `autobins = false`).
+- **What:** `cargo install … md-cli` (the README's commands and the toolkit's `scripts/install.sh`, which passes no `--bin`) installs this maintainer tool, a 2^32 SHA-256 grind that regenerates `tests/fixtures/seating/v-ap2.txt`, next to `md` in the holder's bin directory. The release archives carry only `md`. Take it out of the default build (`[[bin]] name = "gen_ap2_grind"` with `required-features = ["maintainer-tools"]`, and the fixture's regeneration note updated), and keep `tests/output_errors.rs::every_binary_includes_the_out_module` for whatever stays in `src/bin`.
+- **Why deferred:** not a defect in any output (Minor); the output-sink PR routes it through `out.rs` as it ships today.
+- **Status:** `open`, Minor.
+- **Tier:** `v1+`
```


## 4. RED rows

The rows are `crates/md-cli/tests/output_errors.rs` (9 tests, new file) and one
row appended to `crates/md-cli/tests/r3_verify_against.rs`, byte for byte
`rows.diff` (361 lines, sha256 `5f59d692c62aba711b4bd69ab55639be53a2ca579cd85ec480c44a2897c7af50`). They supersede the
triage's rows; do not commit those. Apply the blocks, then run each command in
the `sh` block on the unmodified code, from the repo root with
`$HOME/.cargo/bin` first in PATH (the pin is 1.85.0). The text after
`## expect:` is a substring of the failing run's output. The rows are
`cfg(target_os = "linux")`.

```diff
diff --git c/crates/md-cli/tests/output_errors.rs i/crates/md-cli/tests/output_errors.rs
new file mode 100644
index 0000000..1580a0b
--- /dev/null
+++ i/crates/md-cli/tests/output_errors.rs
@@ -0,0 +1,310 @@
+//! What md does when stdout cannot take its output (crash sweep, the family
+//! rule of grade-mk-crash-sweep-1 Q4; toolkit manual `42-md.md`
+//! "When stdout fails").
+//!
+//! - A reader that closed the pipe only stops the output: no text, no
+//!   signal, the exit code is the verb's own.
+//! - Any other write error (`/dev/full`, ENOSPC), from a verb or `--help`:
+//!   "md: cannot write the output: <error>" on stderr, exit 1. A code that
+//!   means success with the output as the result (0, partial decode 4,
+//!   repair 5) becomes 1; a failing code is kept (pinned by the unit test in
+//!   `src/out.rs`, since no md verb fails after printing).
+//! - No file of the crate but `src/out.rs` names stdout.
+#![allow(missing_docs)]
+#![cfg(target_os = "linux")]
+
+use std::fs::OpenOptions;
+use std::os::fd::{FromRawFd, OwnedFd};
+use std::os::unix::process::ExitStatusExt;
+use std::process::{Command, Output, Stdio};
+
+const XPUB: &str = "xpub6BmeGmRo4LosAcU21HDaGcvtaQ7GrqQcY48nBkE22qM6KVwQUjRJ1BGzk84SFVHgLcd61Vcnhr8petHexjjn5WbQ9PriVrRhphw4oCp2z6a";
+const TEMPLATE: &str = "wpkh(@0/<0;1>/*)";
+const CARD: &str = "md1yqpqqxqq8xtwhw4xwn4qh";
+/// `CARD` with its last character changed: repair corrects it and exits 5.
+const DAMAGED: &str = "md1yqpqqxqq8xtwhw4xwn4qx";
+const FULL: &str = "md: cannot write the output: No space left on device (os error 28)";
+
+fn md() -> Command {
+    Command::new(env!("CARGO_BIN_EXE_md"))
+}
+
+fn s(v: &[&str]) -> Vec<String> {
+    v.iter().map(|x| (*x).to_owned()).collect()
+}
+
+fn address(extra: &[&str]) -> Vec<String> {
+    let mut v = s(&[
+        "address",
+        "--template",
+        TEMPLATE,
+        "--key",
+        &format!("@0={XPUB}"),
+        "--path",
+        "m/84'/0'/0'",
+    ]);
+    v.extend(extra.iter().map(|x| (*x).to_owned()));
+    v
+}
+
+fn closed_pipe() -> OwnedFd {
+    let mut fds = [0i32; 2];
+    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
+    unsafe { libc::close(fds[0]) }; // the reader is gone before md starts
+    unsafe { OwnedFd::from_raw_fd(fds[1]) }
+}
+
+fn on_full(a: &[String]) -> Output {
+    let full = OpenOptions::new().write(true).open("/dev/full").unwrap();
+    md().args(a)
+        .stdout(Stdio::from(full))
+        .stderr(Stdio::piped())
+        .output()
+        .unwrap()
+}
+
+fn on_closed(a: &[String], stderr_too: bool) -> Output {
+    let err = if stderr_too {
+        Stdio::from(closed_pipe())
+    } else {
+        Stdio::piped()
+    };
+    md().args(a)
+        .stdout(Stdio::from(closed_pipe()))
+        .stderr(err)
+        .output()
+        .unwrap()
+}
+
+fn full_case(a: &[String], code: i32) {
+    let o = on_full(a);
+    let err = String::from_utf8_lossy(&o.stderr);
+    assert!(!err.contains("panicked"), "{a:?}: panic: {err}");
+    assert_eq!(
+        o.status.code(),
+        Some(code),
+        "{a:?}: status {:?}: {err}",
+        o.status
+    );
+    assert_eq!(err.lines().last(), Some(FULL), "{a:?}: stderr: {err}");
+}
+
+fn closed_case(a: &[String], code: i32, stderr_too: bool) {
+    let o = on_closed(a, stderr_too);
+    let err = String::from_utf8_lossy(&o.stderr);
+    assert_eq!(
+        o.status.signal(),
+        None,
+        "{a:?}: killed by a signal: {:?}",
+        o.status
+    );
+    assert_eq!(
+        o.status.code(),
+        Some(code),
+        "{a:?}: status {:?}: {err}",
+        o.status
+    );
+    assert!(
+        !err.contains("panicked") && !err.contains("cannot write"),
+        "{a:?}: {err}"
+    );
+}
+
+#[test]
+fn full_stdout_exits_1_with_the_line() {
+    let mut rows = vec![
+        s(&["gui-schema"]),
+        s(&["encode", TEMPLATE, "--group-size", "0"]),
+        s(&["encode", "--json", TEMPLATE]),
+        s(&["decode", CARD]),
+        s(&["decode", "--json", CARD]),
+        s(&["inspect", CARD]),
+        s(&["inspect", "--json", CARD]),
+        s(&["bytecode", CARD]),
+        s(&["verify", CARD, "--template", TEMPLATE]),
+        s(&["repair", CARD]),
+        s(&["repair", "--json", CARD]),
+        s(&["shape-key", CARD]),
+        s(&[
+            "descriptor",
+            "--template",
+            TEMPLATE,
+            "--key",
+            &format!("@0={XPUB}"),
+            "--path",
+            "m/84'/0'/0'",
+        ]),
+        s(&["compose", "--wrapper", "wsh", "--path", "2of3"]),
+        s(&["compose", "--wrapper", "wsh", "--path", "2of3", "--json"]),
+        s(&[
+            "decompose",
+            &format!("wpkh([aabbccdd/84'/0'/0']{XPUB}/<0;1>/*)"),
+        ]),
+        s(&[
+            "decompose",
+            "--emit",
+            "all",
+            &format!("wpkh([aabbccdd/84'/0'/0']{XPUB}/<0;1>/*)"),
+        ]),
+        address(&["--count", "2"]),
+        address(&["--count", "2", "--json"]),
+        s(&["--help"]),
+        s(&["--version"]),
+        s(&["decode", "--help"]),
+        s(&["help", "encode"]),
+    ];
+    if cfg!(feature = "cli-compiler") {
+        rows.push(s(&["compile", "pk(@0)", "--context", "segwitv0"]));
+    }
+    for a in rows {
+        full_case(&a, 1);
+    }
+}
+
+#[test]
+fn full_stdout_after_a_repair_exits_1_not_5() {
+    let o = md().args(["repair", DAMAGED]).output().unwrap();
+    assert_eq!(o.status.code(), Some(5), "fixture: repair should exit 5");
+    full_case(&s(&["repair", DAMAGED]), 1);
+    full_case(&s(&["repair", "--json", DAMAGED]), 1);
+}
+
+#[test]
+fn full_stdout_after_a_partial_decode_exits_1_not_4() {
+    // A dead shape (no origin to resolve) decodes partially and exits 4.
+    let t = "wsh(pk(@0/<0;1>/*))";
+    let enc = md()
+        .args(["encode", "--group-size", "0", t])
+        .output()
+        .unwrap();
+    let card = String::from_utf8(enc.stdout)
+        .unwrap()
+        .lines()
+        .next()
+        .unwrap()
+        .to_owned();
+    let o = md().args(["decode", &card]).output().unwrap();
+    assert_eq!(
+        o.status.code(),
+        Some(4),
+        "fixture: partial decode should exit 4"
+    );
+    full_case(&s(&["decode", &card]), 1);
+    full_case(&s(&["inspect", &card]), 1);
+}
+
+#[test]
+fn full_stdout_with_nothing_to_print_keeps_the_verbs_code() {
+    // A verb that fails before printing never touches stdout: its own code
+    // and no "cannot write" line.
+    let a = s(&["decode", "not-a-card"]);
+    let o = on_full(&a);
+    let err = String::from_utf8_lossy(&o.stderr);
+    assert_eq!(o.status.code(), Some(1), "{err}");
+    assert!(!err.contains("cannot write"), "{err}");
+    let a = s(&["repair", "md1qqqqqqqqqqqqqqqqqqqqqqqq"]);
+    let o = on_full(&a);
+    assert_eq!(o.status.code(), Some(2));
+}
+
+#[test]
+fn closed_reader_exits_with_the_verbs_code() {
+    let mut rows = vec![
+        (s(&["gui-schema"]), 0),
+        (s(&["encode", TEMPLATE]), 0),
+        (s(&["decode", CARD]), 0),
+        (address(&["--count", "1000"]), 0),
+        (address(&["--count", "1000", "--json"]), 0),
+        (s(&["--help"]), 0),
+        (s(&["--version"]), 0),
+        (s(&["repair", DAMAGED]), 5),
+    ];
+    rows.push((s(&["decode", "--help"]), 0));
+    for (a, code) in rows {
+        closed_case(&a, code, false);
+    }
+}
+
+#[test]
+fn closed_reader_on_stdout_and_stderr_exits_0() {
+    // These verbs print on stderr with `eprintln!` (encode's policy-id line,
+    // compose's notes, a failing verb's `md: ...` line) after or before their
+    // stdout: with stderr on the closed pipe too, no panic and the verb's own
+    // code.
+    closed_case(
+        &s(&["encode", TEMPLATE, "--policy-id-fingerprint"]),
+        0,
+        true,
+    );
+    closed_case(
+        &s(&["compose", "--wrapper", "wsh", "--path", "2of3"]),
+        0,
+        true,
+    );
+    closed_case(&address(&["--count", "2"]), 0, true);
+    closed_case(&s(&["decode", CARD]), 0, true);
+    closed_case(&s(&["decode", "not-a-card"]), 1, true);
+    closed_case(&s(&["repair", "md1qqqqqqqqqqqqqqqqqqqqqqqq"]), 2, true);
+}
+
+#[test]
+fn a_stderr_that_cannot_take_text_never_changes_the_result() {
+    // Rule (c): stderr write errors are ignored. compose notes on stderr
+    // first; its card on stdout must still arrive, exit 0, whether stderr is
+    // a closed pipe or /dev/full.
+    let a = s(&["compose", "--wrapper", "wsh", "--path", "2of3"]);
+    let full = || OpenOptions::new().write(true).open("/dev/full").unwrap();
+    for err in [Stdio::from(closed_pipe()), Stdio::from(full())] {
+        let o = md().args(&a).stderr(err).output().unwrap();
+        assert_eq!(o.status.code(), Some(0), "{:?}", o.status);
+        assert!(String::from_utf8_lossy(&o.stdout).starts_with("wsh("));
+    }
+    let o = md()
+        .args(["decode", "not-a-card"])
+        .stderr(Stdio::from(full()))
+        .output()
+        .unwrap();
+    assert_eq!(o.status.code(), Some(1));
+}
+
+#[test]
+fn only_out_rs_names_stdout() {
+    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
+    let mut bad = Vec::new();
+    let mut stack = vec![dir];
+    while let Some(d) = stack.pop() {
+        for e in std::fs::read_dir(&d).unwrap() {
+            let p = e.unwrap().path();
+            if p.is_dir() {
+                stack.push(p);
+            } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("src/out.rs") {
+                let t = std::fs::read_to_string(&p).unwrap();
+                for n in ["stdout(", "std::print", "std::eprint", "core::print"] {
+                    if t.contains(n) {
+                        bad.push(format!("{}: {n}", p.display()));
+                    }
+                }
+            }
+        }
+    }
+    assert!(bad.is_empty(), "stdout named outside src/out.rs: {bad:?}");
+}
+
+#[test]
+fn every_binary_includes_the_out_module() {
+    // `src/bin/gen_ap2_grind.rs` is a second binary of the package (too slow to
+    // run here, a ~2^32 grind): it must take its print macros from `out.rs`.
+    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
+    let mut bins = vec![src.join("main.rs")];
+    for e in std::fs::read_dir(src.join("bin")).unwrap() {
+        bins.push(e.unwrap().path());
+    }
+    for p in bins {
+        let t = std::fs::read_to_string(&p).unwrap();
+        assert!(
+            t.contains("mod out;") && t.contains("#[macro_use]"),
+            "{} does not take its print macros from out.rs",
+            p.display()
+        );
+    }
+}
```

```diff
diff --git c/crates/md-cli/tests/r3_verify_against.rs i/crates/md-cli/tests/r3_verify_against.rs
index c53c017..124b390 100644
--- c/crates/md-cli/tests/r3_verify_against.rs
+++ i/crates/md-cli/tests/r3_verify_against.rs
@@ -412,3 +412,40 @@ fn r3_an_empty_verify_against_file_names_verify_against_not_in() {
         "the flag the operator never typed must not appear: {stderr}"
     );
 }
+
+// ─── (g) a NOT-equal verdict whose stdout is lost (output-sink rule) ────────
+
+/// The 5 carries a result (the composition on stdout): when stdout cannot take
+/// it, the exit is 1 with `md: cannot write the output: …`, and the verdict
+/// line on stderr is still printed.
+#[cfg(target_os = "linux")]
+#[test]
+fn r3_a_not_equal_verdict_whose_stdout_is_lost_exits_1_and_keeps_the_verdict_line() {
+    let (_, x1) = key_record(1);
+    let (_, x2) = key_record(2);
+    let (_, x3) = key_record(3);
+    let card_a = mint_two_of_two("73c5da0a", &x1, &x2);
+    let card_b = mint_two_of_two("73c5da0a", &x1, &x3);
+    let dir = tempfile::tempdir().unwrap();
+    let target = write_lines(dir.path(), "card_b.txt", &card_b);
+
+    let full = std::fs::OpenOptions::new()
+        .write(true)
+        .open("/dev/full")
+        .unwrap();
+    let o = std::process::Command::new(env!("CARGO_BIN_EXE_md"))
+        .arg("descriptor")
+        .args(&card_a)
+        .args(["--verify-against", target.to_str().unwrap()])
+        .stdout(std::process::Stdio::from(full))
+        .output()
+        .unwrap();
+    let err = err_of(&o);
+    assert_eq!(o.status.code(), Some(1), "{err}");
+    assert!(err.contains(&not_equal_line("VALUES")), "{err}");
+    assert_eq!(
+        err.lines().last(),
+        Some("md: cannot write the output: No space left on device (os error 28)"),
+        "{err}"
+    );
+}
```


```sh
cargo nextest run --locked -j 8 -p md-cli -E 'test(/(^|::)every_binary_includes_the_out_module$/)' ## expect: does not take its print macros from out.rs
cargo nextest run --locked -j 8 -p md-cli -E 'test(/(^|::)closed_reader_exits_with_the_verbs_code$/)' ## expect: failed printing to stdout: Broken pipe (os error 32)
cargo nextest run --locked -j 8 -p md-cli -E 'test(/(^|::)a_stderr_that_cannot_take_text_never_changes_the_result$/)' ## expect: ExitStatus(unix_wait_status(25856))
cargo nextest run --locked -j 8 -p md-cli -E 'test(/(^|::)full_stdout_exits_1_with_the_line$/)' ## expect: failed printing to stdout: No space left on device (os error 28)
cargo nextest run --locked -j 8 -p md-cli -E 'test(/(^|::)full_stdout_after_a_repair_exits_1_not_5$/)' ## expect: failed printing to stdout: No space left on device (os error 28)
cargo nextest run --locked -j 8 -p md-cli -E 'test(/(^|::)closed_reader_on_stdout_and_stderr_exits_0$/)' ## expect: ["encode", "wpkh(@0/<0;1>/*)", "--policy-id-fingerprint"]: status ExitStatus(unix_wait_status(25856))
cargo nextest run --locked -j 8 -p md-cli -E 'test(/(^|::)full_stdout_after_a_partial_decode_exits_1_not_4$/)' ## expect: failed printing to stdout: No space left on device (os error 28)
cargo nextest run --locked -j 8 -p md-cli -E 'test(/(^|::)r3_a_not_equal_verdict_whose_stdout_is_lost_exits_1_and_keeps_the_verdict_line$/)' ## expect: failed printing to stdout: No space left on device (os error 28)
```

| row | expected after GREEN | on the base code (measured by the grade, `red.log`) |
|---|---|---|
| `every_binary_includes_the_out_module` | `main.rs` and `src/bin/gen_ap2_grind.rs` each hold `mod out;` and `#[macro_use]` | FAIL: `.../src/main.rs does not take its print macros from out.rs` |
| `closed_reader_exits_with_the_verbs_code` | nine invocations exit with the verb's code (0; 5 for `repair`), no signal, no `panicked`, no `cannot write` | FAIL: `["gui-schema"]: status ExitStatus(unix_wait_status(25856))` / `failed printing to stdout: Broken pipe (os error 32)`, left `Some(101)`, right `Some(0)` |
| `a_stderr_that_cannot_take_text_never_changes_the_result` | `compose` exits 0 with its card on stdout whether stderr is a closed pipe or `/dev/full`; `decode not-a-card` with stderr on `/dev/full` exits 1 | FAIL: `ExitStatus(unix_wait_status(25856))`, left `Some(101)`, right `Some(0)` |
| `full_stdout_exits_1_with_the_line` | 23 invocations (24 with `cli-compiler`) each exit 1 with the line last on stderr, no panic | FAIL: `["gui-schema"]: panic:` / `failed printing to stdout: No space left on device (os error 28)` |
| `full_stdout_after_a_repair_exits_1_not_5` | the fixture exits 5 on a good stdout; on a full stdout exit 1 (text and `--json`) | FAIL: the same panic (`["repair", "md1yqpqqxqq8xtwhw4xwn4qx"]`) |
| `closed_reader_on_stdout_and_stderr_exits_0` | six invocations, stdout and stderr both closed: the verb's own code (0, 0, 0, 0, 1, 2), no signal, no panic | FAIL: `["encode", "wpkh(@0/<0;1>/*)", "--policy-id-fingerprint"]: status ExitStatus(unix_wait_status(25856))` |
| `full_stdout_after_a_partial_decode_exits_1_not_4` | the fixture exits 4 on a good stdout; `decode` and `inspect` on a full stdout exit 1 | FAIL: the same panic (`["decode", "md1yqpqqxpzsn9favzyjwfcgx"]`) |
| `r3_a_not_equal_verdict_whose_stdout_is_lost_exits_1_and_keeps_the_verdict_line` | exit 1; stderr keeps the `NOT spend-equal` verdict line | FAIL: left `Some(101)`, right `Some(1)` |
| `only_out_rs_names_stdout` | PASS | PASS on the base too: a source-scan guard, not a RED row. Its teeth are plants P7 and P8 (below). |
| `full_stdout_with_nothing_to_print_keeps_the_verbs_code` | PASS | PASS on the base too: a guard (a verb that fails before printing never touches stdout). Kills nothing alone; it protects against a "every code becomes 1" over-reach together with the unit test. |

Whole-file RED (grade): **10 tests run: 2 passed, 8 failed** (the two rows
files together, the filter in section 7). The unit test
`out::tests::a_closed_pipe_keeps_every_code_and_a_write_error_turns_success_into_1`
lives in `out.rs` (block 3.4), so it does not exist on the base; it is GREEN
only (one test per binary, because the statics cannot be reset: a closed pipe first, then
a write error). It is killed by P1, P2, P3, P3b, P3c and P6.

### Tests can fail (the grade's 14 plants)

Each plant was made against the fix; each fails at least one row (grade
`plants.txt`, 14 of 14 die; the gate does not re-run them).

| plant | what is planted | fails |
|---|---|---|
| P1 | BrokenPipe is recorded as a write error | `closed_reader_exits_with_the_verbs_code`, `closed_reader_on_stdout_and_stderr_exits_0`, the unit test |
| P2 | no mapping to 1 for any success-with-output code | `full_stdout_after_a_partial_decode_exits_1_not_4`, `full_stdout_after_a_repair_exits_1_not_5`, `full_stdout_exits_1_with_the_line`, the unit test, the `r3_...` row |
| P3 | `repair`'s and verify-against's 5 is kept | `full_stdout_after_a_repair_exits_1_not_5`, the unit test, the `r3_...` row |
| P3b | partial decode's 4 is kept | `full_stdout_after_a_partial_decode_exits_1_not_4`, the unit test |
| P3c | 0 is kept | `full_stdout_exits_1_with_the_line`, the unit test |
| P4 | `--help`/`--version` not routed through the sink | `full_stdout_exits_1_with_the_line` |
| P5 | a std `eprintln!` | `a_stderr_that_cannot_take_text_never_changes_the_result`, `closed_reader_on_stdout_and_stderr_exits_0` |
| P6 | every code becomes 1 | the unit test |
| P7 | `std::io::stdout()` named in another file | `only_out_rs_names_stdout` |
| P8 | a std `println!` site | `closed_reader_exits_with_the_verbs_code`, `closed_reader_on_stdout_and_stderr_exits_0`, `full_stdout_exits_1_with_the_line`, `only_out_rs_names_stdout` |
| P9 | no `cannot write` line printed | `full_stdout_after_a_partial_decode_exits_1_not_4`, `full_stdout_after_a_repair_exits_1_not_5`, `full_stdout_exits_1_with_the_line`, the `r3_...` row |
| P10 | `gen_ap2_grind` not routed through `out.rs` | `every_binary_includes_the_out_module` |
| P11 | a stderr write error is recorded | `a_stderr_that_cannot_take_text_never_changes_the_result` |
| P12 | `Cli::parse()` instead of `try_parse()` | `full_stdout_exits_1_with_the_line` |

(The names are the plants' labels in `plants.txt`; the failing sets are its
rows, abbreviated. The `r3_...` row is
`r3_a_not_equal_verdict_whose_stdout_is_lost_exits_1_and_keeps_the_verdict_line`.)

## 5. Tests that change

None changes. One file is added, `crates/md-cli/tests/output_errors.rs`, and
one test function is appended to `crates/md-cli/tests/r3_verify_against.rs`
(section 4). The triage's rows are not committed. The unit test is part of
`out.rs` (block 3.4).

## 6. Build order

1. Add the rows (section 4) as their own commit; run the RED commands: eight FAIL for the stated reasons, the two guards PASS.
2. Apply sections 3.1 to 3.5 (the grade's diff). Run the rows and `out::tests`: GREEN.
3. Fill S2 (the `<COMMIT>` of `md-output-sink`) as the proposal says, or as the coordinator rules.
4. Gate (section 7).
5. Toolkit manual section (section 8): not part of this PR; it goes in the toolkit PR that pins the first md-cli release carrying this fix (S1, ruled).
6. CI on the PR must show the legs the grade did not run (section 9).

## 7. Gate

`scripts/md-output-sink-fix-gate.sh` runs the whole of it: it extracts this
document's blocks, checks the code blocks and the rows against the sha256 the
document states (and `cmp`s the grade's files when they exist), runs every RED
row as printed on the base, applies the code blocks, shows the rows and the
full md-cli suite green twice, and has a self-test that goes red on a broken
row. Its header states what it does not cover. The commands it runs, each to a
file, then grep:

- `cargo nextest run --locked -j 8 -p md-cli --features cli-compiler`
  expecting **942 tests run: 942 passed, 3 skipped** (grade, `green-final.log`).
- `cargo nextest run --locked -j 8 -p md-cli` (default features) expecting
  **904 tests run: 904 passed, 3 skipped** (grade, `green-default.log`).
- `cargo nextest run --locked -j 8 -p md-cli -E 'binary(output_errors) | test(/(^|::)r3_a_not_equal_verdict_whose_stdout_is_lost_exits_1_and_keeps_the_verdict_line$/)'`:
  **10 tests run: 10 passed** (a short or empty count is a FAIL); on the base,
  **10 tests run: 2 passed, 8 failed**.
- `cargo nextest run --locked -j 8 -p md-cli -E 'test(/^out::tests::/)'`: **2 tests run: 2 passed** (the unit test runs once in the `md` binary and once in `gen_ap2_grind`, which includes `out.rs`).
- `cargo test --locked --workspace --doc --all-features`
- `cargo fmt --all --check` (the pinned 1.85.0 toolchain; the grade ran `cargo +stable fmt`)
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
- `bash ci/code-docs-check.sh` (the clippy job's "no code names a docs file" step: the new test files and the gate script must not name a docs file as a whole name).
- Then `coordinator/scripts/mutation-gate.sh <worktree> <base>` before any review (not in the script).

## 8. Toolkit manual lockstep (42-md.md)

The text goes in the toolkit PR that moves `scripts/install.sh`'s md pin
(`descriptor-mnemonic-md-cli-v0.20.4`, L55 on toolkit master `f1388e8c`) to the
first md-cli release carrying this fix (0.20.6 or later). It goes in the same
commit as that pin and the `42-md.md` install line, and it may not merge
before that pin. It is plain text, no `(unreleased: ...)` marker (the lint
reads that marker only on flag rows, so a behaviour marker could never fail).
It is not in the release-F1 PR, which pins md 0.20.5, which does not have the
fix, unless F1 is built after a fixed md release ships and pins it: then F1
carries it.

Insert in toolkit `docs/manual/src/40-cli-reference/42-md.md`, after its last
line (today L791, the `1` row of the exit-code table): a blank line, `---`, a
blank line, `## When stdout fails`, a blank line, then this paragraph, verbatim
from the grade:

> A reader that closes the pipe early (`md address … | head -3`) only stops the output: the exit code is the verb's own. Any other write error on stdout (a full disk, `> /dev/full`), from a verb or `--help`, prints `md: cannot write the output: <error>` on stderr and exits 1; a verb that was already failing keeps its own code.

The grade ran the toolkit lint with these texts (`tk/lint-{base,mod}.log`): the
same failure set before and after (pre-existing); markdownlint and cspell pass.
No md flag or help text changes, so the mnemonic-gui md schema mirror needs no
change. The stderr line and the exit codes are md's only printed-text change.

Toolkit companion entry, as the text a toolkit FOLLOWUPS PR files (the manual
paragraph is held verbatim above):

> ### `md-manual-when-stdout-fails` — at the md re-pin carrying the output sink, `42-md.md` states what md does when stdout fails (repo: **mnemonic-toolkit**; owning phase: **the toolkit pin bump to the md-cli release carrying descriptor-mnemonic `md-output-sink`**)
> - Surfaced: coordinator agent-reports/grade-ms-md-output-errors.md section 4 (S1). Companion: descriptor-mnemonic design/FOLLOWUPS.md::md-output-sink.
> - What: insert in docs/manual/src/40-cli-reference/42-md.md, after its last line (today L791: blank, `---`, blank, `## When stdout fails`, blank), the paragraph of section 8 of descriptor-mnemonic `design/FIX_md_output_sink.md` verbatim (the paragraph above), in the same commit as the scripts/install.sh md pin. Plain text, no (unreleased: …) marker.
> - Status: `open`.
> - Tier: `docs`, `cross-repo`.

The Companion line in this repo's own entry (`md-output-sink`, block 3.5)
names that toolkit entry.

## 9. Checks recorded

- **Version:** none (section 1).
- **Evidence (grade):** RED `10 tests run: 2 passed, 8 failed` (`red.log`); GREEN `-p md-cli --features cli-compiler` 942 run, 942 passed, 3 skipped (`green-final.log`, rerun after the plants) and default features 904 passed, 3 skipped (`green-default.log`); `cargo clippy --workspace --all-targets --all-features -D warnings` rc 0; `cargo +stable fmt --all --check` rc 0; `cargo test --workspace --doc --all-features` rc 0; `cargo test ... out::tests` rc 0; 14 of 14 plants die. On the fixed binary, `compose ... 2>/dev/full` gave 0 of 100 nonzero and `2>&1 | head -1` 0 of 100 (on the base: 100 of 100 and 1 of 100; `freq-stderr.txt`).
- **Not run by the grade, so CI on the PR must show them** (`.github/workflows/`; the rows are Linux-only; `out.rs` is std-only and std maps Windows' closed-pipe errors to BrokenPipe):
  1. `ci.yml` `cargo test (macos-latest)` and the advisory `cargo test (windows-latest)` (built on a ready PR): `cargo nextest run --workspace --all-targets --all-features --no-fail-fast`. The Windows leg must confirm std's closed-pipe mapping; there is no Windows row, so read it first.
  2. `ci.yml` `cargo test (ubuntu-latest)` (the required context): `cargo nextest run --workspace --all-targets --all-features --no-fail-fast` (the gate runs `-p md-cli` twice, not the whole workspace); plus `cargo test --workspace --doc --all-features`.
  3. `ci.yml` `cargo doc`: `cargo doc --workspace --no-deps --document-private-items --all-features` (the new `//!` and `///` comments are rustdoc).
  4. `ci.yml` `freebsd compile-gate`: `cargo check --target x86_64-unknown-freebsd -p md-cli`.
  5. `ci.yml` `musl-check`: x86_64 `cargo nextest run -p md-cli` and aarch64 `cargo nextest run --release -p md-cli` (the rows run in release mode there: overflow checks off).
  6. `ci.yml` `clippy` and `fmt` run on the head as the gate ran them; its docs-only and code-docs steps (`ci/`) are in the gate.
  7. Other workflows: `bitcoind-differential.yml` (md-codec only), `fuzz-smoke.yml`, `vendor-freshness.yml` (no `Cargo.lock` or `vendor/` change) and `man-pages.yml` (builds `md-cli` in release; no help text changes) are unaffected by design; CI on the PR must still show them green if they trigger.
- **Gate coverage:** see the script header; it does not re-run the 14 plants, the CI legs above, the toolkit manual, or `mutation-gate.sh`.

## 10. STOP

- **S1 (ruled 2026-11-13, coordinator `agent-reports/grade-ms-md-output-errors.md` section 4; not open).** The section 8 paragraph goes in the toolkit PR that moves `scripts/install.sh`'s md pin to the first md-cli release carrying this fix, in the same commit, and may not merge before that pin. Plain text, no marker. This PR files the cross-repo pair: descriptor-mnemonic `design/FOLLOWUPS.md::md-output-sink` (its Companion line, block 3.5) and toolkit `design/FOLLOWUPS.md::md-manual-when-stdout-fails` (section 8).
- **S2 (open; P-1 (proposed)).** The grade's FOLLOWUPS diff gives the entry `md-output-sink` the status `resolved <COMMIT>` and does not say how the `<COMMIT>` is filled. The repo's own form (`design/FOLLOWUPS.md` L44) says: change `Status:` to `resolved <COMMIT>`, where `<COMMIT>` is the short SHA of the fix. A commit cannot hold its own SHA, and block 3.5 is held byte for byte. P-1 (proposed): the build applies block 3.5 with the literal `<COMMIT>` in the code commit, and a second commit on the PR branch replaces it with the short SHA of that code commit (one line, `md-output-sink` only; `md-gen-ap2-grind-installed` stays `open`). The coordinator rules this before the build, or names another way.
