#!/bin/bash

# Image Optimization Verification Script
# Checks for unoptimized <img> tags and provides optimization report

echo "🔍 Image Optimization Verification"
echo "=================================="
echo ""

# Check for <img> tags in source files (excluding tests)
echo "1. Checking for standard <img> tags..."
IMG_COUNT=$(find src -type f \( -name "*.tsx" -o -name "*.ts" -o -name "*.jsx" -o -name "*.js" \) ! -path "*/\__tests__/*" ! -path "*/*.test.*" ! -path "*/*.spec.*" -exec grep -l "<img" {} \; 2>/dev/null | wc -l)

if [ "$IMG_COUNT" -eq 0 ]; then
  echo "   ✅ No unoptimized <img> tags found in source files"
else
  echo "   ⚠️  Found $IMG_COUNT file(s) with <img> tags:"
  find src -type f \( -name "*.tsx" -o -name "*.ts" -o -name "*.jsx" -o -name "*.js" \) ! -path "*/\__tests__/*" ! -path "*/*.test.*" ! -path "*/*.spec.*" -exec grep -l "<img" {} \; 2>/dev/null
  echo ""
  echo "   Consider replacing with Next.js <Image> component"
fi
echo ""

# Check for Next.js Image component usage
echo "2. Checking Next.js Image component usage..."
IMAGE_COUNT=$(find src -type f \( -name "*.tsx" -o -name "*.ts" \) -exec grep -l "from 'next/image'" {} \; 2>/dev/null | wc -l)
echo "   ✅ Found $IMAGE_COUNT file(s) using Next.js Image component"
echo ""

# Check next.config.ts for image optimization
echo "3. Checking next.config.ts configuration..."
if grep -q "images:" next.config.ts 2>/dev/null; then
  echo "   ✅ Image optimization configured in next.config.ts"
  
  if grep -q "formats:" next.config.ts; then
    echo "   ✅ Modern formats (WebP/AVIF) enabled"
  fi
  
  if grep -q "remotePatterns:" next.config.ts; then
    echo "   ✅ Remote patterns configured for external images"
  fi
else
  echo "   ⚠️  Image optimization not configured in next.config.ts"
  echo "   Add image configuration to enable optimization"
fi
echo ""

# Check for image files in public directory
echo "4. Checking public directory for images..."
if [ -d "public" ]; then
  PNG_COUNT=$(find public -type f -name "*.png" 2>/dev/null | wc -l)
  JPG_COUNT=$(find public -type f \( -name "*.jpg" -o -name "*.jpeg" \) 2>/dev/null | wc -l)
  SVG_COUNT=$(find public -type f -name "*.svg" 2>/dev/null | wc -l)
  WEBP_COUNT=$(find public -type f -name "*.webp" 2>/dev/null | wc -l)
  
  echo "   📊 Image inventory:"
  echo "      PNG:  $PNG_COUNT files"
  echo "      JPG:  $JPG_COUNT files"
  echo "      SVG:  $SVG_COUNT files (no optimization needed)"
  echo "      WebP: $WEBP_COUNT files"
else
  echo "   ℹ️  No public directory found"
fi
echo ""

# Check for build output
echo "5. Checking build optimization..."
if [ -d ".next/cache/images" ]; then
  OPTIMIZED_COUNT=$(find .next/cache/images -type f 2>/dev/null | wc -l)
  echo "   ✅ Found $OPTIMIZED_COUNT optimized images in cache"
  
  if [ "$OPTIMIZED_COUNT" -gt 0 ]; then
    echo "   📦 Cache size: $(du -sh .next/cache/images 2>/dev/null | cut -f1)"
  fi
else
  echo "   ℹ️  No optimized images cache found (run 'npm run build' first)"
fi
echo ""

# Summary
echo "=================================="
echo "📋 Summary"
echo "=================================="

if [ "$IMG_COUNT" -eq 0 ] && grep -q "images:" next.config.ts 2>/dev/null; then
  echo "✅ Image optimization is fully implemented!"
  echo ""
  echo "Next steps:"
  echo "  1. Run 'npm run build' to generate optimized images"
  echo "  2. Run 'npx lighthouse http://localhost:3000' to test performance"
  echo "  3. Check .next/cache/images for WebP/AVIF files"
else
  echo "⚠️  Image optimization needs attention"
  echo ""
  echo "Action items:"
  [ "$IMG_COUNT" -gt 0 ] && echo "  - Replace <img> tags with Next.js <Image> component"
  ! grep -q "images:" next.config.ts 2>/dev/null && echo "  - Configure image optimization in next.config.ts"
fi
echo ""

# Performance tips
echo "💡 Performance Tips:"
echo "  - Use 'priority' prop for above-fold images"
echo "  - Specify width/height to prevent layout shift"
echo "  - Use 'fill' for responsive container images"
echo "  - Add 'sizes' prop for responsive images"
echo "  - Configure remotePatterns for external images"
echo ""

echo "📚 Documentation:"
echo "  - IMAGE_OPTIMIZATION_GUIDE.md - Detailed guide"
echo "  - IMAGE_OPTIMIZATION_QUICK_REFERENCE.md - Quick reference"
echo ""
