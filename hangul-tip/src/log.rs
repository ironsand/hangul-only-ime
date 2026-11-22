//! Opt-in debug log: set HANGUL_IME_LOG=<file path> in the app's environment.
use std::io::Write;
use std::sync::OnceLock;

static PATH: OnceLock<Option<String>> = OnceLock::new();

pub fn log(msg: std::fmt::Arguments) {
    let path = PATH.get_or_init(|| std::env::var("HANGUL_IME_LOG").ok());
    if let Some(p) = path {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "[{}] {}", std::process::id(), msg);
        }
    }
}

#[macro_export]
macro_rules! dlog {
    ($($t:tt)*) => { $crate::log::log(format_args!($($t)*)) };
}
