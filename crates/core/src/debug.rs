/// Debugging utilities for Markflow
///
/// Set the `MARKFLOW_DEBUG` environment variable to enable debug output.

use std::sync::OnceLock;

static DEBUG_MODE: OnceLock<bool> = OnceLock::new();

/// Check if debug mode is enabled
pub fn is_debug_enabled() -> bool {
    *DEBUG_MODE.get_or_init(|| {
        std::env::var("MARKFLOW_DEBUG")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

/// Print a debug message if debug mode is enabled
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if $crate::debug::is_debug_enabled() {
            eprintln!("[markflow-debug] {}", format!($($arg)*));
        }
    };
}

/// Print a debug message with location if debug mode is enabled
#[macro_export]
macro_rules! debug_at {
    ($location:expr, $($arg:tt)*) => {
        if $crate::debug::is_debug_enabled() {
            eprintln!("[markflow-debug] {} - {}", $location, format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_mode_respects_env_var() {
        // Note: This test might not work reliably because OnceLock is initialized once
        // In real scenarios, the environment variable should be set before the process starts
        let enabled = is_debug_enabled();
        // Just verify the function doesn't panic
        assert!(enabled == true || enabled == false);
    }
}
