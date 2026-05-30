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

use serde_json::Value as Json;

#[derive(Debug, PartialEq)]
pub struct Expectation {
    pub key: String,
    pub expected: Json,
}

fn toml_to_json(v: &toml::Value) -> Json {
    match v {
        toml::Value::String(s) => Json::String(s.clone()),
        toml::Value::Integer(i) => Json::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(Json::Number)
            .unwrap_or(Json::Null),
        toml::Value::Boolean(b) => Json::Bool(*b),
        toml::Value::Array(a) => Json::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            Json::Object(t.iter().map(|(k, v)| (k.clone(), toml_to_json(v))).collect())
        }
        toml::Value::Datetime(d) => Json::String(d.to_string()),
    }
}

pub fn expectations(p: &Policy) -> Vec<Expectation> {
    let mut out = Vec::new();
    for (branch, fields) in &p.branch_protection {
        for (k, v) in fields {
            out.push(Expectation {
                key: format!("branch_protection.{branch}.{k}"),
                expected: toml_to_json(v),
            });
        }
    }
    for (k, v) in &p.repo {
        out.push(Expectation {
            key: format!("repo.{k}"),
            expected: toml_to_json(v),
        });
    }
    for (k, v) in &p.security {
        out.push(Expectation {
            key: format!("security.{k}"),
            expected: toml_to_json(v),
        });
    }
    for l in &p.label {
        out.push(Expectation {
            key: format!("label.{}", l.name),
            expected: Json::Bool(true),
        });
    }
    out
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

    #[test]
    fn flattens_policy_to_dotted_keys() {
        let m = parse(SAMPLE).unwrap();
        let exps = expectations(&m.policy);
        let keys: Vec<&str> = exps.iter().map(|e| e.key.as_str()).collect();
        assert!(keys.contains(&"branch_protection.main.enforce_admins"));
        assert!(keys.contains(&"repo.visibility"));
        assert!(keys.contains(&"security.secret_scanning"));
        assert!(keys.contains(&"label.puzzle"));
    }

    #[test]
    fn label_expectation_is_true() {
        let m = parse(SAMPLE).unwrap();
        let e = expectations(&m.policy)
            .into_iter()
            .find(|e| e.key == "label.puzzle")
            .unwrap();
        assert_eq!(e.expected, serde_json::json!(true));
    }

    #[test]
    fn string_value_converts_to_json_string() {
        let m = parse(SAMPLE).unwrap();
        let e = expectations(&m.policy)
            .into_iter()
            .find(|e| e.key == "repo.visibility")
            .unwrap();
        assert_eq!(e.expected, serde_json::json!("private"));
    }
}
