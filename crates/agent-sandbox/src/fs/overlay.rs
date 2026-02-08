use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::Result;

/// Tracks filesystem changes by comparing against initial snapshots.
#[derive(Debug, Clone, PartialEq)]
pub enum FsChangeKind {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct FsChange {
    pub path: String,
    pub kind: FsChangeKind,
}

/// Filesystem overlay that tracks changes to the work directory.
pub struct FsOverlay {
    root: PathBuf,
    /// SHA-256 hashes of files at snapshot time.
    snapshot: HashMap<PathBuf, Vec<u8>>,
}

impl FsOverlay {
    /// Create a new overlay and snapshot the current state of the root directory.
    pub fn new(root: &Path) -> Result<Self> {
        let root = root.canonicalize()?;
        let mut snapshot = HashMap::new();
        snapshot_dir(&root, &mut snapshot)?;

        Ok(Self { root, snapshot })
    }

    /// Compare the current state against the snapshot and return changes.
    pub fn diff(&self) -> Result<Vec<FsChange>> {
        let mut changes = Vec::new();
        let mut current_files = HashMap::new();

        // Walk current state
        snapshot_dir(&self.root, &mut current_files)?;

        // Find created and modified files
        for (path, hash) in &current_files {
            let rel = path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            match self.snapshot.get(path) {
                None => {
                    changes.push(FsChange {
                        path: rel,
                        kind: FsChangeKind::Created,
                    });
                }
                Some(old_hash) if old_hash != hash => {
                    changes.push(FsChange {
                        path: rel,
                        kind: FsChangeKind::Modified,
                    });
                }
                _ => {}
            }
        }

        // Find deleted files
        for path in self.snapshot.keys() {
            if !current_files.contains_key(path) {
                let rel = path
                    .strip_prefix(&self.root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                changes.push(FsChange {
                    path: rel,
                    kind: FsChangeKind::Deleted,
                });
            }
        }

        changes.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(changes)
    }
}

fn snapshot_dir(dir: &Path, snapshot: &mut HashMap<PathBuf, Vec<u8>>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            snapshot_dir(&path, snapshot)?;
        } else if path.is_file() {
            let content = std::fs::read(&path)?;
            let hash = Sha256::digest(&content).to_vec();
            snapshot.insert(path, hash);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_created_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(root.join("existing.txt"), "hello").unwrap();

        let overlay = FsOverlay::new(root).unwrap();

        // Create a new file
        std::fs::write(root.join("new.txt"), "world").unwrap();

        let changes = overlay.diff().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "new.txt");
        assert_eq!(changes[0].kind, FsChangeKind::Created);
    }

    #[test]
    fn test_detect_modified_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(root.join("file.txt"), "original").unwrap();

        let overlay = FsOverlay::new(root).unwrap();

        // Modify the file
        std::fs::write(root.join("file.txt"), "modified").unwrap();

        let changes = overlay.diff().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "file.txt");
        assert_eq!(changes[0].kind, FsChangeKind::Modified);
    }

    #[test]
    fn test_detect_deleted_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(root.join("file.txt"), "content").unwrap();

        let overlay = FsOverlay::new(root).unwrap();

        // Delete the file
        std::fs::remove_file(root.join("file.txt")).unwrap();

        let changes = overlay.diff().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "file.txt");
        assert_eq!(changes[0].kind, FsChangeKind::Deleted);
    }

    #[test]
    fn test_no_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        std::fs::write(root.join("file.txt"), "content").unwrap();

        let overlay = FsOverlay::new(root).unwrap();
        let changes = overlay.diff().unwrap();
        assert!(changes.is_empty());
    }
}
