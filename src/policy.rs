use crate::manifest::{expectations, Policy};
use crate::report::{PolicyFinding, PolicyStatus};
use serde_json::Value;
use std::collections::BTreeMap;

pub type State = BTreeMap<String, Value>;

pub fn check(policy: &Policy, state: &State) -> Vec<PolicyFinding> {
    expectations(policy)
        .into_iter()
        .map(|e| match state.get(&e.key) {
            None => PolicyFinding {
                rule: e.key,
                expected: e.expected,
                actual: Value::Null,
                status: PolicyStatus::Unknown,
            },
            Some(actual) if *actual == e.expected => PolicyFinding {
                rule: e.key,
                expected: e.expected,
                actual: actual.clone(),
                status: PolicyStatus::Ok,
            },
            Some(actual) => PolicyFinding {
                rule: e.key,
                expected: e.expected,
                actual: actual.clone(),
                status: PolicyStatus::Drift,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::parse;
    use crate::report::PolicyStatus;
    use serde_json::json;

    const SAMPLE: &str = r#"
[policy.repo]
visibility = "private"

[policy.security]
secret_scanning = true
"#;

    fn state(pairs: &[(&str, serde_json::Value)]) -> State {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn ok_when_actual_matches() {
        let p = parse(SAMPLE).unwrap().policy;
        let s = state(&[("repo.visibility", json!("private")), ("security.secret_scanning", json!(true))]);
        let findings = check(&p, &s);
        assert!(findings.iter().all(|f| f.status == PolicyStatus::Ok));
    }

    #[test]
    fn drift_when_actual_differs() {
        let p = parse(SAMPLE).unwrap().policy;
        let s = state(&[("repo.visibility", json!("public")), ("security.secret_scanning", json!(true))]);
        let findings = check(&p, &s);
        let f = findings.iter().find(|f| f.rule == "repo.visibility").unwrap();
        assert_eq!(f.status, PolicyStatus::Drift);
        assert_eq!(f.actual, json!("public"));
    }

    #[test]
    fn unknown_when_state_missing_key() {
        let p = parse(SAMPLE).unwrap().policy;
        let s = state(&[("repo.visibility", json!("private"))]); // no security key
        let findings = check(&p, &s);
        let f = findings.iter().find(|f| f.rule == "security.secret_scanning").unwrap();
        assert_eq!(f.status, PolicyStatus::Unknown);
        assert_eq!(f.actual, serde_json::Value::Null);
    }
}
