#!/usr/bin/env bash
# Builds a flat state.json: dotted policy key -> JSON value.
# Requires: gh (authenticated), jq. REPO and BRANCH from env.
set -euo pipefail

repo="${REPO:?REPO required}"      # owner/name
branch="${BRANCH:-main}"

state='{}'
put() { state="$(jq --arg k "$1" --argjson v "$2" '. + {($k): $v}' <<<"$state")"; }

# Repo settings (GITHUB_TOKEN can read these).
if rep="$(gh api "repos/$repo" 2>/dev/null)"; then
  put "repo.visibility"             "$(jq '.visibility' <<<"$rep")"
  put "repo.default_branch"         "$(jq '.default_branch' <<<"$rep")"
  put "repo.delete_branch_on_merge" "$(jq '.delete_branch_on_merge' <<<"$rep")"
  put "repo.allow_merge_commit"     "$(jq '.allow_merge_commit' <<<"$rep")"
  # secret scanning: present only with admin scope; absent -> unknown.
  sa="$(jq -c '.security_and_analysis // empty' <<<"$rep")"
  if [ -n "$sa" ]; then
    put "security.secret_scanning" \
        "$(jq '(.secret_scanning.status // "") == "enabled"' <<<"$sa")"
  fi
fi

# Branch protection (needs administration:read; absent keys -> "unknown").
if bp="$(gh api "repos/$repo/branches/$branch/protection" 2>/dev/null)"; then
  put "branch_protection.$branch.enforce_admins" \
      "$(jq '.enforce_admins.enabled' <<<"$bp")"
  put "branch_protection.$branch.linear_history" \
      "$(jq '.required_linear_history.enabled' <<<"$bp")"
  put "branch_protection.$branch.required_reviews" \
      "$(jq '.required_pull_request_reviews.required_approving_review_count // null' <<<"$bp")"
  put "branch_protection.$branch.required_checks" \
      "$(jq '.required_status_checks.contexts // null' <<<"$bp")"
fi

# Labels -> label.<name> = true, plus a sentinel marking the set complete.
# The sentinel lets the checker treat an absent required label as drift
# (closed-world) rather than "could not verify".
if labels="$(gh api "repos/$repo/labels" --paginate 2>/dev/null)"; then
  while read -r name; do
    [ -n "$name" ] && put "label.$name" true
  done < <(jq -r '.[].name' <<<"$labels")
  put "__labels_collected__" true
fi

echo "$state" > state.json
