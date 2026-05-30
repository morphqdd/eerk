use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Manifest {
    #[serde(default)]
    pub file: Vec<FileRule>,
    #[serde(default)]
    pub policy: Policy,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct FileRule {
    pub path: String,
    pub mode: FileMode,
}

#[derive(Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum FileMode {
    Exact,
    Presence,
}

#[derive(Deserialize, Debug, PartialEq, Default)]
pub struct Policy {
    #[serde(default)]
    pub branch_protection: BTreeMap<String, toml::Table>,
    #[serde(default)]
    pub repo: toml::Table,
    #[serde(default)]
    pub security: toml::Table,
    #[serde(default)]
    pub label: Vec<Label>,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Label {
    pub name: String,
}

pub fn parse(s: &str) -> Result<Manifest, toml::de::Error> {
    toml::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[file]]
path = ".github/workflows/ci.yml"
mode = "exact"

[[file]]
path = "LICENSE"
mode = "presence"

[policy.branch_protection.main]
enforce_admins = true

[policy.repo]
visibility = "private"

[policy.security]
secret_scanning = true

[[policy.label]]
name = "puzzle"
"#;

    #[test]
    fn parses_files() {
        let m = parse(SAMPLE).unwrap();
        assert_eq!(m.file.len(), 2);
        assert_eq!(m.file[0].path, ".github/workflows/ci.yml");
        assert_eq!(m.file[0].mode, FileMode::Exact);
        assert_eq!(m.file[1].mode, FileMode::Presence);
    }

    #[test]
    fn parses_policy_sections() {
        let m = parse(SAMPLE).unwrap();
        assert!(m.policy.branch_protection.contains_key("main"));
        assert_eq!(m.policy.label[0].name, "puzzle");
    }

    #[test]
    fn empty_manifest_is_valid() {
        let m = parse("").unwrap();
        assert!(m.file.is_empty());
        assert!(m.policy.label.is_empty());
    }

    #[test]
    fn unknown_mode_errors() {
        let bad = "[[file]]\npath = \"x\"\nmode = \"sometimes\"\n";
        assert!(parse(bad).is_err());
    }
}
