use std::sync::OnceLock;
use std::env;

static GC_DEBUG_ENABLED: OnceLock<bool> = OnceLock::new();
static GC_TRACE_ENABLED: OnceLock<bool> = OnceLock::new();

/// Check if GC_DEBUG environment variable is set and print the message if it is.
/// This function caches the environment variable check on first call.
#[inline]
pub fn gc_debug(msg: &str) {
    let enabled = *GC_DEBUG_ENABLED.get_or_init(|| env::var("GC_DEBUG").is_ok());
    if enabled {
        println!("GC_DEBUG: {}", msg);
    }
}

/// Check if GC_TRACE environment variable is set and print the message if it is.
/// This function caches the environment variable check on first call.
#[inline]
pub fn gc_trace(msg: &str) {
    let enabled = *GC_TRACE_ENABLED.get_or_init(|| env::var("GC_TRACE").is_ok());
    if enabled {
        println!("GC_TRACE: {}", msg);
    }
}
