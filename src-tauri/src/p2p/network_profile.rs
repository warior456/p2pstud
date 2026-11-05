// src-tauri/src/p2p/network_profile.rs
// Campus Wi-Fi Detection & Eduroam Stealth Mode state machine and SSID rules.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{Datelike, Local, Timelike, Weekday};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::storage::db::Database;

pub const MAX_CAMPUS_BACKGROUND_BPS: u64 = 512 * 1024; // 512 KB/s
pub const MAX_LECTURE_BACKGROUND_BPS: u64 = 128 * 1024; // 128 KB/s
pub const LEDBAT_RTT_THRESHOLD_MS: u64 = 150;

static CACHED_SSID: RwLock<Option<(Option<String>, Instant)>> = RwLock::new(None);
const CACHE_TTL_SECS: u64 = 60;

/// Return cached SSID if valid, otherwise detect fresh SSID and update cache.
pub fn get_cached_or_detect_ssid() -> Option<String> {
    if let Ok(guard) = CACHED_SSID.read() {
        if let Some((ref ssid, instant)) = *guard {
            if instant.elapsed().as_secs() < CACHE_TTL_SECS {
                return ssid.clone();
            }
        }
    }
    let fresh = detect_os_ssid();
    if let Ok(mut guard) = CACHED_SSID.write() {
        *guard = Some((fresh.clone(), Instant::now()));
    }
    fresh
}

/// Invalidate cached SSID to force an immediate re-detection.
pub fn invalidate_ssid_cache() {
    if let Ok(mut guard) = CACHED_SSID.write() {
        *guard = None;
    }
}

