use crate::report::{PolicyStatus, Report};

pub fn render(report: &Report) -> String {
    if !report.has_drift() && report.policy.is_empty() {
        return "## Eerk baseline check\n\n✅ All baseline checks pass.\n".to_string();
    }

    let mut out = String::from("## Eerk baseline check\n\n");

    if !report.files.missing.is_empty() {
        out.push_str("### Missing files\n");
        for f in &report.files.missing {
            out.push_str(&format!("- `{f}`\n"));
        }
        out.push('\n');
    }

    if !report.files.drifted.is_empty() {
        out.push_str("### Drifted files\n");
        for f in &report.files.drifted {
            out.push_str(&format!("- `{f}`\n"));
        }
        out.push('\n');
    }

    let violations: Vec<_> = report
        .policy
        .iter()
        .filter(|p| p.status == PolicyStatus::Drift)
        .collect();
    if !violations.is_empty() {
        out.push_str("### Policy violations\n");
        for v in violations {
            out.push_str(&format!(
                "- `{}` — expected `{}`, got `{}`\n",
                v.rule, v.expected, v.actual
            ));
        }
        out.push('\n');
    }

    let unknown: Vec<_> = report
        .policy
        .iter()
        .filter(|p| p.status == PolicyStatus::Unknown)
        .collect();
    if !unknown.is_empty() {
        out.push_str("### Could not verify\n");
        out.push_str("_Insufficient token scope to read these settings._\n");
        for u in unknown {
            out.push_str(&format!("- `{}`\n", u.rule));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{FileFindings, PolicyFinding, PolicyStatus, Report};
    use serde_json::{json, Value};

    #[test]
    fn clean_report_says_pass() {
        let r = Report { files: FileFindings::default(), policy: vec![] };
        let md = render(&r);
        assert!(md.contains("All baseline checks pass"));
    }

    #[test]
    fn lists_missing_drifted_and_violations() {
        let r = Report {
            files: FileFindings {
                missing: vec!["LICENSE".into()],
                drifted: vec![".github/workflows/ci.yml".into()],
            },
            policy: vec![
                PolicyFinding {
                    rule: "branch_protection.main.enforce_admins".into(),
                    expected: json!(true),
                    actual: json!(false),
                    status: PolicyStatus::Drift,
                },
                PolicyFinding {
                    rule: "security.secret_scanning".into(),
                    expected: json!(true),
                    actual: Value::Null,
                    status: PolicyStatus::Unknown,
                },
            ],
        };
        let md = render(&r);
        assert!(md.contains("Missing files"));
        assert!(md.contains("LICENSE"));
        assert!(md.contains("Drifted files"));
        assert!(md.contains("ci.yml"));
        assert!(md.contains("Policy violations"));
        assert!(md.contains("enforce_admins"));
        assert!(md.contains("Could not verify"));
        assert!(md.contains("secret_scanning"));
    }
}
