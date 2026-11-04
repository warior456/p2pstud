// src-tauri/src/storage/path_utils.rs
// Path sanitization utilities to prevent directory traversal attacks.
// All user-supplied paths and filenames must go through `safe_join` before use.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use unicode_normalization::UnicodeNormalization;

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

/// Validate and sanitize a virtual workspace path/filename.
/// Ensures the path is strictly relative, normalized with forward slashes `/`,
/// does not contain directory traversal (`..`), null bytes, Windows drive prefixes,
/// reserved device names, or invalid filename characters.
pub fn sanitize_virtual_path(raw: &str) -> Result<String> {
    if raw.contains('\0') {
        bail!("path contains null byte");
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("path cannot be empty");
    }

    // Reject Windows drive letters (e.g. "C:", "D:\")
    if trimmed.len() >= 2 && trimmed.chars().nth(1) == Some(':') {
        bail!("absolute drive paths not permitted");
    }

    // Convert Windows backslashes to standard forward slashes
    let normalized = trimmed.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|p| !p.is_empty()).collect();

    if parts.is_empty() {
        bail!("path contains no valid components");
    }

    const FORBIDDEN_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    for segment in &parts {
        let seg = segment.trim();
        if seg == "." || seg == ".." {
            bail!("directory traversal '..' detected in path");
        }
        if seg.chars().any(|c| FORBIDDEN_CHARS.contains(&c) || c.is_control()) {
            bail!("invalid characters in path component '{seg}'");
        }
        let base_name = seg.split('.').next().unwrap_or(seg).to_ascii_uppercase();
        if RESERVED_NAMES.contains(&base_name.as_str()) {
            bail!("reserved system filename '{seg}' not permitted");
        }
    }

    Ok(parts.join("/"))
}

/// Normalize and validate a virtual file path component for cross-platform safety.
/// Returns Err if any rule is violated; returns Ok(normalized) on success.
///
/// Rules applied in order:
/// 1. Apply NFC Unicode normalization.
/// 2. Reject strings containing null bytes (\0).
/// 3. Reject strings containing Windows-illegal characters: < > : " \ | ? *
/// 4. Reject strings whose last path component (after final '/') matches a Windows reserved
///    device name (case-insensitive): CON, PRN, AUX, NUL, COM0-COM9, LPT0-LPT9.
/// 5. Reject paths exceeding 200 characters total.
/// 6. Strip leading and trailing whitespace and dots from each path component.
/// 7. After normalization, perform a case-insensitive uniqueness check against existing
///    entries in the same virtual folder. Return Err if a case-insensitive match is found.
pub fn normalize_virtual_path(input: &str, folder_sibling_names: &[&str]) -> Result<String, String> {
    // 1. Apply NFC Unicode normalization
    let nfc_input: String = input.nfc().collect();

    // 2. Reject strings containing null bytes
    if nfc_input.contains('\0') {
        return Err("Path contains null byte".to_string());
    }

    // 3. Reject strings containing Windows-illegal characters
    const FORBIDDEN_CHARS: &[char] = &['<', '>', ':', '"', '\\', '|', '?', '*'];
    if nfc_input.chars().any(|c| FORBIDDEN_CHARS.contains(&c) || c.is_control()) {
        return Err("Path contains illegal characters: < > : \" \\ | ? *".to_string());
    }

    // Split components by forward slash
    let raw_components: Vec<&str> = nfc_input.split('/').collect();
    if raw_components.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    // 6. Strip leading and trailing whitespace and dots from each path component
    let mut normalized_components = Vec::new();
    for comp in raw_components {
        let cleaned = comp.trim().trim_matches('.');
        if cleaned.is_empty() {
            continue;
        }
        normalized_components.push(cleaned.to_string());
    }

    if normalized_components.is_empty() {
        return Err("Path contains no valid components".to_string());
    }

    // 4. Check Windows reserved device names on the last component
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL",
        "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let last_comp = normalized_components.last().unwrap();
    let base_name = last_comp.split('.').next().unwrap_or(last_comp).to_ascii_uppercase();
    if RESERVED_NAMES.contains(&base_name.as_str()) {
        return Err(format!("Reserved system device name '{last_comp}' is not allowed"));
    }

    let normalized_path = normalized_components.join("/");

    // 5. Reject paths exceeding 200 characters total
    if normalized_path.len() > 200 {
        return Err("Path exceeds maximum length of 200 characters".to_string());
    }

    // 7. Case-insensitive uniqueness check against siblings
    let target_name = normalized_components.last().unwrap().as_str();
    for sibling in folder_sibling_names {
        let sibling_name = sibling.split('/').last().unwrap_or(sibling).trim();
        if sibling_name.eq_ignore_ascii_case(target_name) {
            return Err(format!("Case collision detected with existing file '{sibling}'"));
        }
    }

    Ok(normalized_path)
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

    #[test]
    fn virtual_path_sanitization() {
        assert_eq!(sanitize_virtual_path("folder/sub/file.pdf").unwrap(), "folder/sub/file.pdf");
        assert_eq!(sanitize_virtual_path(r"folder\sub\file.pdf").unwrap(), "folder/sub/file.pdf");
        assert_eq!(sanitize_virtual_path("/root/file.txt").unwrap(), "root/file.txt");

        assert!(sanitize_virtual_path("../evil.txt").is_err());
        assert!(sanitize_virtual_path("folder/../../evil.txt").is_err());
        assert!(sanitize_virtual_path("C:\\Windows\\System32").is_err());
        assert!(sanitize_virtual_path("file\0name.txt").is_err());
        assert!(sanitize_virtual_path("COM1.txt").is_err());
        assert!(sanitize_virtual_path("CON").is_err());
        assert!(sanitize_virtual_path("bad*char.txt").is_err());
    }

    #[test]
    fn test_normalize_virtual_path_rules() {
        assert!(normalize_virtual_path("CON.pdf", &[]).is_err());
        assert!(normalize_virtual_path("notes<1>.md", &[]).is_err());
        assert!(normalize_virtual_path("notes.md", &["Notes.md"]).is_err());
        assert_eq!(
            normalize_virtual_path("  .folder. / ..file.txt.. ", &[]).unwrap(),
            "folder/file.txt"
        );
        assert!(normalize_virtual_path("file\0name.txt", &[]).is_err());
        assert!(normalize_virtual_path(&"a".repeat(201), &[]).is_err());
    }
}
