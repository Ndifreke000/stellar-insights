# Image Optimization Verification Script (PowerShell)
# Checks for unoptimized img tags and provides optimization report

Write-Host "Image Optimization Verification" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

# Check for img tags in source files (excluding tests)
Write-Host "1. Checking for standard img tags..." -ForegroundColor Yellow
$imgPattern = "<img"
$imgFiles = @(Get-ChildItem -Path src -Recurse -Include *.tsx,*.ts,*.jsx,*.js -Exclude *test*,*spec* -ErrorAction SilentlyContinue | 
    Select-String -Pattern $imgPattern -List | 
    Select-Object -ExpandProperty Path -Unique)

if ($imgFiles.Count -eq 0) {
    Write-Host "   No unoptimized img tags found in source files" -ForegroundColor Green
} else {
    Write-Host "   Found $($imgFiles.Count) file(s) with img tags:" -ForegroundColor Yellow
    $imgFiles | ForEach-Object { Write-Host "      $_" }
    Write-Host ""
    Write-Host "   Consider replacing with Next.js Image component" -ForegroundColor Yellow
}
Write-Host ""

# Check for Next.js Image component usage
Write-Host "2. Checking Next.js Image component usage..." -ForegroundColor Yellow
$imageFiles = @(Get-ChildItem -Path src -Recurse -Include *.tsx,*.ts -ErrorAction SilentlyContinue | 
    Select-String -Pattern "from 'next/image'" -List | 
    Select-Object -ExpandProperty Path -Unique)

Write-Host "   Found $($imageFiles.Count) file(s) using Next.js Image component" -ForegroundColor Green
Write-Host ""

# Check next.config.ts for image optimization
Write-Host "3. Checking next.config.ts configuration..." -ForegroundColor Yellow
if (Test-Path "next.config.ts") {
    $configContent = Get-Content "next.config.ts" -Raw
    
    if ($configContent -match "images:") {
        Write-Host "   Image optimization configured in next.config.ts" -ForegroundColor Green
        
        if ($configContent -match "formats:") {
            Write-Host "   Modern formats (WebP/AVIF) enabled" -ForegroundColor Green
        }
        
        if ($configContent -match "remotePatterns:") {
            Write-Host "   Remote patterns configured for external images" -ForegroundColor Green
        }
    } else {
        Write-Host "   Image optimization not configured in next.config.ts" -ForegroundColor Yellow
        Write-Host "   Add image configuration to enable optimization" -ForegroundColor Yellow
    }
} else {
    Write-Host "   next.config.ts not found" -ForegroundColor Yellow
}
Write-Host ""

# Check for image files in public directory
Write-Host "4. Checking public directory for images..." -ForegroundColor Yellow
if (Test-Path "public") {
    $pngCount = @(Get-ChildItem -Path public -Recurse -Filter *.png -ErrorAction SilentlyContinue).Count
    $jpgCount = @(Get-ChildItem -Path public -Recurse -Include *.jpg,*.jpeg -ErrorAction SilentlyContinue).Count
    $svgCount = @(Get-ChildItem -Path public -Recurse -Filter *.svg -ErrorAction SilentlyContinue).Count
    $webpCount = @(Get-ChildItem -Path public -Recurse -Filter *.webp -ErrorAction SilentlyContinue).Count
    
    Write-Host "   Image inventory:" -ForegroundColor Cyan
    Write-Host "      PNG:  $pngCount files"
    Write-Host "      JPG:  $jpgCount files"
    Write-Host "      SVG:  $svgCount files (no optimization needed)"
    Write-Host "      WebP: $webpCount files"
} else {
    Write-Host "   No public directory found" -ForegroundColor Gray
}
Write-Host ""

# Check for build output
Write-Host "5. Checking build optimization..." -ForegroundColor Yellow
if (Test-Path ".next/cache/images") {
    $optimizedCount = @(Get-ChildItem -Path .next/cache/images -Recurse -File -ErrorAction SilentlyContinue).Count
    Write-Host "   Found $optimizedCount optimized images in cache" -ForegroundColor Green
    
    if ($optimizedCount -gt 0) {
        $cacheSize = (Get-ChildItem -Path .next/cache/images -Recurse -ErrorAction SilentlyContinue | 
            Measure-Object -Property Length -Sum).Sum / 1MB
        Write-Host "   Cache size: $([math]::Round($cacheSize, 2)) MB"
    }
} else {
    Write-Host "   No optimized images cache found (run 'npm run build' first)" -ForegroundColor Gray
}
Write-Host ""

# Summary
Write-Host "================================" -ForegroundColor Cyan
Write-Host "Summary" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan

$configExists = Test-Path "next.config.ts"
$hasImageConfig = $false
if ($configExists) {
    $configContent = Get-Content "next.config.ts" -Raw
    $hasImageConfig = $configContent -match "images:"
}

if ($imgFiles.Count -eq 0 -and $hasImageConfig) {
    Write-Host "Image optimization is fully implemented!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Cyan
    Write-Host "  1. Run 'npm run build' to generate optimized images"
    Write-Host "  2. Run 'npx lighthouse http://localhost:3000' to test performance"
    Write-Host "  3. Check .next/cache/images for WebP/AVIF files"
} else {
    Write-Host "Image optimization needs attention" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Action items:" -ForegroundColor Cyan
    if ($imgFiles.Count -gt 0) {
        Write-Host "  - Replace img tags with Next.js Image component"
    }
    if (-not $hasImageConfig) {
        Write-Host "  - Configure image optimization in next.config.ts"
    }
}
Write-Host ""

# Performance tips
Write-Host "Performance Tips:" -ForegroundColor Cyan
Write-Host "  - Use 'priority' prop for above-fold images"
Write-Host "  - Specify width/height to prevent layout shift"
Write-Host "  - Use 'fill' for responsive container images"
Write-Host "  - Add 'sizes' prop for responsive images"
Write-Host "  - Configure remotePatterns for external images"
Write-Host ""

Write-Host "Documentation:" -ForegroundColor Cyan
Write-Host "  - IMAGE_OPTIMIZATION_GUIDE.md - Detailed guide"
Write-Host "  - IMAGE_OPTIMIZATION_QUICK_REFERENCE.md - Quick reference"
Write-Host ""
