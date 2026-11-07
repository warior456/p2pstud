// src-tauri/src/p2p/relay.rs
// Helpers for building the RelayMode configuration used by the Iroh node.
// Default relay connects to Number 0's official global relay pool with automatic lowest-latency selection (EU: euc1-1).

use iroh::{RelayMode, RelayUrl};

/// Default Iroh public relay — Western Europe (EU) relay on Iroh 1.4 infrastructure.
pub const DEFAULT_RELAY_URL: &str = "https://euc1-1.relay.n0.iroh.link/";

/// European primary public relay on Iroh 1.4 infrastructure.
pub const EU_RELAY_URL: &str = "https://euc1-1.relay.n0.iroh.link/";

/// Parse a single relay URL string into an `iroh::RelayUrl`.
/// Migrates obsolete Iroh 0.x URLs (e.g. `euw1-1.relay.iroh.network`) to the active Iroh 1.4 endpoint.
pub fn parse_relay_url(url_str: &str) -> anyhow::Result<RelayUrl> {
    let trimmed = url_str.trim();
    let s = if trimmed.is_empty()
        || trimmed.contains("iroh.network")
        || trimmed.contains("use1-1.relay")
        || trimmed.contains("euw1-1")
    {
        DEFAULT_RELAY_URL
    } else {
        trimmed
    };
    s.parse::<RelayUrl>()
        .map_err(|e| anyhow::anyhow!("invalid relay URL '{}': {}", s, e))
}

/// Build appropriate RelayMode based on configuration.
/// Defaults to `RelayMode::Default` (Number 0's full anycast relay map including EU, US, AP)
/// for automatic lowest-latency holepunching and fallback connectivity.
pub fn build_relay_mode(configured: &str) -> (RelayMode, String) {
    let trimmed = configured.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("default")
        || trimmed.eq_ignore_ascii_case("global")
    {
        return (RelayMode::Default, "Default Global Pool (Lowest Latency)".to_string());
    }

    if trimmed.eq_ignore_ascii_case("eu") || trimmed.contains("euw1-1") || trimmed.contains("euc1-1") {
        return match parse_relay_url(EU_RELAY_URL) {
            Ok(url) => (RelayMode::Custom(iroh::RelayMap::from(url)), EU_RELAY_URL.to_string()),
            Err(_) => (RelayMode::Default, "Default Global Pool (Lowest Latency)".to_string()),
        };
    }

    match parse_relay_url(trimmed) {
        Ok(url) => (RelayMode::Custom(iroh::RelayMap::from(url)), trimmed.to_string()),
        Err(e) => {
            tracing::warn!("Failed to parse custom relay URL '{}': {}. Falling back to RelayMode::Default.", trimmed, e);
            (RelayMode::Default, "Default Global Pool (Lowest Latency)".to_string())
        }
    }
}
