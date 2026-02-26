#!/bin/bash
# Phase 1: Reorganize Backend Documentation
# Moves 68 .md files from backend/ root to backend/docs/

set -e

cd "$(dirname "$0")/../.."
BACKEND_DIR="backend"

echo "🗂️  Phase 1: Reorganizing Backend Documentation"
echo "================================================"

# Create organized structure
echo "Creating directory structure..."
mkdir -p "$BACKEND_DIR/docs/security"
mkdir -p "$BACKEND_DIR/docs/operations"
mkdir -p "$BACKEND_DIR/docs/features"
mkdir -p "$BACKEND_DIR/docs/architecture"

# Security docs
echo "Moving security documentation..."
mv "$BACKEND_DIR"/SECURITY_*.md "$BACKEND_DIR/docs/security/" 2>/dev/null || true
mv "$BACKEND_DIR"/LOGGING_REDACTION_*.md "$BACKEND_DIR/docs/security/" 2>/dev/null || true
mv "$BACKEND_DIR"/IP_WHITELIST_*.md "$BACKEND_DIR/docs/security/" 2>/dev/null || true
mv "$BACKEND_DIR"/SEP10_*.md "$BACKEND_DIR/docs/security/" 2>/dev/null || true
mv "$BACKEND_DIR/LOGGING_SECURITY_README.md" "$BACKEND_DIR/docs/security/" 2>/dev/null || true
mv "$BACKEND_DIR/SENSITIVE_LOGGING_RESOLUTION.md" "$BACKEND_DIR/docs/security/" 2>/dev/null || true

# Operations docs
echo "Moving operations documentation..."
mv "$BACKEND_DIR"/GRACEFUL_SHUTDOWN_*.md "$BACKEND_DIR/docs/operations/" 2>/dev/null || true
mv "$BACKEND_DIR"/SHUTDOWN_*.md "$BACKEND_DIR/docs/operations/" 2>/dev/null || true
mv "$BACKEND_DIR/LOAD_TESTING.md" "$BACKEND_DIR/docs/operations/" 2>/dev/null || true
mv "$BACKEND_DIR/CI_CHECKS.md" "$BACKEND_DIR/docs/operations/" 2>/dev/null || true
mv "$BACKEND_DIR/BUILD_VERIFICATION.md" "$BACKEND_DIR/docs/operations/" 2>/dev/null || true
mv "$BACKEND_DIR/ADMIN_AUDIT_LOG.md" "$BACKEND_DIR/docs/operations/" 2>/dev/null || true
mv "$BACKEND_DIR/MIGRATIONS.md" "$BACKEND_DIR/docs/operations/" 2>/dev/null || true
mv "$BACKEND_DIR/MIGRATION_SYSTEM_SUMMARY.md" "$BACKEND_DIR/docs/operations/" 2>/dev/null || true

# Features docs
echo "Moving features documentation..."
mv "$BACKEND_DIR"/CACHE_*.md "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR"/CACHING_*.md "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR"/REDIS_*.md "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR"/RATE_LIMITING*.md "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR"/RPC_RATE_LIMITING.md "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR"/GRAPHQL_*.md "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR/WEBSOCKET_API.md" "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR/ML_README.md" "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR/BACKGROUND_JOBS.md" "$BACKEND_DIR/docs/features/" 2>/dev/null || true
mv "$BACKEND_DIR/COMPRESSION.md" "$BACKEND_DIR/docs/features/" 2>/dev/null || true

# Architecture docs
echo "Moving architecture documentation..."
mv "$BACKEND_DIR/APM_ARCHITECTURE.md" "$BACKEND_DIR/docs/architecture/" 2>/dev/null || true
mv "$BACKEND_DIR"/CORRIDOR_*.md "$BACKEND_DIR/docs/architecture/" 2>/dev/null || true
mv "$BACKEND_DIR"/SNAPSHOT_*.md "$BACKEND_DIR/docs/architecture/" 2>/dev/null || true
mv "$BACKEND_DIR/STELLAR_TOML_IMPLEMENTATION.md" "$BACKEND_DIR/docs/architecture/" 2>/dev/null || true
mv "$BACKEND_DIR/CONTRACT_EVENT_REPLAY_SYSTEM.md" "$BACKEND_DIR/docs/architecture/" 2>/dev/null || true
mv "$BACKEND_DIR/STRUCTURED_ERROR_RESPONSES.md" "$BACKEND_DIR/docs/architecture/" 2>/dev/null || true

# Summary/changelog docs
echo "Moving summary documentation..."
mv "$BACKEND_DIR"/CHANGES_*.md "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR"/FIXES_*.md "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR"/FIX_*.md "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR"/IMPLEMENTATION_*.md "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR"/DELIVERY_*.md "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR"/REFACTOR_*.md "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR/CODE_QUALITY.md" "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR/COMPILATION_FIXES.md" "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR/ERROR_HANDLING_VERIFIED.md" "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR/PERFORMANCE_INDEXES_GUIDE.md" "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR/QUICK_QUALITY_GUIDE.md" "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR/QUICK_REFERENCE.md" "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR/PR_DESCRIPTION.md" "$BACKEND_DIR/docs/" 2>/dev/null || true
mv "$BACKEND_DIR/README_FIX.md" "$BACKEND_DIR/docs/" 2>/dev/null || true

# Create index file
echo "Creating documentation index..."
cat > "$BACKEND_DIR/docs/README.md" << 'EOF'
# Backend Documentation

## Directory Structure

- **security/** - Security features, authentication, logging redaction
- **operations/** - Deployment, migrations, monitoring, CI/CD
- **features/** - Feature documentation (caching, rate limiting, GraphQL, etc.)
- **architecture/** - System architecture, design decisions

## Quick Links

### Security
- [SEP-10 Authentication](security/SEP10_AUTHENTICATION.md)
- [IP Whitelisting](security/IP_WHITELIST_DOCUMENTATION.md)
- [Logging Redaction](security/LOGGING_REDACTION_GUIDE.md)

### Operations
- [Graceful Shutdown](operations/GRACEFUL_SHUTDOWN_README.md)
- [Load Testing](operations/LOAD_TESTING.md)
- [Migrations](operations/MIGRATIONS.md)

### Features
- [Caching Implementation](features/CACHING_IMPLEMENTATION.md)
- [Rate Limiting](features/RATE_LIMITING.md)
- [GraphQL API](features/GRAPHQL_API.md)

### Architecture
- [APM Architecture](architecture/APM_ARCHITECTURE.md)
- [Corridor Detection](architecture/CORRIDOR_DETECTION_FLOW.md)
- [Snapshot System](architecture/SNAPSHOT_HASH_SERVICE.md)
EOF

echo ""
echo "✅ Backend documentation reorganized successfully!"
echo ""
echo "Summary:"
echo "  - Created docs/{security,operations,features,architecture}/"
echo "  - Moved ~68 .md files from backend/ root"
echo "  - Created docs/README.md index"
echo ""
echo "Next: Run 2-reorganize-frontend-docs.sh"
