#!/bin/bash

# Upload source maps to Sentry after build
# Usage: ./scripts/upload-sourcemaps.sh

set -e

# Check required environment variables
if [ -z "$SENTRY_AUTH_TOKEN" ]; then
  echo "Error: SENTRY_AUTH_TOKEN not set"
  exit 1
fi

if [ -z "$SENTRY_ORG" ]; then
  echo "Error: SENTRY_ORG not set"
  exit 1
fi

if [ -z "$SENTRY_PROJECT" ]; then
  echo "Error: SENTRY_PROJECT not set"
  exit 1
fi

RELEASE="${NEXT_PUBLIC_APP_VERSION:-$(git rev-parse --short HEAD)}"

echo "Uploading source maps to Sentry..."
echo "Release: $RELEASE"
echo "Organization: $SENTRY_ORG"
echo "Project: $SENTRY_PROJECT"

# Create release
sentry-cli releases -o "$SENTRY_ORG" -p "$SENTRY_PROJECT" create "$RELEASE" || true

# Upload source maps
sentry-cli releases -o "$SENTRY_ORG" -p "$SENTRY_PROJECT" files "$RELEASE" upload-sourcemaps \
  --url-prefix "~/app" \
  .next/static

# Finalize release
sentry-cli releases -o "$SENTRY_ORG" -p "$SENTRY_PROJECT" finalize "$RELEASE"

echo "Source maps uploaded successfully!"
