#!/bin/bash
# Entrypoint for backend Docker container: runs migrations, then starts the app
set -e

/app/scripts/run_migrations.sh

exec "$@"
