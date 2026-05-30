use crate::manifest::{FileMode, FileRule};
use crate::report::FileFindings;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn check(rules: &[FileRule], baseline: &Path, repo: &Path) -> FileFindings {
    let mut findings = FileFindings::default();
    for rule in rules {
        let repo_path = repo.join(&rule.path);
        if !repo_path.exists() {
            findings.missing.push(rule.path.clone());
            continue;
        }
        if rule.mode == FileMode::Exact {
            let base_path = baseline.join(&rule.path);
            if hash(&base_path) != hash(&repo_path) {
                findings.drifted.push(rule.path.clone());
            }
        }
    }
    findings
}

fn hash(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{FileMode, FileRule};
    use std::fs;

    fn write(dir: &std::path::Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    #[test]
    fn detects_missing_drifted_and_clean() {
        let base = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();

        write(base.path(), "ci.yml", "A");
        write(base.path(), "LICENSE", "MIT");
        write(repo.path(), "ci.yml", "B");

        let rules = vec![
            FileRule {
                path: "ci.yml".into(),
                mode: FileMode::Exact,
            },
            FileRule {
                path: "LICENSE".into(),
                mode: FileMode::Presence,
            },
        ];
        let f = check(&rules, base.path(), repo.path());
        assert_eq!(f.drifted, vec!["ci.yml".to_string()]);
        assert_eq!(f.missing, vec!["LICENSE".to_string()]);
    }

    #[test]
    fn exact_match_is_clean() {
        let base = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        write(base.path(), "ci.yml", "SAME");
        write(repo.path(), "ci.yml", "SAME");
        let rules = vec![FileRule {
            path: "ci.yml".into(),
            mode: FileMode::Exact,
        }];
        let f = check(&rules, base.path(), repo.path());
        assert!(f.drifted.is_empty() && f.missing.is_empty());
    }

    #[test]
    fn presence_ignores_content() {
        let base = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        write(base.path(), "LICENSE", "MIT");
        write(repo.path(), "LICENSE", "APACHE");
        let rules = vec![FileRule {
            path: "LICENSE".into(),
            mode: FileMode::Presence,
        }];
        let f = check(&rules, base.path(), repo.path());
        assert!(f.drifted.is_empty() && f.missing.is_empty());
    }
}
