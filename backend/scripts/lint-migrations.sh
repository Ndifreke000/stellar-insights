#!/bin/bash
# Migration linter - checks for destructive SQL statements without contract-phase markers
#
# This script ensures all destructive migrations (DROP TABLE, DROP COLUMN, etc.)
# have a -- contract-phase: marker to prevent issues during rolling deployments.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATIONS_DIR="${SCRIPT_DIR}/../migrations"

if [[ ! -d "$MIGRATIONS_DIR" ]]; then
    echo "ERROR: Migrations directory not found: $MIGRATIONS_DIR"
    exit 1
fi

# Find all migration files
MIGRATION_FILES=$(find "$MIGRATIONS_DIR" -name "*.sql" -type f | sort)

if [[ -z "$MIGRATION_FILES" ]]; then
    echo "No migration files found in $MIGRATIONS_DIR"
    exit 0
fi

ERRORS=0
WARNINGS=0

echo "Checking migrations for safety issues..."
echo ""

for file in $MIGRATION_FILES; do
    filename=$(basename "$file")
    
    # Check for destructive statements without contract-phase marker
    if grep -qiE "^\s*(DROP\s+(TABLE|COLUMN|INDEX|CONSTRAINT)|ALTER\s+TABLE\s+.*\bDROP\s+(COLUMN|CONSTRAINT)|RENAME\s+COLUMN)" "$file"; then
        if ! grep -q "contract-phase:" "$file"; then
            echo "❌ ERROR: $filename contains destructive statement without contract-phase marker"
            echo "   Please add: -- contract-phase: <description>"
            echo ""
            ERRORS=$((ERRORS + 1))
        fi
    fi
    
    # Check for contract-phase markers on safe migrations (shouldn't have them)
    if grep -qi "contract-phase:" "$file"; then
        # Check if it's actually a destructive migration
        if ! grep -qiE "^\s*(DROP\s+(TABLE|COLUMN|INDEX|CONSTRAINT)|ALTER\s+TABLE\s+.*\bDROP\s+(COLUMN|CONSTRAINT)|RENAME\s+COLUMN)" "$file"; then
            echo "⚠️  WARNING: $filename has contract-phase marker but is not destructive"
            echo "   Consider removing the contract-phase marker"
            echo ""
            WARNINGS=$((WARNINGS + 1))
        fi
    fi
    
    # Check for ALTER TABLE without proper formatting
    if grep -qiE "^\s*ALTER\s+TABLE\s+\S+\s+DROP\s+COLUMN" "$file" | grep -v "^--"; then
        if ! grep -qi "contract-phase:" "$file"; then
            echo "❌ ERROR: $filename contains ALTER TABLE DROP COLUMN without contract-phase marker"
            echo ""
            ERRORS=$((ERRORS + 1))
        fi
    fi
done

echo ""
echo "=== Lint Summary ==="
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"
echo ""

if [[ $ERRORS -gt 0 ]]; then
    echo "❌ Migration linting failed. Please fix the errors above."
    echo ""
    echo "For more information, see: backend/docs/MIGRATION_SAFETY.md"
    exit 1
fi

if [[ $WARNINGS -gt 0 ]]; then
    echo "⚠️  Migration linting passed with warnings. Please review the warnings above."
fi

echo "✅ Migration linting passed!"
