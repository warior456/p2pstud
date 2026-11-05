// src-tauri/src/p2p/relay.rs
// Helpers for building the RelayMode configuration used by the Iroh node.
// Default relay is strictly EU (euw1-1).

use iroh::{RelayMode, RelayUrl};

/// Default Iroh public relay — Western Europe (EU) relay.
pub const DEFAULT_RELAY_URL: &str = "https://euw1-1.relay.iroh.network/";

/// European primary public relay.
pub const EU_RELAY_URL: &str = "https://euw1-1.relay.iroh.network/";

/// Parse a single relay URL string into an `iroh::RelayUrl`.
/// Returns the EU default relay if the input string is empty.
pub fn parse_relay_url(url_str: &str) -> anyhow::Result<RelayUrl> {
    let s = if url_str.is_empty() || url_str.contains("use1-1") {
        DEFAULT_RELAY_URL
    } else {
        url_str
    };
    s.parse::<RelayUrl>()
        .map_err(|e| anyhow::anyhow!("invalid relay URL '{}': {}", s, e))
}

/// Build appropriate RelayMode based on configuration.
/// Only keeps the EU relay as the standard relay.
pub fn build_relay_mode(configured: &str) -> (RelayMode, String) {
    let trimmed = configured.trim();
    let target_url = if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("default")
        || trimmed.eq_ignore_ascii_case("global")
        || trimmed.eq_ignore_ascii_case("eu")
        || trimmed.contains("use1-1")
    {
        EU_RELAY_URL
    } else {
        trimmed
    };

    match parse_relay_url(target_url) {
        Ok(url) => (RelayMode::Custom(iroh::RelayMap::from(url)), target_url.to_string()),
        Err(e) => {
            tracing::warn!("Failed to parse relay URL '{}': {}. Falling back to default EU relay.", target_url, e);
            match parse_relay_url(EU_RELAY_URL) {
                Ok(url) => (RelayMode::Custom(iroh::RelayMap::from(url)), EU_RELAY_URL.to_string()),
                Err(_) => (RelayMode::Default, "Default EU Relay".to_string()),
            }
        }
    }
}
