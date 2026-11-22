//! Debug-build-only log: set HANGUL_IME_LOG=<file path> in the app's environment.
//!
//! The log records keystrokes, so it must never be compiled into release builds. In release
//! builds `dlog!` expands to dead code (arguments are type-checked but nothing is written) and
//! the environment variable is never read.

#[cfg(debug_assertions)]
pub fn log(msg: std::fmt::Arguments) {
    use std::io::Write;
    use std::sync::OnceLock;

    static PATH: OnceLock<Option<String>> = OnceLock::new();
    let path = PATH.get_or_init(|| std::env::var("HANGUL_IME_LOG").ok());
    if let Some(p) = path {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "[{}] {}", std::process::id(), msg);
        }
    }
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! dlog {
    ($($t:tt)*) => { $crate::log::log(format_args!($($t)*)) };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! dlog {
    ($($t:tt)*) => {
        if false {
            let _ = format_args!($($t)*);
        }
    };
}
