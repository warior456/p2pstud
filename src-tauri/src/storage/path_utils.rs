// src-tauri/src/storage/path_utils.rs
// Path sanitization utilities to prevent directory traversal attacks.
// All user-supplied paths and filenames must go through `safe_join` before use.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped.to_string());
        }
    }
    path
}

/// Join `base` with `user_input` and verify the result stays within `base`.
/// Returns an error on any path traversal attempt (e.g., "../../../etc/passwd").
pub fn safe_join(base: &Path, user_input: &str) -> Result<PathBuf> {
    // Reject null bytes — they would truncate paths on C-backed syscalls.
    if user_input.contains('\0') {
        bail!("path contains null byte");
    }

    let joined = base.join(user_input);

    // Canonicalize requires the path to exist on disk.
    let canonical = strip_unc_prefix(
        joined
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("path does not exist or cannot be resolved: {e}"))?,
    );

    let canonical_base = strip_unc_prefix(
        base.canonicalize()
            .map_err(|e| anyhow::anyhow!("base path cannot be resolved: {e}"))?,
    );

    if !canonical.starts_with(&canonical_base) {
        bail!(
            "path traversal attempt: '{}' is outside base '{}'",
            canonical.display(),
            canonical_base.display()
        );
    }
    Ok(canonical)
}

/// Like `safe_join`, but for paths that may not yet exist on disk.
/// Uses lexical normalization (no symlink resolution) — sufficient for
/// validating new file names before creation.
pub fn safe_join_new(base: &Path, user_input: &str) -> Result<PathBuf> {
    if user_input.contains('\0') {
        bail!("path contains null byte");
    }

    // The canonical base must exist (we only relax requirements for the child).
    let canonical_base = strip_unc_prefix(
        base.canonicalize()
            .map_err(|e| anyhow::anyhow!("base path cannot be resolved: {e}"))?,
    );

    let joined = canonical_base.join(user_input);
    let normalized = normalize_path(&joined);

    if !normalized.starts_with(&canonical_base) {
        bail!(
            "path traversal attempt: '{}' is outside base '{}'",
            normalized.display(),
            canonical_base.display()
        );
    }
    Ok(normalized)
}

/// Lexically normalize a path by resolving `.` and `..` components without
/// hitting the filesystem. This is conservative — it does not follow symlinks.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components: Vec<std::ffi::OsString> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // Pop the last real component (if any) — this removes `..`.
                components.pop();
            }
            std::path::Component::CurDir => {
                // Skip `.` — it adds nothing.
            }
            c => {
                components.push(c.as_os_str().to_owned());
            }
        }
    }
    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn path_within_base_accepted() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        let result = safe_join(dir.path(), "subdir");
        assert!(result.is_ok());
        let expected = strip_unc_prefix(dir.path().canonicalize().unwrap());
        assert!(result.unwrap().starts_with(&expected));
    }

    #[test]
    fn path_traversal_rejected() {
        let dir = TempDir::new().unwrap();
        let result = safe_join(dir.path(), "../../etc/passwd");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("traversal") || msg.contains("outside") || msg.contains("does not exist"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn null_byte_rejected() {
        let dir = TempDir::new().unwrap();
        let result = safe_join(dir.path(), "file\0name");
        assert!(result.is_err());
    }

    #[test]
    fn new_file_safe_join_accepted() {
        let dir = TempDir::new().unwrap();
        // File doesn't exist yet — safe_join_new should still work.
        let result = safe_join_new(dir.path(), "new_document.md");
        assert!(result.is_ok());
    }

    #[test]
    fn new_file_traversal_rejected() {
        let dir = TempDir::new().unwrap();
        let result = safe_join_new(dir.path(), "../outside.txt");
        assert!(result.is_err());
    }
}
