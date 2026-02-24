import { getContrastRatio, meetsWCAG_AA } from './src/utils/contrast-checker';

const tests = [
    { name: 'Light Mode Primary', fg: '#111827', bg: '#ffffff' },
    { name: 'Light Mode Secondary', fg: '#374151', bg: '#ffffff' },
    { name: 'Light Mode Muted', fg: '#4b5563', bg: '#ffffff' },
    { name: 'Light Mode Link', fg: '#2563eb', bg: '#ffffff' },
    { name: 'Dark Mode Primary', fg: '#f9fafb', bg: '#0a0a0a' },
    { name: 'Dark Mode Secondary', fg: '#e5e7eb', bg: '#0a0a0a' },
    { name: 'Dark Mode Muted', fg: '#d1d5db', bg: '#0a0a0a' },
    { name: 'Dark Mode Link', fg: '#60a5fa', bg: '#0a0a0a' },
];

console.log('--- WCAG Contrast Ratio Verification ---');
let allPassed = true;

tests.forEach(test => {
    const ratio = getContrastRatio(test.fg, test.bg);
    const passed = meetsWCAG_AA(ratio);
    console.log(`${passed ? '✅' : '❌'} ${test.name}: ${ratio.toFixed(2)}:1`);
    if (!passed) allPassed = false;
});

if (allPassed) {
    console.log('\nAll tests passed! Colors are WCAG 2.1 Level AA compliant.');
} else {
    console.log('\nSome tests failed. Please check the ratios.');
    process.exit(1);
}
