# Eerk

Repo baseline & compliance checker. Detects when a repository has drifted
from a canonical baseline — both files and GitHub settings — and reports it
to a single tracking issue. Never overwrites anything.

## CLI

    eerk check --baseline <dir> --manifest baseline.toml \
               --repo . --state state.json --out report.json
    eerk render --report report.json

`check` exits 1 when drift is present, 0 when clean.

## How it runs

`action/eerk.yml` is seeded into each repo from the template. On a schedule it
fetches the pinned baseline, collects settings via `gh api`
(`action/collect-state.sh`), runs `eerk check`, and upserts the `repo-drift`
issue.

See `examples/baseline.toml` for the manifest format.