/// Block the calling thread until a native OS network change event occurs.
/// Uses `NotifyAddrChange` on Windows, and Netlink route socket on Linux.
pub fn wait_for_os_network_change() {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "iphlpapi")]
        extern "system" {
            fn NotifyAddrChange(
                handle: *mut std::ffi::c_void,
                overlapped: *mut std::ffi::c_void,
            ) -> u32;
        }

        unsafe {
            // Blocks until an IP address or interface change occurs on the system
            let _ = NotifyAddrChange(std::ptr::null_mut(), std::ptr::null_mut());
        }
    }

    #[cfg(target_os = "linux")]
    {
        #[repr(C)]
        struct SockAddrNl {
            nl_family: u16,
            nl_pad: u16,
            nl_pid: u32,
            nl_groups: u32,
        }

        extern "C" {
            fn socket(domain: i32, type_: i32, protocol: i32) -> i32;
            fn bind(sockfd: i32, addr: *const SockAddrNl, addrlen: u32) -> i32;
            fn recv(sockfd: i32, buf: *mut u8, len: usize, flags: i32) -> isize;
            fn close(fd: i32) -> i32;
        }

        unsafe {
            // AF_NETLINK = 16, SOCK_RAW = 3, NETLINK_ROUTE = 0
            let fd = socket(16, 3, 0);
            if fd >= 0 {
                let addr = SockAddrNl {
                    nl_family: 16,
                    nl_pad: 0,
                    nl_pid: 0,
                    nl_groups: 1 | 0x10 | 0x100, // RTMGRP_LINK | RTMGRP_IPV4_IFADDR | RTMGRP_IPV6_IFADDR
                };
                if bind(fd, &addr, std::mem::size_of::<SockAddrNl>() as u32) == 0 {
                    let mut buf = [0u8; 4096];
                    let _ = recv(fd, buf.as_mut_ptr(), buf.len(), 0);
                }
                close(fd);
            } else {
                std::thread::sleep(Duration::from_secs(60));
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkProfile {
    Campus,
    Trusted,
}

impl NetworkProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkProfile::Campus => "campus",
            NetworkProfile::Trusted => "trusted",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "campus" => Some(NetworkProfile::Campus),
            "trusted" => Some(NetworkProfile::Trusted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkProfileInfo {
    pub profile: String,
    pub ssid: Option<String>,
    pub lecture_throttle_active: bool,
    pub override_active: Option<String>,
    pub background_cap_bps: u64,
    pub is_known_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsidRule {
    pub ssid: String,
    pub profile: String,
    pub added_at: i64,
}

/// Detect the currently connected Wi-Fi SSID using native OS commands without shell invocation.
pub fn detect_os_ssid() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = std::process::Command::new("netsh")
            .args(["wlan", "show", "interfaces"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            // Match "SSID                   : MyNetwork" but NOT "BSSID"
            if trimmed.starts_with("SSID") && !trimmed.starts_with("BSSID") {
                if let Some((_, val)) = trimmed.split_once(':') {
                    let ssid = val.trim();
                    if !ssid.is_empty() {
                        return Some(ssid.to_string());
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("iwgetid")
            .arg("-r")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let ssid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if ssid.is_empty() {
            None
        } else {
            Some(ssid)
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        None
    }
}

/// Check if lecture hour throttling applies (Monday-Friday 09:00 - 17:00 local time).
pub fn is_lecture_hours_at(weekday: Weekday, hour: u32) -> bool {
    let is_weekday = matches!(
        weekday,
        Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
    );
    let is_hour = (9..17).contains(&hour);
    is_weekday && is_hour
}

/// Check if current local time is within lecture hours.
pub fn is_current_lecture_hours() -> bool {
    let now = Local::now();
    is_lecture_hours_at(now.weekday(), now.hour())
}

/// Look up a profile rule for an SSID in SQLite.
pub fn get_rule_for_ssid(db: &Database, ssid: &str) -> Result<Option<NetworkProfile>> {
    let conn = db.lock();
    let profile_str: Result<String, rusqlite::Error> = conn.query_row(
        "SELECT profile FROM network_ssid_rules WHERE ssid = ?1",
        params![ssid],
        |r| r.get(0),
    );

    match profile_str {
        Ok(s) => Ok(NetworkProfile::from_str_opt(&s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Check if an SSID has an explicit classification rule stored in the database.
pub fn is_ssid_classified(db: &Database, ssid: &str) -> bool {
    get_rule_for_ssid(db, ssid).unwrap_or(None).is_some()
}

/// Resolve the active profile based on manual override or detected SSID.
pub fn resolve_network_profile(
    db: &Database,
    current_ssid: Option<&str>,
) -> (NetworkProfile, Option<String>) {
    if let Ok(Some(ov)) = db.get_setting("network_profile_override") {
        let ov_trimmed = ov.trim().to_lowercase();
        if ov_trimmed == "campus" {
            return (NetworkProfile::Campus, Some("campus".to_string()));
        } else if ov_trimmed == "trusted" {
            return (NetworkProfile::Trusted, Some("trusted".to_string()));
        }
    }

    if let Some(ssid) = current_ssid {
        if let Ok(Some(profile)) = get_rule_for_ssid(db, ssid) {
            return (profile, None);
        }
    }

    // Default to Campus (conservative stealth mode) if unknown or unclassified
    (NetworkProfile::Campus, None)
}

/// Check what discovery flags are permitted for a given profile.
pub fn is_discovery_allowed(profile: NetworkProfile) -> (bool, bool) {
    match profile {
        NetworkProfile::Campus => (false, false), // DHT disabled, mDNS disabled
        NetworkProfile::Trusted => (true, true),  // DHT enabled, mDNS enabled
    }
}

/// Returns the background bandwidth limit (bytes per second) for a network profile.
/// On Campus networks during lecture hours (Mon-Fri 09:00-17:00), limits to MAX_LECTURE_BACKGROUND_BPS (128 KB/s) by default.
/// On Campus networks outside lecture hours, limits to MAX_CAMPUS_BACKGROUND_BPS (512 KB/s) by default.
/// On Trusted networks, returns None (unlimited).
pub fn get_background_bandwidth_limit(db: &Database, profile: NetworkProfile) -> Option<u64> {
    get_background_bandwidth_limit_at(db, profile, is_current_lecture_hours())
}

/// Calculate background bandwidth limit based on profile and whether lecture hours are active.
pub fn get_background_bandwidth_limit_at(
    db: &Database,
    profile: NetworkProfile,
    is_lecture: bool,
) -> Option<u64> {
    match profile {
        NetworkProfile::Campus => {
            if is_lecture {
                let custom = db
                    .get_setting("lecture_bandwidth_cap_bps")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<u64>().ok());
                Some(custom.unwrap_or(MAX_LECTURE_BACKGROUND_BPS))
            } else {
                let custom = db
                    .get_setting("campus_bandwidth_cap_bps")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<u64>().ok());
                Some(custom.unwrap_or(MAX_CAMPUS_BACKGROUND_BPS))
            }
        }
        NetworkProfile::Trusted => {
            db.get_setting("trusted_bandwidth_cap_bps")
                .ok()
                .flatten()
                .and_then(|s| s.parse::<u64>().ok())
        }
    }
}

/// Add or update an SSID rule.
pub fn save_ssid_rule(db: &Database, ssid: &str, profile: &str) -> Result<()> {
    let valid_profile = match profile.to_lowercase().as_str() {
        "campus" => "campus",
        "trusted" => "trusted",
        _ => anyhow::bail!("Invalid profile: must be 'campus' or 'trusted'"),
    };

    let conn = db.lock();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO network_ssid_rules(ssid, profile, added_at)
         VALUES(?1, ?2, ?3)
         ON CONFLICT(ssid) DO UPDATE SET profile = excluded.profile, added_at = excluded.added_at",
        params![ssid.trim(), valid_profile, now],
    )?;
    Ok(())
}

/// List all saved SSID rules.
pub fn list_all_ssid_rules(db: &Database) -> Result<Vec<SsidRule>> {
    let conn = db.lock();
    let mut stmt = conn.prepare("SELECT ssid, profile, added_at FROM network_ssid_rules ORDER BY added_at DESC")?;
    let rows = stmt.query_map([], |r| {
        Ok(SsidRule {
            ssid: r.get(0)?,
            profile: r.get(1)?,
            added_at: r.get(2)?,
        })
    })?;

    let mut rules = Vec::new();
    for row in rows {
        rules.push(row?);
    }
    Ok(rules)
}

/// Delete an SSID rule.
pub fn delete_ssid_rule_db(db: &Database, ssid: &str) -> Result<()> {
    let conn = db.lock();
    conn.execute("DELETE FROM network_ssid_rules WHERE ssid = ?1", params![ssid])?;
    Ok(())
}

/// Background monitor task that subscribes to native OS network change events (Windows & Linux)
/// and emits 'network-new-ssid' and 'network-profile-changed' events to the frontend.
pub fn spawn_network_profile_monitor(db: Arc<Database>, app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_detected_ssid = get_cached_or_detect_ssid();

        loop {
            // Block waiting for native OS network change event
            wait_for_os_network_change();

            // Debounce for 1 second in case the OS fires multiple interface/IP assignment events
            std::thread::sleep(Duration::from_millis(1000));

            // Invalidate cache and detect fresh SSID
            invalidate_ssid_cache();
            let current_ssid = detect_os_ssid();

            if current_ssid != last_detected_ssid {
                tracing::info!(old = ?last_detected_ssid, new = ?current_ssid, "OS network change detected");
                last_detected_ssid = current_ssid.clone();

                if let Some(ref ssid) = current_ssid {
                    // Check if unrecognized
                    if let Ok(None) = get_rule_for_ssid(&db, ssid) {
                        #[derive(Clone, Serialize)]
                        struct NewSsidPayload {
                            ssid: String,
                        }
                        let _ = app.emit("network-new-ssid", NewSsidPayload { ssid: ssid.clone() });
                    }
                }

                // Broadcast updated profile info so frontend UI updates reactively without polling
                let (profile, override_active) = resolve_network_profile(&db, current_ssid.as_deref());
                let is_lecture = is_current_lecture_hours();
                let cap = match profile {
                    NetworkProfile::Campus => {
                        if is_lecture {
                            MAX_LECTURE_BACKGROUND_BPS
                        } else {
                            MAX_CAMPUS_BACKGROUND_BPS
                        }
                    }
                    NetworkProfile::Trusted => u64::MAX,
                };

                let is_known = current_ssid
                    .as_ref()
                    .map(|s| is_ssid_classified(&db, s))
                    .unwrap_or(true)
                    || override_active.is_some();

                let _ = app.emit(
                    "network-profile-changed",
                    NetworkProfileInfo {
                        profile: profile.as_str().to_string(),
                        ssid: current_ssid,
                        lecture_throttle_active: is_lecture && profile == NetworkProfile::Campus,
                        override_active,
                        background_cap_bps: cap,
                        is_known_network: is_known,
                    },
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_db() -> (TempDir, Arc<Database>) {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(&dir.path().join("test.db")).unwrap());
        (dir, db)
    }

    #[test]
    fn test_eduroam_stealth_disables_dht_and_mdns() {
        let (_dir, db) = make_test_db();
        // Eduroam is pre-seeded as 'campus' in migration 0007
        let profile = get_rule_for_ssid(&db, "eduroam").unwrap();
        assert_eq!(profile, Some(NetworkProfile::Campus));

        let (dht, mdns) = is_discovery_allowed(profile.unwrap());
        assert!(!dht, "DHT must be disabled in campus stealth mode");
        assert!(!mdns, "mDNS must be disabled in campus stealth mode");
    }

    #[test]
    fn test_trusted_profile_enables_discovery() {
        let (dht, mdns) = is_discovery_allowed(NetworkProfile::Trusted);
        assert!(dht, "DHT must be enabled in trusted mode");
        assert!(mdns, "mDNS must be enabled in trusted mode");
    }

    #[test]
    fn test_lecture_hour_throttling() {
        // Monday 10:00 -> within lecture hours (09:00 - 17:00 weekdays)
        assert!(is_lecture_hours_at(Weekday::Mon, 10));
        assert!(is_lecture_hours_at(Weekday::Fri, 9));
        assert!(is_lecture_hours_at(Weekday::Wed, 16));

        // Outside lecture hours
        assert!(!is_lecture_hours_at(Weekday::Mon, 8));
        assert!(!is_lecture_hours_at(Weekday::Mon, 17));
        assert!(!is_lecture_hours_at(Weekday::Mon, 20));

        // Weekends -> never lecture hours
        assert!(!is_lecture_hours_at(Weekday::Sat, 12));
        assert!(!is_lecture_hours_at(Weekday::Sun, 14));
    }

    #[test]
    fn test_ssid_rules_crud() {
        let (_dir, db) = make_test_db();
        save_ssid_rule(&db, "MyHomeWiFi", "trusted").unwrap();
        let rule = get_rule_for_ssid(&db, "MyHomeWiFi").unwrap();
        assert_eq!(rule, Some(NetworkProfile::Trusted));

        let rules = list_all_ssid_rules(&db).unwrap();
        assert!(rules.iter().any(|r| r.ssid == "MyHomeWiFi" && r.profile == "trusted"));

        delete_ssid_rule_db(&db, "MyHomeWiFi").unwrap();
        assert_eq!(get_rule_for_ssid(&db, "MyHomeWiFi").unwrap(), None);
    }

    #[test]
    fn test_background_bandwidth_limits() {
        let (_dir, db) = make_test_db();

        // 1. Campus during lecture hours: defaults to 128 KB/s
        let cap_lecture = get_background_bandwidth_limit_at(&db, NetworkProfile::Campus, true);
        assert_eq!(cap_lecture, Some(MAX_LECTURE_BACKGROUND_BPS));

        // 2. Campus outside lecture hours: defaults to 512 KB/s
        let cap_outside = get_background_bandwidth_limit_at(&db, NetworkProfile::Campus, false);
        assert_eq!(cap_outside, Some(MAX_CAMPUS_BACKGROUND_BPS));

        // 3. Trusted network: None (unmetered)
        let cap_trusted = get_background_bandwidth_limit_at(&db, NetworkProfile::Trusted, true);
        assert_eq!(cap_trusted, None);

        // 4. Custom setting override
        db.set_setting("lecture_bandwidth_cap_bps", "65536").unwrap();
        let cap_custom = get_background_bandwidth_limit_at(&db, NetworkProfile::Campus, true);
        assert_eq!(cap_custom, Some(65536));
    }
}
