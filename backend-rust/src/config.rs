//! Application configuration constants.

use std::path::PathBuf;

/// Maximum timer value in seconds (1 hour).
pub const TIMER_MAX_SECONDS: i64 = 3600;

/// APNS sandbox host. Use `https://api.push.apple.com` for production.
pub const APNS_HOST: &str = "https://api.sandbox.push.apple.com";
pub const APNS_TEAM_ID: &str = "WVCM8HLGRN";
pub const APNS_BUNDLE_ID: &str = "dev.bytealigned.DeathsDoor";

/// Candidate glob patterns for the APNS auth key, in priority order.
pub fn apns_key_globs() -> Vec<PathBuf> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut globs = vec![
        // backend-rust/keys/AuthKey_*.p8
        crate_root.join("keys").join("AuthKey_*.p8"),
    ];
    // Repo-root keys/AuthKey_*.p8 (where the key has historically lived).
    if let Some(repo_root) = crate_root.parent() {
        globs.push(repo_root.join("keys").join("AuthKey_*.p8"));
    }
    globs
}
