use std::collections::HashSet;

/// Generate a unique 6-character hex baum ID
///
/// The ID is guaranteed to be unique within the provided set of existing IDs.
/// Uses cryptographic randomness from getrandom.
pub fn generate_baum_id(existing_ids: &HashSet<String>) -> String {
    loop {
        let mut bytes = [0u8; 3];
        getrandom::getrandom(&mut bytes).expect("failed to generate random bytes");
        let id = hex::encode(bytes);

        if !existing_ids.contains(&id) {
            return id;
        }
    }
}

/// Format a wald local branch name
///
/// Returns `wald/<baum_id>/<branch>` for tracking branches.
pub fn format_wald_branch(baum_id: &str, branch: &str) -> String {
    format!("wald/{}/{}", baum_id, branch)
}

/// Parse a wald local branch name
///
/// Returns `(baum_id, branch)` if the branch matches `wald/<id>/<branch>` pattern.
pub fn parse_wald_branch(branch: &str) -> Option<(&str, &str)> {
    let branch = branch.strip_prefix("wald/")?;
    let (baum_id, branch) = branch.split_once('/')?;
    if baum_id.len() == 6 && baum_id.chars().all(|c| c.is_ascii_hexdigit()) {
        Some((baum_id, branch))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_baum_id_format() {
        let id = generate_baum_id(&HashSet::new());
        assert_eq!(id.len(), 6);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_baum_id_uniqueness() {
        let mut existing = HashSet::new();
        for _ in 0..100 {
            let id = generate_baum_id(&existing);
            assert!(!existing.contains(&id));
            existing.insert(id);
        }
    }

    #[test]
    fn test_generate_baum_id_avoids_existing() {
        let mut existing = HashSet::new();
        existing.insert("abc123".to_string());
        let id = generate_baum_id(&existing);
        assert_ne!(id, "abc123");
    }

    #[test]
    fn test_format_wald_branch() {
        assert_eq!(format_wald_branch("abc123", "main"), "wald/abc123/main");
        assert_eq!(
            format_wald_branch("def456", "feature/foo"),
            "wald/def456/feature/foo"
        );
    }

    #[test]
    fn test_parse_wald_branch() {
        assert_eq!(
            parse_wald_branch("wald/abc123/main"),
            Some(("abc123", "main"))
        );
        assert_eq!(
            parse_wald_branch("wald/def456/feature/foo"),
            Some(("def456", "feature/foo"))
        );
    }

    #[test]
    fn test_parse_wald_branch_invalid() {
        // Not a wald branch
        assert_eq!(parse_wald_branch("main"), None);
        assert_eq!(parse_wald_branch("refs/heads/main"), None);

        // Invalid ID format
        assert_eq!(parse_wald_branch("wald/abc/main"), None); // too short
        assert_eq!(parse_wald_branch("wald/abcdefgh/main"), None); // too long
        assert_eq!(parse_wald_branch("wald/abcxyz/main"), None); // not hex

        // Missing parts
        assert_eq!(parse_wald_branch("wald/abc123"), None);
        assert_eq!(parse_wald_branch("wald/"), None);
    }
}
