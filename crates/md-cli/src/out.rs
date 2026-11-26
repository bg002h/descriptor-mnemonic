//! The one stdout path. Every `print!`/`println!` in md-cli resolves to the
//! macros below (declared `#[macro_use]` first in `main.rs`, so they shadow
//! std's in every module), and so do `eprint!`/`eprintln!`.
//!
//! std's macros panic (exit 101) when a write fails. Here a write error stops
//! the output instead. A reader that closed the pipe (`md … | head -3`,
//! `ErrorKind::BrokenPipe`) only stops it: the exit code is the verb's own.
//! Any other stdout error (a full disk, `> /dev/full`) is kept, and `main`
//! prints `md: cannot write the output: <error>` and adds 32 (IO) to the
//! verb's code (exit-code spec R8; crash sweep, grade-mk-crash-sweep-1 Q4). A stderr write error is ignored.
//! `tests/output_errors.rs` pins both cases and that no other file of the
//! crate names stdout.

use std::io::Write;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static CLOSED: AtomicBool = AtomicBool::new(false);
static WRITE_ERROR: OnceLock<String> = OnceLock::new();

/// Writes to stdout; after the first failure, writes nothing.
pub(crate) fn stdout_write(args: std::fmt::Arguments<'_>) {
    if CLOSED.load(Ordering::Relaxed) || read_only() {
        return;
    }
    let mut o = std::io::stdout().lock();
    if let Err(e) = o.write_fmt(args).and_then(|()| o.flush()) {
        failed(&e);
    }
}

/// std's stdout reports EBADF as a successful write, so a stdout that cannot
/// write (opened read-only, `1</dev/null`, as O_PATH, with Linux's access
/// mode 3, or closed at exec, `>&-`, see `fd_probe`) would lose the output
/// with exit 0. Checked once,
/// at the first write (or clap's `--help`/`--version`): then EBADF is recorded
/// as the write error. A verb that writes nothing to stdout is not touched.
pub(crate) fn read_only() -> bool {
    static RO: OnceLock<bool> = OnceLock::new();
    *RO.get_or_init(|| {
        #[cfg(unix)]
        {
            // SAFETY: fcntl(F_GETFL) takes no pointer; a bad fd returns -1.
            let fl = unsafe { libc::fcntl(1, libc::F_GETFL) };
            let ro = STDOUT_WAS_CLOSED.load(Ordering::Relaxed)
                || (fl >= 0 && !matches!(fl & libc::O_ACCMODE, libc::O_WRONLY | libc::O_RDWR));
            if ro {
                failed(&std::io::Error::from_raw_os_error(libc::EBADF));
            }
            ro
        }
        // Windows: std turns only an invalid handle into a success, so a
        // read-only handle's error reaches the sink as written.
        #[cfg(not(unix))]
        {
            false
        }
    })
}

/// Set when fd 1 was closed at exec (`md … >&-`).
static STDOUT_WAS_CLOSED: AtomicBool = AtomicBool::new(false);

/// A closed fd 1 cannot be seen from `main`: std's runtime opens `/dev/null`
/// on every closed fd 0-2 before `main` runs, and the output then vanishes
/// with exit 0. So this runs earlier, as a load-time constructor: it records
/// a closed fd 1 (then `read_only` reports EBADF at the first stdout write)
/// and itself puts `/dev/null` on each closed fd 0-2, in order, so no later
/// `open` (an `--out` file, say) can take a standard fd number.
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
    target_os = "macos",
    target_os = "ios"
))]
mod fd_probe {
    #[used]
    #[cfg_attr(
        any(target_os = "macos", target_os = "ios"),
        unsafe(link_section = "__DATA,__mod_init_func")
    )]
    #[cfg_attr(
        not(any(target_os = "macos", target_os = "ios")),
        unsafe(link_section = ".init_array")
    )]
    static PROBE: extern "C" fn() = probe;

    extern "C" fn probe() {
        for fd in 0..=2 {
            // SAFETY: fcntl(F_GETFD) takes no pointer; a closed fd returns -1
            // with EBADF.
            let closed = unsafe { libc::fcntl(fd, libc::F_GETFD) } == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::EBADF);
            if !closed {
                continue;
            }
            if fd == 1 {
                super::STDOUT_WAS_CLOSED.store(true, super::Ordering::Relaxed);
            }
            // SAFETY: a NUL-terminated path; the fd is left open on purpose.
            // open returns the lowest free fd, which is `fd` (lower ones are
            // open by now); anything else is closed again and std's own
            // /dev/null fallback takes over.
            let got = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_RDWR) };
            if got >= 0 && got != fd {
                // SAFETY: `got` is the fd just opened here.
                unsafe { libc::close(got) };
            }
        }
    }
}

/// Records a failed stdout write (also used for clap's `--help`/`--version`).
pub(crate) fn failed(e: &std::io::Error) {
    CLOSED.store(true, Ordering::Relaxed);
    if e.kind() != std::io::ErrorKind::BrokenPipe {
        let _ = WRITE_ERROR.set(e.to_string());
    }
}

/// Writes to stderr, ignoring a failure.
pub(crate) fn stderr_write(args: std::fmt::Arguments<'_>) {
    let _ = std::io::stderr().lock().write_fmt(args);
}

/// The process exit code: `code`, unless a stdout write failed for a reason
/// other than a closed pipe. Then the error is printed and 32 (IO) is ADDED to
/// the verb's bits (exit-code spec R8): it does not replace them. 64 (usage)
/// and 101 (internal) never combine and are kept as they are.
pub(crate) fn exit_code(code: u8) -> u8 {
    match WRITE_ERROR.get() {
        None => code,
        Some(e) => {
            stderr_write(format_args!("md: cannot write the output: {e}\n"));
            if code < crate::exit::USAGE {
                (crate::exit::Bits::of(code) | crate::exit::IO).to_exit_code()
            } else {
                code
            }
        }
    }
}

macro_rules! print {
    ($($a:tt)*) => { $crate::out::stdout_write(format_args!($($a)*)) };
}
// Unused in `src/bin/gen_ap2_grind.rs`, which includes this file.
#[allow(unused_macros)]
macro_rules! println {
    () => { $crate::out::stdout_write(format_args!("\n")) };
    ($($a:tt)*) => { $crate::out::stdout_write(format_args!("{}\n", format_args!($($a)*))) };
}
// Unused today; defined so a future `eprint!` cannot reach std's.
#[allow(unused_macros)]
macro_rules! eprint {
    ($($a:tt)*) => { $crate::out::stderr_write(format_args!($($a)*)) };
}
macro_rules! eprintln {
    () => { $crate::out::stderr_write(format_args!("\n")) };
    ($($a:tt)*) => { $crate::out::stderr_write(format_args!("{}\n", format_args!($($a)*))) };
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test, because the statics cannot be reset: a closed pipe first
    // (every code kept), then a write error (32 added to every bit sum; 64 and
    // 101 kept). Two tests would share the statics under `cargo test`.
    #[test]
    fn a_closed_pipe_keeps_every_code_and_a_write_error_adds_io() {
        failed(&std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        for c in [0u8, 1, 2, 4, 16, 20, 64, 101] {
            assert_eq!(exit_code(c), c, "closed pipe, code {c}");
        }
        failed(&std::io::Error::from_raw_os_error(28));
        for (c, want) in [(0u8, 32u8), (1, 33), (2, 34), (4, 36), (16, 48), (20, 52)] {
            assert_eq!(exit_code(c), want, "write error, code {c}");
        }
        for single in [64u8, 101] {
            assert_eq!(exit_code(single), single, "single code {single}");
        }
    }
}
