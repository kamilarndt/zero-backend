use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Isolated workspace for a hand instance.
///
/// Provides a temporary directory at `/tmp/zeroclaw/hands/{hand_id}` for
/// hand-specific file operations, with cleanup support.
#[derive(Debug, Clone)]
pub struct HandWorkspace {
    /// Absolute path to the workspace directory
    pub path: PathBuf,
    /// Unique identifier for this hand
    pub hand_id: String,
}

impl HandWorkspace {
    /// Base directory for all hand workspaces
    const BASE_DIR: &'static str = "/tmp/zeroclaw/hands";

    /// Create a new workspace for the given hand_id.
    ///
    /// Creates the directory structure `/tmp/zeroclaw/hands/{hand_id}` if it
    /// doesn't already exist.
    ///
    /// # Arguments
    /// * `base_dir` - Base directory for workspaces (typically `/tmp/zeroclaw`)
    /// * `hand_id` - Unique identifier for this hand
    ///
    /// # Returns
    /// A `HandWorkspace` instance with the created directory path
    ///
    /// # Errors
    /// Returns an error if directory creation fails
    pub fn create(base_dir: &str, hand_id: &str) -> Result<Self> {
        let path = PathBuf::from(base_dir).join("hands").join(hand_id);

        // Create the directory structure
        fs::create_dir_all(&path)?;

        Ok(Self {
            path,
            hand_id: hand_id.to_string(),
        })
    }

    /// Remove the workspace directory and all its contents.
    ///
    /// # Errors
    /// Returns an error if the directory removal fails
    pub fn cleanup(&self) -> Result<()> {
        if self.path.exists() {
            fs::remove_dir_all(&self.path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_creation() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_str().unwrap();
        let hand_id = "test-hand-1";

        let workspace = HandWorkspace::create(base_dir, hand_id).unwrap();

        // Verify path structure
        let expected_path = PathBuf::from(base_dir).join("hands").join(hand_id);
        assert_eq!(workspace.path, expected_path);
        assert_eq!(workspace.hand_id, hand_id);

        // Verify directory exists
        assert!(workspace.path.exists());
        assert!(workspace.path.is_dir());
    }

    #[test]
    fn test_workspace_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_str().unwrap();
        let hand_id = "test-hand-2";

        let workspace = HandWorkspace::create(base_dir, hand_id).unwrap();
        let workspace_path = workspace.path.clone();

        // Verify directory exists
        assert!(workspace_path.exists());

        // Cleanup and verify removal
        workspace.cleanup().unwrap();
        assert!(!workspace_path.exists());
    }

    #[test]
    fn test_concurrent_isolation() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_str().unwrap();

        // Create multiple workspaces concurrently
        let workspace1 = HandWorkspace::create(base_dir, "hand-1").unwrap();
        let workspace2 = HandWorkspace::create(base_dir, "hand-2").unwrap();
        let workspace3 = HandWorkspace::create(base_dir, "hand-3").unwrap();

        // Verify each has unique path
        assert_ne!(workspace1.path, workspace2.path);
        assert_ne!(workspace2.path, workspace3.path);
        assert_ne!(workspace1.path, workspace3.path);

        // Verify all exist
        assert!(workspace1.path.exists());
        assert!(workspace2.path.exists());
        assert!(workspace3.path.exists());

        // Cleanup one and verify others remain
        workspace2.cleanup().unwrap();
        assert!(workspace1.path.exists());
        assert!(!workspace2.path.exists());
        assert!(workspace3.path.exists());
    }

    #[test]
    fn test_multiple_cleanup_calls() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_str().unwrap();
        let hand_id = "test-hand-multi-cleanup";

        let workspace = HandWorkspace::create(base_dir, hand_id).unwrap();

        // First cleanup should succeed
        workspace.cleanup().unwrap();

        // Second cleanup should also succeed (idempotent)
        workspace.cleanup().unwrap();
    }

    #[test]
    fn test_nested_hand_id() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_str().unwrap();
        let hand_id = "agent-1/session-2/hand-3";

        let workspace = HandWorkspace::create(base_dir, hand_id).unwrap();

        // Verify nested directory structure is created
        let expected_path = PathBuf::from(base_dir).join("hands").join(hand_id);
        assert_eq!(workspace.path, expected_path);
        assert!(workspace.path.exists());
        assert!(workspace.path.is_dir());

        // Verify cleanup removes entire nested structure
        workspace.cleanup().unwrap();
        assert!(!workspace.path.exists());
    }
}
