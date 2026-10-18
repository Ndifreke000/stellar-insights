#!/usr/bin/env bash
#
# check-git-credentials.sh — fail if credentials are embedded where they will leak.
#
# Two distinct exposures, because they leak through different channels:
#
#   1. Remote URLs (`git remote -v`, `.git/config`). A token here is captured by
#      anything that dumps remotes: CI logs, shell history, `git config --list`,
#      bug reports, screen shares. This is local-only state and is NOT fixed by
#      a commit — see docs/runbooks/credential-rotation.md.
#
#   2. Tracked files. A token committed here is in the history permanently and
#      is public the moment the repository is.
#
# Exit codes: 0 clean, 1 credentials found, 2 usage error.
#
# Usage:
#   scripts/check-git-credentials.sh            # check remotes + tracked files
#   scripts/check-git-credentials.sh --tracked  # tracked files only (CI)
#   scripts/check-git-credentials.sh --remotes  # remotes only (local)

set -euo pipefail

MODE="${1:-all}"
FOUND=0

# GitHub token prefixes, all followed by 36 base62 chars:
#   ghp_ personal access    gho_ oauth       ghu_ user-to-server
#   ghs_ server-to-server   ghr_ refresh     github_pat_ fine-grained
TOKEN_RE='((ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22,})'
# Any userinfo in an https remote — covers tokens that do not match the
# prefixes above, including other forges.
URL_CRED_RE='https://[^/[:space:]@]+:[^/[:space:]@]+@'

# Never print a matched secret. Redaction is the whole point of a checker that
# runs in CI, where the output is itself a log that persists.
redact() { sed -E "s/${TOKEN_RE}/<REDACTED-TOKEN>/g; s#${URL_CRED_RE}#https://<REDACTED-CREDENTIALS>@#g"; }

check_remotes() {
  echo "→ Checking git remote URLs..."
  local hits
  hits="$(git remote -v 2>/dev/null | grep -nE "${TOKEN_RE}|${URL_CRED_RE}" || true)"

  if [[ -n "${hits}" ]]; then
    echo "✗ Credentials embedded in remote URLs:"
    echo "${hits}" | redact | sed 's/^/    /'
    echo
    echo "  These are visible to anything that reads .git/config or runs"
    echo "  'git remote -v'. Rotate the token, then switch to a credential"
    echo "  helper: docs/runbooks/credential-rotation.md"
    FOUND=1
  else
    echo "  ✓ no credentials in remote URLs"
  fi
}

check_tracked() {
  echo "→ Checking tracked files..."
  local hits
  # This script necessarily contains the patterns it searches for, so exclude
  # itself — otherwise the checker fails on its own source.
  hits="$(git grep -nE "${TOKEN_RE}|${URL_CRED_RE}" -- . \
            ":(exclude)scripts/check-git-credentials.sh" \
            ":(exclude)docs/runbooks/credential-rotation.md" 2>/dev/null || true)"

  if [[ -n "${hits}" ]]; then
    echo "✗ Credentials found in tracked files:"
    echo "${hits}" | redact | sed 's/^/    /'
    echo
    echo "  A committed token is in the history permanently. Rotate it first,"
    echo "  then purge: docs/runbooks/credential-rotation.md"
    FOUND=1
  else
    echo "  ✓ no credentials in tracked files"
  fi
}

case "${MODE}" in
  all)      check_remotes; check_tracked ;;
  --remotes) check_remotes ;;
  --tracked) check_tracked ;;
  *) echo "usage: $0 [--remotes|--tracked]" >&2; exit 2 ;;
esac

if [[ "${FOUND}" -ne 0 ]]; then
  echo
  echo "Credential check FAILED."
  exit 1
fi

echo
echo "Credential check passed."
