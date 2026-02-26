# Color Contrast Quick Reference

## Quick Test
```bash
npm run test:contrast
```

## WCAG AA Standards
- **Normal text**: 4.5:1 minimum
- **Large text**: 3:1 minimum
- **UI components**: 3:1 minimum

## Color Variables

### Text Colors
```css
/* Use these for text */
color: var(--text-primary);      /* Highest contrast */
color: var(--text-secondary);    /* High contrast */
color: var(--text-muted);        /* Medium contrast */
color: var(--text-disabled);     /* Lower contrast (still AA) */
color: var(--muted-foreground);  /* Muted text */
```

### Link Colors
```css
/* Use these for links */
color: var(--link-primary);
color: var(--link-hover);  /* On hover */
```

### Status Colors
```css
/* Use these for status indicators */
color: var(--success);  /* Green */
color: var(--warning);  /* Amber */
color: var(--error);    /* Red */
```

## Tailwind Classes

```tsx
// Existing classes (now WCAG compliant)
<p className="text-muted-foreground">Muted text</p>
<p className="text-foreground">Primary text</p>

// Custom color classes
<p className="text-[var(--text-secondary)]">Secondary text</p>
<a className="text-[var(--link-primary)]">Link</a>
<span className="text-[var(--success)]">Success</span>
```

## Testing New Colors

```typescript
import { validateColorPair } from '@/utils/contrast-checker';

const result = validateColorPair('#yourcolor', '#background');
console.log(result.meetsAA); // true/false
console.log(result.formatted); // "5.24:1"
```

## All Colors Pass ✅

- Dark mode: 10/10 colors pass AA (most AAA)
- Light mode: 10/10 colors pass AA (most AAA)

## Resources

- Full guide: `COLOR_CONTRAST_GUIDE.md`
- Issue resolution: `ISSUE_496_RESOLUTION.md`
- Validation script: `scripts/validate-colors.js`
- Utility: `src/utils/contrast-checker.ts`
