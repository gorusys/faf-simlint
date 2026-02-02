//! Shared utilities: logging, paths, bounds.

use std::path::Path;
use tracing::Level;

/// Initialize tracing with env filter. Safe to call once at startup.
pub fn init_logging(verbose: bool) {
    let level = if verbose { Level::DEBUG } else { Level::INFO };
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| level.to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

/// Normalize blueprint ID or name for lookup (lowercase, trim).
pub fn normalize_id(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Check path is under a given root and within size limit.
pub fn check_file_bounds(path: &Path, root: &Path, max_bytes: usize) -> Result<u64, String> {
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let root_canon = root.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(root_canon) {
        return Err("path escapes data directory".to_string());
    }
    let meta = std::fs::metadata(&canonical).map_err(|e| e.to_string())?;
    let size = meta.len();
    if size > max_bytes as u64 {
        return Err(format!(
            "file too large: {} bytes (max {})",
            size, max_bytes
        ));
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_id_trim_lower() {
        assert_eq!(normalize_id("  UEL0101  "), "uel0101");
        assert_eq!(normalize_id("Aeon T1 Tank"), "aeon t1 tank");
    }
}
