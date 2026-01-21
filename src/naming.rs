//! Branch name normalization for safe directory names.
//!
//! Git branch names can contain characters that are problematic for directory names:
//! - `/` (common in feature/foo, bugfix/bar patterns)
//! - Spaces
//! - Backslashes (on Windows)
//! - Other special characters
//!
//! This module provides normalization to create safe worktree directory names
//! while preserving the original branch name in the manifest.

/// Normalize a branch name for use as a directory component
///
/// Transformations:
/// - `/` → `--` (preserves hierarchy readability: feature/foo → feature--foo)
/// - ` ` → `-` (spaces to dashes)
/// - `\` → `--` (backslashes like slashes)
/// - Multiple consecutive `-` collapsed to `--` max
/// - Leading/trailing `-` stripped
///
/// The original branch name is preserved in the baum manifest; only the
/// directory path uses the normalized form.
pub fn normalize_branch_for_path(branch: &str) -> String {
    let mut result = String::with_capacity(branch.len() + 10);

    for c in branch.chars() {
        match c {
            '/' | '\\' => result.push_str("--"),
            ' ' => result.push('-'),
            // Keep alphanumeric, dash, underscore, dot
            c if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' => result.push(c),
            // Skip other special characters
            _ => {}
        }
    }

    // Collapse runs of dashes to max 2
    let mut collapsed = String::with_capacity(result.len());
    let mut dash_count = 0;

    for c in result.chars() {
        if c == '-' {
            dash_count += 1;
            if dash_count <= 2 {
                collapsed.push(c);
            }
        } else {
            dash_count = 0;
            collapsed.push(c);
        }
    }

    // Trim leading/trailing dashes
    collapsed.trim_matches('-').to_string()
}

/// Generate a worktree directory name from a branch name
///
/// Format: `_{normalized_branch}.wt`
pub fn worktree_dir_name(branch: &str) -> String {
    let normalized = normalize_branch_for_path(branch);
    format!("_{}.wt", normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_branch() {
        assert_eq!(normalize_branch_for_path("main"), "main");
        assert_eq!(normalize_branch_for_path("dev"), "dev");
    }

    #[test]
    fn test_feature_branch_with_slash() {
        assert_eq!(normalize_branch_for_path("feature/foo"), "feature--foo");
        assert_eq!(
            normalize_branch_for_path("bugfix/issue-123"),
            "bugfix--issue-123"
        );
    }

    #[test]
    fn test_nested_slashes() {
        assert_eq!(
            normalize_branch_for_path("feature/foo/bar"),
            "feature--foo--bar"
        );
    }

    #[test]
    fn test_spaces() {
        assert_eq!(normalize_branch_for_path("my branch"), "my-branch");
    }

    #[test]
    fn test_backslash() {
        assert_eq!(normalize_branch_for_path("feature\\foo"), "feature--foo");
    }

    #[test]
    fn test_special_chars_stripped() {
        assert_eq!(normalize_branch_for_path("branch:name"), "branchname");
        assert_eq!(normalize_branch_for_path("branch@name"), "branchname");
    }

    #[test]
    fn test_dash_collapse() {
        // Multiple slashes shouldn't create excessive dashes
        assert_eq!(normalize_branch_for_path("a//b"), "a--b");
        assert_eq!(normalize_branch_for_path("a///b"), "a--b");
    }

    #[test]
    fn test_leading_trailing_stripped() {
        assert_eq!(normalize_branch_for_path("/feature"), "feature");
        assert_eq!(normalize_branch_for_path("feature/"), "feature");
    }

    #[test]
    fn test_worktree_dir_name() {
        assert_eq!(worktree_dir_name("main"), "_main.wt");
        assert_eq!(worktree_dir_name("feature/foo"), "_feature--foo.wt");
    }

    #[test]
    fn test_preserves_dots_underscores() {
        assert_eq!(normalize_branch_for_path("release_1.0"), "release_1.0");
        assert_eq!(normalize_branch_for_path("v2.0.0-rc1"), "v2.0.0-rc1");
    }
}
