// src-tauri/src/p2p/relay.rs
// Helpers for building the RelayMode configuration used by the Iroh node.
// Default relay is now EU (euw1-1), with fallback support for US and global relays.

use iroh::{RelayMode, RelayUrl};

/// Default Iroh public relay — Western Europe (EU) relay.
pub const DEFAULT_RELAY_URL: &str = "https://euw1-1.relay.iroh.network/";

/// European primary public relay.
pub const EU_RELAY_URL: &str = "https://euw1-1.relay.iroh.network/";

/// US primary public relay.
pub const US_RELAY_URL: &str = "https://use1-1.relay.iroh.network/";

/// Parse a single relay URL string into an `iroh::RelayUrl`.
/// Returns the EU default relay if the input string is empty.
pub fn parse_relay_url(url_str: &str) -> anyhow::Result<RelayUrl> {
    let s = if url_str.is_empty() {
        DEFAULT_RELAY_URL
    } else {
        url_str
    };
    s.parse::<RelayUrl>()
        .map_err(|e| anyhow::anyhow!("invalid relay URL '{}': {}", s, e))
}

/// Build appropriate RelayMode based on configuration.
/// If empty, "default", or "global": RelayMode::Default uses Iroh's global relay pool (EU, US, Asia).
/// If "eu": uses the EU relay.
/// If "us": uses the US relay.
/// Otherwise, parses custom relay URL.
pub fn build_relay_mode(configured: &str) -> (RelayMode, String) {
    let trimmed = configured.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") || trimmed.eq_ignore_ascii_case("global") {
        (RelayMode::Default, "global default relays (EU & US)".to_string())
    } else if trimmed.eq_ignore_ascii_case("eu") {
        match parse_relay_url(EU_RELAY_URL) {
            Ok(url) => (RelayMode::Custom(iroh::RelayMap::from(url)), EU_RELAY_URL.to_string()),
            Err(_) => (RelayMode::Default, "global default relays (EU & US)".to_string()),
        }
    } else if trimmed.eq_ignore_ascii_case("us") {
        match parse_relay_url(US_RELAY_URL) {
            Ok(url) => (RelayMode::Custom(iroh::RelayMap::from(url)), US_RELAY_URL.to_string()),
            Err(_) => (RelayMode::Default, "global default relays (EU & US)".to_string()),
        }
    } else {
        match parse_relay_url(trimmed) {
            Ok(url) => (RelayMode::Custom(iroh::RelayMap::from(url)), trimmed.to_string()),
            Err(e) => {
                tracing::warn!("Failed to parse custom relay URL '{}': {}. Falling back to global relays.", trimmed, e);
                (RelayMode::Default, "global default relays (EU & US)".to_string())
            }
        }
    }
}
