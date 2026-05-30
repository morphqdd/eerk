use std::fs;
use std::process::Command;

fn write(dir: &std::path::Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[test]
fn check_produces_report_with_drift() {
    let base = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    write(base.path(), "LICENSE", "MIT");
    // repo missing LICENSE entirely

    let manifest = work.path().join("baseline.toml");
    fs::write(&manifest, "[[file]]\npath = \"LICENSE\"\nmode = \"presence\"\n").unwrap();

    let state = work.path().join("state.json");
    fs::write(&state, "{}").unwrap();

    let report = work.path().join("report.json");

    let status = Command::new(env!("CARGO_BIN_EXE_eerk"))
        .args([
            "check",
            "--baseline", base.path().to_str().unwrap(),
            "--manifest", manifest.to_str().unwrap(),
            "--repo", repo.path().to_str().unwrap(),
            "--state", state.to_str().unwrap(),
            "--out", report.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(1));

    let body = fs::read_to_string(&report).unwrap();
    assert!(body.contains("\"missing\""));
    assert!(body.contains("LICENSE"));
}
