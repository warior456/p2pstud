// src-tauri/src/p2p/identity.rs
// Persists the node's Ed25519 SecretKey across restarts.
// File is stored at <data_dir>/iroh_secret.key with owner-only permissions.

use std::path::Path;

use anyhow::{Context, Result};
use iroh::key::SecretKey;

/// Path within the app data directory where the secret key is stored.
const KEY_FILENAME: &str = "iroh_secret.key";

/// Load the node's secret key from disk, or generate and persist a new one.
pub fn load_or_create_secret_key(data_dir: &Path) -> Result<SecretKey> {
    let key_path = data_dir.join(KEY_FILENAME);

    if key_path.exists() {
        let bytes = std::fs::read(&key_path)
            .with_context(|| format!("reading secret key from {}", key_path.display()))?;

        let arr: [u8; 32] = bytes.try_into().map_err(|_| {
            anyhow::anyhow!("secret key file has wrong length — delete it to regenerate")
        })?;

        Ok(SecretKey::from_bytes(&arr))
    } else {
        let key = SecretKey::generate();
        let bytes = key.to_bytes();

        // Write atomically via a temp file, then rename.
        let tmp = key_path.with_extension("tmp");
        std::fs::write(&tmp, bytes)
            .with_context(|| format!("writing secret key to {}", tmp.display()))?;

        // Set owner-only permissions before the final rename.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))
                .context("setting secret key permissions to 0o600")?;
        }

        #[cfg(windows)]
        restrict_file_to_current_user(&tmp)?;

        std::fs::rename(&tmp, &key_path)
            .with_context(|| format!("renaming temp key to {}", key_path.display()))?;

        tracing::info!(
            "Generated new node identity, saved to {}",
            key_path.display()
        );
        Ok(key)
    }
}

/// On Windows, remove the "Everyone" ACE and restrict the key file to the
/// current user only. Uses the `icacls` system utility (available on all
/// modern Windows versions).
#[cfg(windows)]
fn restrict_file_to_current_user(path: &std::path::Path) -> Result<()> {
    use std::process::Command;

    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let path_str = full_path.to_string_lossy();
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "Everyone".to_string());

    // Remove inherited permissions and grant current user full control only.
    let status = Command::new("icacls")
        .args([
            path_str.as_ref(),
            "/inheritance:r", // remove inherited ACEs
            "/grant:r",
            &format!("{}:(F)", username),
        ])
        .status()
        .context("running icacls to restrict secret key permissions")?;

    if !status.success() {
        anyhow::bail!("icacls failed with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn secret_key_round_trips() {
        let dir = TempDir::new().unwrap();
        let key1 = load_or_create_secret_key(dir.path()).unwrap();
        let key2 = load_or_create_secret_key(dir.path()).unwrap();
        assert_eq!(key1.to_bytes(), key2.to_bytes());
    }

    #[test]
    fn fresh_key_is_created_when_missing() {
        let dir = TempDir::new().unwrap();
        assert!(!dir.path().join("iroh_secret.key").exists());
        let _key = load_or_create_secret_key(dir.path()).unwrap();
        assert!(dir.path().join("iroh_secret.key").exists());
    }
}
