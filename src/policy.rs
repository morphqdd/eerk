use crate::manifest::{expectations, Policy};
use crate::report::{PolicyFinding, PolicyStatus};
use serde_json::Value;
use std::collections::BTreeMap;

pub type State = BTreeMap<String, Value>;

fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return false;
            }
            let mut xs: Vec<String> = x.iter().map(|v| v.to_string()).collect();
            let mut ys: Vec<String> = y.iter().map(|v| v.to_string()).collect();
            xs.sort();
            ys.sort();
            xs == ys
        }
        _ => a == b,
    }
}

const LABELS_COLLECTED: &str = "__labels_collected__";

pub fn check(policy: &Policy, state: &State) -> Vec<PolicyFinding> {
    let labels_collected = matches!(state.get(LABELS_COLLECTED), Some(Value::Bool(true)));
    expectations(policy)
        .into_iter()
        .map(|e| {
            let present = matches!(state.get(&e.key), Some(v) if !v.is_null());
            let (actual, status) = if present {
                let a = state.get(&e.key).unwrap().clone();
                let status = if json_eq(&a, &e.expected) {
                    PolicyStatus::Ok
                } else {
                    PolicyStatus::Drift
                };
                (a, status)
            } else if e.key.starts_with("label.") && labels_collected {
                (Value::Bool(false), PolicyStatus::Drift)
            } else {
                (Value::Null, PolicyStatus::Unknown)
            };
            PolicyFinding {
                rule: e.key,
                expected: e.expected,
                actual,
                status,
            }
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
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn ok_when_actual_matches() {
        let p = parse(SAMPLE).unwrap().policy;
        let s = state(&[
            ("repo.visibility", json!("private")),
            ("security.secret_scanning", json!(true)),
        ]);
        let findings = check(&p, &s);
        assert!(findings.iter().all(|f| f.status == PolicyStatus::Ok));
    }

    #[test]
    fn drift_when_actual_differs() {
        let p = parse(SAMPLE).unwrap().policy;
        let s = state(&[
            ("repo.visibility", json!("public")),
            ("security.secret_scanning", json!(true)),
        ]);
        let findings = check(&p, &s);
        let f = findings
            .iter()
            .find(|f| f.rule == "repo.visibility")
            .unwrap();
        assert_eq!(f.status, PolicyStatus::Drift);
        assert_eq!(f.actual, json!("public"));
    }

    #[test]
    fn array_order_does_not_matter() {
        let p = parse("[policy.branch_protection.main]\nrequired_checks = [\"build\", \"test\"]\n")
            .unwrap()
            .policy;
        let s = state(&[(
            "branch_protection.main.required_checks",
            json!(["test", "build"]),
        )]);
        let findings = check(&p, &s);
        assert!(findings.iter().all(|f| f.status == PolicyStatus::Ok));
    }

    #[test]
    fn unknown_when_state_missing_key() {
        let p = parse(SAMPLE).unwrap().policy;
        let s = state(&[("repo.visibility", json!("private"))]); // no security key
        let findings = check(&p, &s);
        let f = findings
            .iter()
            .find(|f| f.rule == "security.secret_scanning")
            .unwrap();
        assert_eq!(f.status, PolicyStatus::Unknown);
        assert_eq!(f.actual, serde_json::Value::Null);
    }

    #[test]
    fn null_actual_is_unknown() {
        let p = parse("[policy.repo]\nallow_merge_commit = false\n")
            .unwrap()
            .policy;
        let s = state(&[("repo.allow_merge_commit", serde_json::Value::Null)]);
        let f = check(&p, &s);
        let x = f
            .iter()
            .find(|f| f.rule == "repo.allow_merge_commit")
            .unwrap();
        assert_eq!(x.status, PolicyStatus::Unknown);
    }

    #[test]
    fn missing_label_is_drift_when_labels_collected() {
        let p = parse("[[policy.label]]\nname = \"puzzle\"\n")
            .unwrap()
            .policy;
        let s = state(&[
            ("__labels_collected__", json!(true)),
            ("label.bug", json!(true)),
        ]);
        let f = check(&p, &s);
        let x = f.iter().find(|f| f.rule == "label.puzzle").unwrap();
        assert_eq!(x.status, PolicyStatus::Drift);
        assert_eq!(x.actual, json!(false));
    }

    #[test]
    fn present_label_is_ok() {
        let p = parse("[[policy.label]]\nname = \"bug\"\n").unwrap().policy;
        let s = state(&[
            ("__labels_collected__", json!(true)),
            ("label.bug", json!(true)),
        ]);
        let f = check(&p, &s);
        assert!(f.iter().all(|f| f.status == PolicyStatus::Ok));
    }

    #[test]
    fn missing_label_is_unknown_when_not_collected() {
        let p = parse("[[policy.label]]\nname = \"puzzle\"\n")
            .unwrap()
            .policy;
        let s = state(&[]);
        let f = check(&p, &s);
        let x = f.iter().find(|f| f.rule == "label.puzzle").unwrap();
        assert_eq!(x.status, PolicyStatus::Unknown);
    }
}
