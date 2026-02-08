use std::path::{Path, PathBuf};

use crate::error::{Result, SandboxError};

/// Validate that a path resolves within the allowed root directory.
/// Prevents path traversal attacks (e.g., `../../etc/passwd`).
pub fn validate_path(root: &Path, requested: &str) -> Result<PathBuf> {
    let root = root.canonicalize().map_err(SandboxError::Io)?;

    let full_path = root.join(requested);

    // Resolve the path (handles `..`, `.`, symlinks)
    let resolved = if full_path.exists() {
        full_path.canonicalize().map_err(SandboxError::Io)?
    } else {
        // For paths that don't exist yet, normalize manually
        normalize_path(&full_path)
    };

    if !resolved.starts_with(&root) {
        return Err(SandboxError::PathTraversal(format!(
            "'{}' escapes sandbox root '{}'",
            requested,
            root.display()
        )));
    }

    Ok(resolved)
}

/// Normalize a path without requiring it to exist on disk.
fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_path() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create a file inside root
        std::fs::write(root.join("test.txt"), "hello").unwrap();

        let result = validate_path(root, "test.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_traversal_blocked() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let result = validate_path(root, "../../../etc/passwd");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SandboxError::PathTraversal(_)
        ));
    }

    #[test]
    fn test_nested_path() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::write(root.join("a/b/c.txt"), "content").unwrap();

        let result = validate_path(root, "a/b/c.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nonexistent_path_within_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let result = validate_path(root, "new_file.txt");
        assert!(result.is_ok());
    }
}
