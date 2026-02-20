#!/usr/bin/env bash
set -euo pipefail

BASE_BRANCH="${1:-main}"

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    printf "Missing required command: %s\n" "$cmd" >&2
    exit 1
  fi
}

require_cmd git
require_cmd gh

if ! gh auth status -h github.com >/dev/null 2>&1; then
  printf "GitHub CLI is not authenticated. Run: gh auth login\n" >&2
  exit 1
fi

CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$CURRENT_BRANCH" = "HEAD" ]; then
  printf "Detached HEAD is not supported for PR creation.\n" >&2
  exit 1
fi

if [ "$CURRENT_BRANCH" = "$BASE_BRANCH" ]; then
  printf "Current branch is '%s'. Create a feature branch first.\n" "$BASE_BRANCH" >&2
  exit 1
fi

TITLE="$(git log -1 --pretty=%s)"
BODY="$(git log -1 --pretty=%b)"

if [ -z "${TITLE//[[:space:]]/}" ]; then
  printf "Unable to derive PR title from the latest commit subject.\n" >&2
  exit 1
fi

if [ -z "${BODY//[[:space:]]/}" ] && [ -f ".github/pull_request_template.md" ]; then
  BODY="$(cat .github/pull_request_template.md)"
fi

if [ -z "${BODY//[[:space:]]/}" ]; then
  BODY="Auto-generated PR for branch '$CURRENT_BRANCH'. Please add details before merge."
fi

git push -u origin "$CURRENT_BRANCH"

EXISTING_PR_NUMBER="$(gh pr list \
  --head "$CURRENT_BRANCH" \
  --base "$BASE_BRANCH" \
  --state open \
  --json number \
  --jq '.[0].number' 2>/dev/null || true)"

if [ -n "$EXISTING_PR_NUMBER" ]; then
  gh pr edit "$EXISTING_PR_NUMBER" \
    --title "$TITLE" \
    --body "$BODY"
  printf "Updated PR #%s for branch '%s'.\n" "$EXISTING_PR_NUMBER" "$CURRENT_BRANCH"
else
  gh pr create \
    --base "$BASE_BRANCH" \
    --head "$CURRENT_BRANCH" \
    --title "$TITLE" \
    --body "$BODY"
fi
