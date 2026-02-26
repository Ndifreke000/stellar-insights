#!/bin/bash
# Phase 1: Reorganize Frontend Documentation
# Moves 35 .md files from frontend/ root to frontend/docs/

set -e

cd "$(dirname "$0")/../.."
FRONTEND_DIR="frontend"

echo "🗂️  Phase 1: Reorganizing Frontend Documentation"
echo "================================================"

# Create organized structure
echo "Creating directory structure..."
mkdir -p "$FRONTEND_DIR/docs/accessibility"
mkdir -p "$FRONTEND_DIR/docs/security"
mkdir -p "$FRONTEND_DIR/docs/guides"

# Accessibility docs (8 files)
echo "Moving accessibility documentation..."
mv "$FRONTEND_DIR"/ACCESSIBILITY_*.md "$FRONTEND_DIR/docs/accessibility/" 2>/dev/null || true

# Security docs (4 files)
echo "Moving security documentation..."
mv "$FRONTEND_DIR"/SECURITY_*.md "$FRONTEND_DIR/docs/security/" 2>/dev/null || true
mv "$FRONTEND_DIR"/CSRF_*.md "$FRONTEND_DIR/docs/security/" 2>/dev/null || true

# Development guides
echo "Moving development guides..."
mv "$FRONTEND_DIR"/CONSOLE_LOGGING_*.md "$FRONTEND_DIR/docs/guides/" 2>/dev/null || true
mv "$FRONTEND_DIR"/KEYBOARD_SHORTCUTS*.md "$FRONTEND_DIR/docs/guides/" 2>/dev/null || true
mv "$FRONTEND_DIR"/IMAGE_OPTIMIZATION_*.md "$FRONTEND_DIR/docs/guides/" 2>/dev/null || true
mv "$FRONTEND_DIR"/COLOR_CONTRAST_*.md "$FRONTEND_DIR/docs/guides/" 2>/dev/null || true
mv "$FRONTEND_DIR/MIGRATION_GUIDE.md" "$FRONTEND_DIR/docs/guides/" 2>/dev/null || true
mv "$FRONTEND_DIR/CHART_EXPORT_FEATURE.md" "$FRONTEND_DIR/docs/guides/" 2>/dev/null || true

# Build and issue docs
echo "Moving build and issue documentation..."
mv "$FRONTEND_DIR"/BUILD_*.md "$FRONTEND_DIR/docs/" 2>/dev/null || true
mv "$FRONTEND_DIR"/FIXES_*.md "$FRONTEND_DIR/docs/" 2>/dev/null || true
mv "$FRONTEND_DIR"/ISSUE_*.md "$FRONTEND_DIR/docs/" 2>/dev/null || true
mv "$FRONTEND_DIR/NODE_VERSION_FIX.md" "$FRONTEND_DIR/docs/" 2>/dev/null || true
mv "$FRONTEND_DIR/FRONTEND_BUILD_VERIFICATION.md" "$FRONTEND_DIR/docs/" 2>/dev/null || true
mv "$FRONTEND_DIR/TESTING_CHECKLIST.md" "$FRONTEND_DIR/docs/" 2>/dev/null || true

# Create index file
echo "Creating documentation index..."
cat > "$FRONTEND_DIR/docs/README.md" << 'EOF'
# Frontend Documentation

## Directory Structure

- **accessibility/** - WCAG compliance, ARIA, keyboard navigation
- **security/** - CSRF protection, XSS prevention, security updates
- **guides/** - Development guides (console logging, keyboard shortcuts, images, etc.)

## Quick Links

### Accessibility
- [Accessibility Index](accessibility/ACCESSIBILITY_INDEX.md)
- [Quick Start Guide](accessibility/ACCESSIBILITY_QUICK_START.md)
- [Testing Guide](accessibility/ACCESSIBILITY_TESTING_GUIDE.md)
- [Implementation Guide](accessibility/ACCESSIBILITY_IMPLEMENTATION_GUIDE.md)

### Security
- [Security Update Guide](security/SECURITY_UPDATE_GUIDE.md)
- [CSRF Protection](security/CSRF_PROTECTION.md)
- [Vulnerability Resolution](security/SECURITY_VULNERABILITY_RESOLUTION.md)

### Development Guides
- [Console Logging Removal](guides/CONSOLE_LOGGING_REMOVAL_GUIDE.md)
- [Keyboard Shortcuts](guides/KEYBOARD_SHORTCUTS.md)
- [Image Optimization](guides/IMAGE_OPTIMIZATION_GUIDE.md)
- [Color Contrast](guides/COLOR_CONTRAST_GUIDE.md)
- [Migration Guide](guides/MIGRATION_GUIDE.md)

## Build & Testing
- [Build Verification](FRONTEND_BUILD_VERIFICATION.md)
- [Testing Checklist](TESTING_CHECKLIST.md)
EOF

echo ""
echo "✅ Frontend documentation reorganized successfully!"
echo ""
echo "Summary:"
echo "  - Created docs/{accessibility,security,guides}/"
echo "  - Moved ~35 .md files from frontend/ root"
echo "  - Created docs/README.md index"
echo ""
echo "Next: Run 3-remove-unused-features.sh"
