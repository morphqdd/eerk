use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Report {
    pub files: FileFindings,
    pub policy: Vec<PolicyFinding>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct FileFindings {
    pub missing: Vec<String>,
    pub drifted: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PolicyFinding {
    pub rule: String,
    pub expected: Value,
    pub actual: Value,
    pub status: PolicyStatus,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PolicyStatus {
    Ok,
    Drift,
    Unknown,
}

impl Report {
    pub fn has_drift(&self) -> bool {
        !self.files.missing.is_empty()
            || !self.files.drifted.is_empty()
            || self.policy.iter().any(|p| p.status == PolicyStatus::Drift)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn has_drift_true_on_missing_file() {
        let r = Report {
            files: FileFindings {
                missing: vec!["LICENSE".into()],
                drifted: vec![],
            },
            policy: vec![],
        };
        assert!(r.has_drift());
    }

    #[test]
    fn has_drift_false_when_clean_and_unknown_only() {
        let r = Report {
            files: FileFindings::default(),
            policy: vec![PolicyFinding {
                rule: "security.secret_scanning".into(),
                expected: json!(true),
                actual: serde_json::Value::Null,
                status: PolicyStatus::Unknown,
            }],
        };
        assert!(!r.has_drift());
    }

    #[test]
    fn serializes_status_lowercase() {
        let s = serde_json::to_string(&PolicyStatus::Drift).unwrap();
        assert_eq!(s, "\"drift\"");
    }
}
