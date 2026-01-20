use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// A detected move from git history
#[derive(Debug, Clone)]
pub struct MoveEntry {
    pub old_path: String,
    pub new_path: String,
    pub similarity: u8,
}

/// Detect baum moves between two commits using `git diff -M`
///
/// Returns moves of .baum/manifest.yaml files, which indicate baum relocations.
pub fn detect_moves(repo_path: &Path, from_commit: &str, to_commit: &str) -> Result<Vec<MoveEntry>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("diff")
        .arg("-M")
        .arg("--name-status")
        .arg("--first-parent")
        .arg("--diff-filter=R")
        .arg(format!("{}..{}", from_commit, to_commit))
        .output()
        .with_context(|| format!("failed to run git diff for move detection"))?;

    if !output.status.success() {
        // Empty result on error (not a fatal condition)
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_move_output(&stdout)
}

fn parse_move_output(output: &str) -> Result<Vec<MoveEntry>> {
    let mut moves = Vec::new();

    for line in output.lines() {
        // Format: R<similarity>\t<old_path>\t<new_path>
        // Example: R100	old/path/.baum/manifest.yaml	new/path/.baum/manifest.yaml
        if !line.starts_with('R') {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 3 {
            continue;
        }

        let old_path = parts[1];
        let new_path = parts[2];

        // Only track moves of .baum/manifest.yaml files
        if !old_path.ends_with(".baum/manifest.yaml") {
            continue;
        }

        // Extract similarity from R<number>
        let similarity: u8 = parts[0]
            .strip_prefix('R')
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        // Convert paths from .baum/manifest.yaml to container paths
        let old_container = old_path
            .strip_suffix("/.baum/manifest.yaml")
            .or_else(|| old_path.strip_suffix(".baum/manifest.yaml"))
            .unwrap_or(old_path);

        let new_container = new_path
            .strip_suffix("/.baum/manifest.yaml")
            .or_else(|| new_path.strip_suffix(".baum/manifest.yaml"))
            .unwrap_or(new_path);

        moves.push(MoveEntry {
            old_path: old_container.to_string(),
            new_path: new_container.to_string(),
            similarity,
        });
    }

    Ok(moves)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_move_output() {
        let output = "R100\ttools/repo/.baum/manifest.yaml\tadmin/repo/.baum/manifest.yaml\n";
        let moves = parse_move_output(output).unwrap();

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].old_path, "tools/repo");
        assert_eq!(moves[0].new_path, "admin/repo");
        assert_eq!(moves[0].similarity, 100);
    }

    #[test]
    fn test_parse_move_output_ignores_non_baum() {
        let output = "R100\ttools/file.txt\tadmin/file.txt\n";
        let moves = parse_move_output(output).unwrap();

        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_parse_move_output_multiple() {
        let output = r#"R100	tools/repo1/.baum/manifest.yaml	admin/repo1/.baum/manifest.yaml
R095	tools/repo2/.baum/manifest.yaml	research/repo2/.baum/manifest.yaml
"#;
        let moves = parse_move_output(output).unwrap();

        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0].old_path, "tools/repo1");
        assert_eq!(moves[1].new_path, "research/repo2");
        assert_eq!(moves[1].similarity, 95);
    }
}
