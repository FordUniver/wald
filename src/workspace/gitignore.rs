use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Markers for wald-managed gitignore section (per ADR-004)
const GITIGNORE_MARKER_START: &str = "# wald:start (managed by wald, do not edit)";
const GITIGNORE_MARKER_END: &str = "# wald:end";

/// Wald-managed gitignore patterns (per ADR-004)
const GITIGNORE_PATTERNS: &[&str] = &[
    ".wald/repos/",
    ".wald/state.yaml",
    "**/.baum/manifest.local.yaml",
    "**/_*.wt/",
];

/// Ensure the workspace .gitignore has the wald managed section
pub fn ensure_gitignore_section(workspace_root: &Path) -> Result<()> {
    let gitignore_path = workspace_root.join(".gitignore");
    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("failed to read .gitignore: {}", gitignore_path.display()))?
    } else {
        String::new()
    };

    // Check if section exists and is complete (has markers and all patterns)
    let has_complete_section = content.contains(GITIGNORE_MARKER_START)
        && content.contains(GITIGNORE_MARKER_END)
        && GITIGNORE_PATTERNS.iter().all(|p| content.contains(p));

    if has_complete_section {
        return Ok(());
    }

    // Remove existing incomplete section if present
    let content = remove_wald_section(&content);

    // Create managed section with all patterns
    let patterns = GITIGNORE_PATTERNS.join("\n");
    let managed_section = format!(
        "\n{}\n{}\n{}\n",
        GITIGNORE_MARKER_START, patterns, GITIGNORE_MARKER_END
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

/// Remove existing wald section from gitignore content (inclusive of markers)
fn remove_wald_section(content: &str) -> String {
    let start_idx = content.find(GITIGNORE_MARKER_START);
    let end_idx = content.find(GITIGNORE_MARKER_END);

    match (start_idx, end_idx) {
        (Some(start), Some(end)) if start < end => {
            // Find the end of the end marker line
            let end_line_end = content[end..]
                .find('\n')
                .map(|i| end + i + 1)
                .unwrap_or(content.len());

            // Find start of the start marker line
            // If marker isn't at position 0, look for preceding newline to include in output
            let section_start = if start > 0 {
                content[..start].rfind('\n').map(|i| i + 1).unwrap_or(0)
            } else {
                0
            };

            let mut result = String::new();
            result.push_str(&content[..section_start]);
            result.push_str(&content[end_line_end..]);
            result
        }
        _ => content.to_string(),
    }
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
        assert!(content.contains(GITIGNORE_MARKER_END));
        // Check all ADR-004 patterns are present
        assert!(content.contains(".wald/repos/"));
        assert!(content.contains(".wald/state.yaml"));
        assert!(content.contains("**/.baum/manifest.local.yaml"));
        assert!(content.contains("**/_*.wt/"));
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

    #[test]
    fn test_ensure_gitignore_section_repairs_incomplete() {
        let dir = TempDir::new().unwrap();

        // Write incomplete section (missing patterns)
        let incomplete = format!(
            "*.log\n{}\n.wald/repos/\n{}\n",
            GITIGNORE_MARKER_START, GITIGNORE_MARKER_END
        );
        fs::write(dir.path().join(".gitignore"), &incomplete).unwrap();

        // Should repair the incomplete section
        ensure_gitignore_section(dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();

        // Should have exactly one section
        let count = content.matches(GITIGNORE_MARKER_START).count();
        assert_eq!(count, 1);

        // Should have all patterns
        assert!(content.contains(".wald/repos/"));
        assert!(content.contains(".wald/state.yaml"));
        assert!(content.contains("**/.baum/manifest.local.yaml"));
        assert!(content.contains("**/_*.wt/"));

        // Should preserve other content
        assert!(content.contains("*.log"));
    }

    #[test]
    fn test_remove_wald_section() {
        let content =
            "before\n# wald:start (managed by wald, do not edit)\npattern\n# wald:end\nafter\n";
        let result = remove_wald_section(content);
        assert_eq!(result, "before\nafter\n");
    }

    #[test]
    fn test_remove_wald_section_no_section() {
        let content = "*.log\n*.tmp\n";
        let result = remove_wald_section(content);
        assert_eq!(result, content);
    }
}
