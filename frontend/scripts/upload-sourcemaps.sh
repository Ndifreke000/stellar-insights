#!/usr/bin/env bash
set -euo pipefail

# PayRaider Frontend Sentry Release & Source Map Upload
# Uploads Next.js source maps to Sentry for readable stack traces.

VERSION="${NEXT_PUBLIC_APP_VERSION:-${APP_VERSION:-$(git rev-parse HEAD 2>/dev/null || echo "0.1.0")}}"
ORG="${SENTRY_ORG:-payraider}"
PROJECT="${SENTRY_PROJECT:-payraider-frontend}"

if [ -z "${SENTRY_AUTH_TOKEN:-}" ]; then
  echo "[Sentry] SENTRY_AUTH_TOKEN is not set. Skipping source map upload."
  exit 0
fi

echo "[Sentry] Uploading source maps for release ${VERSION} (${ORG}/${PROJECT})..."

if command -v sentry-cli >/dev/null 2>&1; then
  SENTRY_CMD="sentry-cli"
else
  SENTRY_CMD="npx @sentry/cli"
fi

# Create release
$SENTRY_CMD releases new "$VERSION" --org "$ORG" --project "$PROJECT"

# Associate commits if git repository is available
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  $SENTRY_CMD releases set-commits "$VERSION" --auto --org "$ORG" --ignore-missing || true
fi

# Upload source maps from .next build output
if [ -d ".next" ]; then
  $SENTRY_CMD releases files "$VERSION" upload-sourcemaps .next \
    --org "$ORG" \
    --project "$PROJECT" \
    --url-prefix '~/_next' || true
fi

# Finalize release
$SENTRY_CMD releases finalize "$VERSION" --org "$ORG" --project "$PROJECT"

echo "[Sentry] Successfully completed source map upload for release ${VERSION}."
