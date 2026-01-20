use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const GITIGNORE_MARKER_START: &str = "# >>> wald managed";
const GITIGNORE_MARKER_END: &str = "# <<< wald managed";

/// Ensure the workspace .gitignore has the wald managed section
pub fn ensure_gitignore_section(workspace_root: &Path) -> Result<()> {
    let gitignore_path = workspace_root.join(".gitignore");
    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("failed to read .gitignore: {}", gitignore_path.display()))?
    } else {
        String::new()
    };

    // Check if managed section already exists
    if content.contains(GITIGNORE_MARKER_START) {
        return Ok(());
    }

    // Create managed section
    let managed_section = format!(
        r#"
{GITIGNORE_MARKER_START}
# Local sync state (not committed)
.wald/state.yaml
{GITIGNORE_MARKER_END}
"#
    );

    // Append to existing content
    let new_content = if content.is_empty() {
        managed_section.trim_start().to_string()
    } else if content.ends_with('\n') {
        format!("{}{}", content, managed_section)
    } else {
        format!("{}\n{}", content, managed_section)
    };

    fs::write(&gitignore_path, new_content)
        .with_context(|| format!("failed to write .gitignore: {}", gitignore_path.display()))?;

    Ok(())
}

/// Add a worktree pattern to the container's .gitignore
pub fn add_worktree_to_gitignore(container: &Path, worktree_path: &str) -> Result<()> {
    let gitignore_path = container.join(".gitignore");
    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("failed to read .gitignore: {}", gitignore_path.display()))?
    } else {
        String::new()
    };

    // Check if pattern already exists
    let pattern = format!("/{}", worktree_path);
    if content.lines().any(|line| line.trim() == pattern) {
        return Ok(());
    }

    // Add pattern
    let new_content = if content.is_empty() {
        format!("{}\n", pattern)
    } else if content.ends_with('\n') {
        format!("{}{}\n", content, pattern)
    } else {
        format!("{}\n{}\n", content, pattern)
    };

    fs::write(&gitignore_path, new_content)
        .with_context(|| format!("failed to write .gitignore: {}", gitignore_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_gitignore_section_creates_file() {
        let dir = TempDir::new().unwrap();
        ensure_gitignore_section(dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.contains(GITIGNORE_MARKER_START));
        assert!(content.contains(".wald/state.yaml"));
    }

    #[test]
    fn test_ensure_gitignore_section_appends() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();

        ensure_gitignore_section(dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.starts_with("*.log"));
        assert!(content.contains(GITIGNORE_MARKER_START));
    }

    #[test]
    fn test_ensure_gitignore_section_idempotent() {
        let dir = TempDir::new().unwrap();
        ensure_gitignore_section(dir.path()).unwrap();
        ensure_gitignore_section(dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        let count = content.matches(GITIGNORE_MARKER_START).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_add_worktree_to_gitignore() {
        let dir = TempDir::new().unwrap();
        add_worktree_to_gitignore(dir.path(), "_main.wt").unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.contains("/_main.wt"));
    }

    #[test]
    fn test_add_worktree_to_gitignore_idempotent() {
        let dir = TempDir::new().unwrap();
        add_worktree_to_gitignore(dir.path(), "_main.wt").unwrap();
        add_worktree_to_gitignore(dir.path(), "_main.wt").unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        let count = content.matches("/_main.wt").count();
        assert_eq!(count, 1);
    }
}
