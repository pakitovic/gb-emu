#!/usr/bin/env bash
set -euo pipefail

BASE_BRANCH="main"
DRY_RUN=0

while (($# > 0)); do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --help|-h)
      cat <<'EOF'
Usage: scripts/dev/create_pr.sh [--dry-run] [base-branch]

Creates or updates a GitHub PR for the current branch using the latest commit
subject/body as title/description.

Options:
  --dry-run   Print the resolved PR title/body and exit without pushing/editing.
  -h, --help  Show this help.
EOF
      exit 0
      ;;
    -*)
      printf "Unknown option: %s\n" "$1" >&2
      exit 1
      ;;
    *)
      if [ "$BASE_BRANCH" != "main" ]; then
        printf "Unexpected extra argument: %s\n" "$1" >&2
        exit 1
      fi
      BASE_BRANCH="$1"
      shift
      ;;
  esac
done

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    printf "Missing required command: %s\n" "$cmd" >&2
    exit 1
  fi
}

normalize_pr_body() {
  local body="$1"
  local body_trimmed="$body"
  local escaped_lf='\n'
  local escaped_crlf='\r\n'
  local escaped_lf_pattern='\\n'
  local escaped_crlf_pattern='\\r\\n'
  local newline=$'\n'

  # If the commit body contains literal "\n" escapes (for example
  # `git commit -m "..." -m "Line1\nLine2"`), decode them so GitHub renders a
  # multiline PR description instead of showing the escape sequences verbatim.
  #
  # `git log --pretty=%b` commonly includes a trailing real newline even when
  # the body content itself was authored as a single line with escaped newlines.
  # Trim trailing newlines for detection so we still decode that case, but avoid
  # rewriting commit bodies that are already multiline and merely mention "\n".
  while [[ "$body_trimmed" == *$'\n' ]]; do
    body_trimmed="${body_trimmed%$'\n'}"
  done

  if [[ "$body_trimmed" != *$'\n'* ]] &&
     [[ "$body" == *"$escaped_lf"* || "$body" == *"$escaped_crlf"* ]]; then
    body="${body//$escaped_crlf_pattern/$newline}"
    body="${body//$escaped_lf_pattern/$newline}"
  fi

  printf '%s' "$body"
}

require_cmd git
if [ "$DRY_RUN" -eq 0 ]; then
  require_cmd gh

  if ! gh auth status -h github.com >/dev/null 2>&1; then
    printf "GitHub CLI is not authenticated. Run: gh auth login\n" >&2
    exit 1
  fi
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
BODY="$(normalize_pr_body "$BODY")"

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

BODY_FILE="$(mktemp)"
trap 'rm -f "$BODY_FILE"' EXIT
printf '%s' "$BODY" > "$BODY_FILE"

if [ "$DRY_RUN" -eq 1 ]; then
  printf "Dry run: no push / no GitHub API calls.\n"
  printf "Base branch: %s\n" "$BASE_BRANCH"
  printf "Head branch: %s\n" "$CURRENT_BRANCH"
  printf "Title: %s\n" "$TITLE"
  printf -- "----- PR body (begin) -----\n"
  cat "$BODY_FILE"
  printf '\n'
  printf -- "----- PR body (end) -----\n"
  exit 0
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
    --body-file "$BODY_FILE"
  printf "Updated PR #%s for branch '%s'.\n" "$EXISTING_PR_NUMBER" "$CURRENT_BRANCH"
else
  gh pr create \
    --base "$BASE_BRANCH" \
    --head "$CURRENT_BRANCH" \
    --title "$TITLE" \
    --body-file "$BODY_FILE"
fi
