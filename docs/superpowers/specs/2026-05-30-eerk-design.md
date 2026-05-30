# Eerk — Repo Baseline & Compliance Checker

**Date:** 2026-05-30
**Status:** Design approved, ready for implementation plan
**Form:** GitHub Action for Rust repositories (no AI)

## Summary

Eerk keeps a fleet of Rust repositories aligned to a single canonical
baseline. A repository is created from a GitHub *template repo* (the source of
truth). Over time, files and repository settings drift from that baseline —
someone edits CI by hand, weakens branch protection, removes a license, drops a
required label. Eerk detects this drift and reports it to a single tracking
issue in the affected repo. It **never overwrites** anything.

Eerk is the first of a planned family of repo-automation tools. Two siblings
are out of scope here and will get their own specs and names later:

- **merge bot** (Rultor-like, comment-driven gated merge)
- **todo→issue** (PDD-like, scans code markers into issues)

## Goals

- Every repo created from the template starts with the same structure.
- Drift from the baseline (files *and* settings) is surfaced automatically.
- Reporting is idempotent and noise-free: one tracking issue, opened when drift
  exists, closed when clean.
- The comparison engine is pure and offline-testable.
- Zero AI. Everything is deterministic: hashing, presence checks, API field
  comparison.

## Non-Goals

- No auto-fix / no overwriting repo content or settings. Report only.
- No partial-file management (no "managed header + local tail"). A file is
  either `exact` or `presence`.
- No central orchestrator. Pull model only (each repo checks itself).
- Not the merge bot or the todo→issue tool.

## Architecture

### Source of truth: the template repo

```
template-repo/
  baseline/                    # canonical copies of managed files
    .github/workflows/ci.yml
    rustfmt.toml
    deny.toml
    ...
  baseline.toml                # manifest: managed files + policy rules
  .github/workflows/eerk.yml   # seeded into every new repo (the Pull worker)
```

New repos are created from this template, so `eerk.yml` is already present in
each repo.

### Pull model

Each repo runs `eerk.yml` on a schedule (and on manual dispatch). The workflow:

1. **Checkout** the target repo.
2. **Fetch baseline** — clone/download `baseline/` + `baseline.toml` from the
   template repo at a **pinned tag** (so editing the template does not break
   every repo at once; bumping the pin is a deliberate act).
3. **Collect settings** — run `gh api` calls to build `state.json`: branch
   protection, repo settings, security toggles, labels, collaborators.
4. **Run the engine** (Rust binary) — input: baseline dir + repo working dir +
   `state.json`. Output: `report.json`.
5. **Report** — render `report.json` to markdown and upsert the tracking issue.

All network access lives in workflow steps. The binary touches only local
files passed to it — it is fully offline-testable.

### Manifest: `baseline.toml`

```toml
# --- files ---
[[file]]
path = ".github/workflows/ci.yml"
mode = "exact"          # drift = content hash differs (also missing)

[[file]]
path = "LICENSE"
mode = "presence"       # drift = file absent

# --- policy (checked via gh api) ---
[policy.branch_protection.main]
required_reviews = 1
required_checks   = ["build", "test", "clippy"]
linear_history    = true
enforce_admins    = true

[policy.repo]
visibility             = "private"
default_branch         = "main"
delete_branch_on_merge = true
allow_merge_commit     = false

[policy.security]
secret_scanning   = true
dependabot_alerts = true

[[policy.label]]          # required by sibling tools (pdd/merge bot)
name = "puzzle"
[[policy.label]]
name = "bug"
```

File modes:

- `exact` — file must exist and its content hash must match the baseline copy.
- `presence` — file must exist; content is not compared.

### Engine (Rust binary)

Pure comparison. No network.

- **Input:**
  - `--baseline <dir>` — checkout of `baseline/`
  - `--manifest <baseline.toml>`
  - `--repo <dir>` — the target repo working tree
  - `--state <state.json>` — settings snapshot collected by the workflow
- **Logic:**
  - For each `[[file]]`: `presence` → exists? `exact` → exists AND
    `sha256(repo file) == sha256(baseline file)`?
  - For each policy rule: compare expected value from manifest to actual value
    in `state.json`. A rule whose actual value is missing from `state.json`
    (insufficient token scope) is reported as `unknown`, not as drift.
- **Output:** `report.json`

```json
{
  "files":  { "missing": ["LICENSE"], "drifted": [".github/workflows/ci.yml"] },
  "policy": [
    { "rule": "branch_protection.main.enforce_admins",
      "expected": true, "actual": false, "status": "drift" },
    { "rule": "security.secret_scanning",
      "expected": true, "actual": null, "status": "unknown" }
  ]
}
```

The engine is the reusable, unit-tested core. It is the only Rust code; it
also dogfoods Rust, matching the target ecosystem.

### Reporting

The workflow renders `report.json` to markdown and upserts a single issue
identified by the label `repo-drift`:

- Drift present → open the issue (or update its body) with the current list of
  missing/drifted files and policy violations. `unknown` items are listed in a
  separate "could not verify" section.
- No drift → close the issue if it is open.

This keeps exactly one issue per repo, always reflecting current state, with no
duplicate spam across runs.

### Triggers

- `schedule` — periodic (default: weekly).
- `workflow_dispatch` — manual run.

### Token / permissions constraint

Reading branch protection and several repo settings requires a token with
`administration:read`. The default `GITHUB_TOKEN` does not always provide this.
Design accommodates both:

- If a stronger token (PAT or GitHub App installation token) is supplied via a
  secret, full policy checks run.
- If only `GITHUB_TOKEN` is available, settings it cannot read appear as
  `unknown` in the report (degrade gracefully) — never as false drift.

## Baseline contents (initial managed set for a Rust repo)

Files: `.github/workflows/*`, `rustfmt.toml`, clippy/lint config, `deny.toml`,
`.github/dependabot.yml`, `rust-toolchain.toml`, `LICENSE`, `.gitignore`,
`.editorconfig`, issue/PR templates, `CONTRIBUTING.md`.

Policy: branch protection on `main`, required status checks, repo merge
settings, visibility, default branch, security toggles, required labels
(including those the sibling tools need).

## Components & boundaries

- **`baseline.toml` parser** — manifest → typed config. Pure.
- **File checker** — manifest files + repo dir + baseline dir → file findings.
  Pure, hashing only.
- **Policy checker** — manifest policy + `state.json` → policy findings. Pure.
- **Report serializer** — findings → `report.json`. Pure.
- **Workflow (`eerk.yml`)** — orchestration + all I/O: checkout, baseline
  fetch, `gh api` collection, run binary, render markdown, upsert issue.
  Not unit-tested in Rust; covered by the action's own integration run.

Each Rust unit has one job, takes explicit inputs, returns data, and is
testable with fixture directories + a fixture `state.json`.

## Testing

- Unit tests per checker with fixture baseline dirs, repo dirs, and
  `state.json` snapshots covering: exact match, drifted content, missing file,
  policy match, policy drift, policy `unknown`.
- Golden-file test: fixtures → expected `report.json`.
- Manifest parser tests: valid manifest, unknown mode, malformed TOML.

## Open questions (deferred, not blocking)

- Exact cron frequency (default weekly; per-repo override later).
- Whether `state.json` collection is a shared composite action step or inline
  shell in `eerk.yml`.
- Distribution of the binary (prebuilt release artifact vs `cargo install` in
  the workflow).
