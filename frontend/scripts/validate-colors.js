#!/usr/bin/env node

/**
 * Color Contrast Validation Script
 * Validates that all colors meet WCAG AA standards
 */

function getLuminance(hex) {
  const color = hex.replace('#', '');
  const rgb = parseInt(color, 16);
  const r = ((rgb >> 16) & 0xff) / 255;
  const g = ((rgb >> 8) & 0xff) / 255;
  const b = (rgb & 0xff) / 255;
  
  const [rs, gs, bs] = [r, g, b].map(c =>
    c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
  );
  
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

function getContrastRatio(fg, bg) {
  const l1 = getLuminance(fg);
  const l2 = getLuminance(bg);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

function meetsWCAG_AA(ratio, isLargeText = false) {
  return isLargeText ? ratio >= 3 : ratio >= 4.5;
}

// Color definitions
const darkMode = {
  background: '#020617',
  colors: {
    'text-primary': '#f8fafc',
    'text-secondary': '#e2e8f0',
    'text-muted': '#cbd5e1',
    'text-disabled': '#94a3b8',
    'muted-foreground': '#cbd5e1',
    'link-primary': '#60a5fa',
    'link-hover': '#93c5fd',
    'success': '#34d399',
    'warning': '#fbbf24',
    'error': '#f87171',
  }
};

const lightMode = {
  background: '#f8fafc',
  colors: {
    'text-primary': '#0f172a',
    'text-secondary': '#1e293b',
    'text-muted': '#334155',
    'text-disabled': '#475569',
    'muted-foreground': '#334155',
    'link-primary': '#2563eb',
    'link-hover': '#1d4ed8',
    'success': '#047857',
    'warning': '#b45309',
    'error': '#dc2626',
  }
};

console.log('🎨 WCAG AA Color Contrast Validation\n');
console.log('=' .repeat(70));

let allPassed = true;

function validateMode(mode, modeName) {
  console.log(`\n${modeName} Mode:`);
  console.log('-'.repeat(70));
  
  for (const [name, color] of Object.entries(mode.colors)) {
    const ratio = getContrastRatio(color, mode.background);
    const passes = meetsWCAG_AA(ratio);
    const status = passes ? '✅' : '❌';
    const level = ratio >= 7 ? 'AAA' : ratio >= 4.5 ? 'AA' : 'FAIL';
    
    console.log(
      `${status} ${name.padEnd(20)} ${color.padEnd(10)} ${ratio.toFixed(2)}:1 (${level})`
    );
    
    if (!passes) {
      allPassed = false;
    }
  }
}

validateMode(darkMode, 'Dark');
validateMode(lightMode, 'Light');

console.log('\n' + '='.repeat(70));

if (allPassed) {
  console.log('✅ All colors meet WCAG AA standards!');
  process.exit(0);
} else {
  console.log('❌ Some colors do not meet WCAG AA standards');
  process.exit(1);
}
