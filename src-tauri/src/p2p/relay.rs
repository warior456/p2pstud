// src-tauri/src/p2p/relay.rs
// Helpers for building the RelayMode configuration used by the Iroh node.
// The relay URL is loaded from the app's settings database, defaulting to
// the public Iroh relay if none has been configured.

use iroh::RelayUrl;

/// Default Iroh public relay — used when no custom relay is configured.
/// Students on a university VPN will override this with the campus anchor relay.
pub const DEFAULT_RELAY_URL: &str = "https://use1-1.relay.iroh.network/";

/// Parse a relay URL string into an `iroh::RelayUrl`.
/// Returns the default relay if the input string is empty.
pub fn parse_relay_url(url_str: &str) -> anyhow::Result<RelayUrl> {
    let s = if url_str.is_empty() {
        DEFAULT_RELAY_URL
    } else {
        url_str
    };
    s.parse::<RelayUrl>().map_err(|e| anyhow::anyhow!("invalid relay URL '{}': {}", s, e))
}

