# Accessibility Documentation

## Overview

This document provides information about the accessibility features implemented in the Stellar Insights frontend application and guidance for maintaining WCAG 2.1 AA compliance.

## Current Status

🎯 **Goal:** WCAG 2.1 Level AA Compliance  
📊 **Status:** In Progress  
🔧 **Priority:** High (Issue #297)

## Quick Start for Developers

### 1. Install Dependencies

```bash
npm install
```

This will install all accessibility testing tools including:
- `eslint-plugin-jsx-a11y` - ESLint rules for accessibility
- `@axe-core/react` - Runtime accessibility testing
- `jest-axe` - Accessibility testing in unit tests
- `vitest-axe` - Vitest integration for axe-core

### 2. Run Accessibility Linting

```bash
npm run lint:a11y
```

This will check your code for common accessibility issues.

### 3. Run Accessibility Tests

```bash
npm run test:a11y
```

This will run all accessibility-specific tests.

### 4. Manual Testing

Use browser extensions for manual testing:
- [axe DevTools](https://www.deque.com/axe/devtools/) - Chrome/Firefox
- [WAVE](https://wave.webaim.org/extension/) - Chrome/Firefox
- [Lighthouse](https://developers.google.com/web/tools/lighthouse) - Built into Chrome

## Key Accessibility Features

### ✅ Implemented

1. **Skip Navigation**
   - Skip links allow keyboard users to bypass repetitive navigation
   - Located in `src/components/SkipNavigation.tsx`

2. **Keyboard Navigation**
   - All interactive elements are keyboard accessible
   - Focus management in modals and menus
   - Escape key closes overlays

3. **Theme Support**
   - Light and dark themes with proper contrast
   - System preference detection
   - Smooth transitions

4. **Semantic HTML**
   - Proper heading hierarchy
   - Landmark regions (header, nav, main, footer)
   - Semantic elements throughout

5. **ARIA Support**
   - Proper ARIA labels and descriptions
   - Live regions for dynamic content
   - Status announcements

6. **Focus Indicators**
   - Visible focus indicators on all interactive elements
   - High contrast focus rings
   - Skip link focus styling

### 🚧 In Progress

1. **Color Contrast Audit**
   - Verifying all color combinations meet WCAG AA standards
   - Testing glassmorphism backgrounds

2. **Chart Accessibility**
   - Adding text alternatives for visualizations
   - Implementing data table alternatives
   - Keyboard navigation for interactive charts

3. **Form Improvements**
   - Enhanced error messaging
   - Better label associations
   - Inline validation

4. **Reduced Motion Support**
   - Respecting `prefers-reduced-motion` preference
   - Conditional animations

### 📋 Planned

1. **Screen Reader Testing**
   - Comprehensive testing with NVDA, JAWS, VoiceOver
   - User testing with assistive technology users

2. **Documentation**
   - Accessibility statement
   - Keyboard shortcuts guide
   - User guides for assistive technology

3. **Automated Testing**
   - CI/CD integration for accessibility tests
   - Automated contrast checking
   - Regular audits

## Development Guidelines

### Writing Accessible Components

#### 1. Always Use Semantic HTML

```tsx
// ✅ Good
<button onClick={handleClick}>Click me</button>

// ❌ Bad
<div onClick={handleClick}>Click me</div>
```

#### 2. Provide Text Alternatives

```tsx
// ✅ Good
<img src="chart.png" alt="Sales increased by 25% in Q4" />

// ❌ Bad
<img src="chart.png" />
```

#### 3. Associate Labels with Inputs

```tsx
// ✅ Good
<label htmlFor="email">Email</label>
<input id="email" type="email" />

// ❌ Bad
<div>Email</div>
<input type="email" />
```

#### 4. Use ARIA Appropriately

```tsx
// ✅ Good - Icon button with label
<button aria-label="Close dialog">
  <X aria-hidden="true" />
</button>

// ❌ Bad - No label
<button>
  <X />
</button>
```

#### 5. Manage Focus

```tsx
// ✅ Good - Focus trap in modal
const modalRef = useFocusTrap(isOpen);

return (
  <div ref={modalRef} role="dialog" aria-modal="true">
    {/* Modal content */}
  </div>
);
```

#### 6. Announce Dynamic Changes

```tsx
// ✅ Good - Live region for status
<div role="status" aria-live="polite">
  {loading ? "Loading..." : "Data loaded"}
</div>

// ❌ Bad - No announcement
<div>{loading ? "Loading..." : "Data loaded"}</div>
```

### Color Contrast Requirements

- **Normal text:** 4.5:1 contrast ratio
- **Large text (18pt+):** 3:1 contrast ratio
- **UI components:** 3:1 contrast ratio
- **Focus indicators:** 3:1 contrast ratio

Use tools like [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/) to verify.

### Keyboard Navigation Requirements

All interactive elements must be:
- Reachable via Tab key
- Activatable via Enter or Space
- Have visible focus indicators
- Not create keyboard traps

### Touch Target Requirements

All interactive elements must have:
- Minimum 44x44px touch target size
- Adequate spacing between targets
- Clear active/pressed states

## Testing Checklist

Before submitting a PR, ensure:

- [ ] ESLint accessibility rules pass
- [ ] Automated accessibility tests pass
- [ ] Manual keyboard navigation works
- [ ] Focus indicators are visible
- [ ] Color contrast meets WCAG AA
- [ ] Screen reader testing completed (if applicable)
- [ ] Touch targets are adequate
- [ ] ARIA attributes are correct

## Common Patterns

### Accessible Button

```tsx
import { Button } from '@/components/ui/button';

// Basic button
<Button>Click me</Button>

// Icon button
<Button aria-label="Close">
  <X aria-hidden="true" />
</Button>

// Loading button
<Button loading loadingText="Saving...">
  Save
</Button>
```

### Accessible Form Input

```tsx
import { FormInput } from '@/components/ui/FormInput';

<FormInput
  label="Email"
  type="email"
  required
  error={errors.email}
  helperText="We'll never share your email"
/>
```

### Accessible Modal

```tsx
import { Dialog } from '@/components/ui/Dialog';

<Dialog
  isOpen={isOpen}
  onClose={handleClose}
  title="Confirm Action"
  description="Are you sure you want to proceed?"
>
  <div>Modal content</div>
</Dialog>
```

### Accessible Chart

```tsx
import { AccessibleChart } from '@/components/charts/AccessibleChart';

<AccessibleChart
  title="Sales Data"
  description="Monthly sales for 2024"
  data={chartData}
>
  <LineChart data={chartData} />
</AccessibleChart>
```

## Resources

### Documentation
- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/)
- [WebAIM Resources](https://webaim.org/resources/)
- [A11y Project](https://www.a11yproject.com/)

### Testing Tools
- [axe DevTools](https://www.deque.com/axe/devtools/)
- [WAVE](https://wave.webaim.org/extension/)
- [Lighthouse](https://developers.google.com/web/tools/lighthouse)
- [Color Contrast Analyzer](https://www.tpgi.com/color-contrast-checker/)

### Screen Readers
- [NVDA](https://www.nvaccess.org/) - Free (Windows)
- [JAWS](https://www.freedomscientific.com/products/software/jaws/) - Paid (Windows)
- [VoiceOver](https://www.apple.com/accessibility/voiceover/) - Built-in (Mac/iOS)
- [TalkBack](https://support.google.com/accessibility/android/answer/6283677) - Built-in (Android)

### Learning Resources
- [Web Accessibility by Google](https://www.udacity.com/course/web-accessibility--ud891)
- [Accessibility Fundamentals](https://www.w3.org/WAI/fundamentals/)
- [Inclusive Components](https://inclusive-components.design/)

## Support

### Reporting Accessibility Issues

If you encounter an accessibility issue:

1. Check if it's already documented in `ACCESSIBILITY_AUDIT.md`
2. Create a new issue with the `accessibility` label
3. Include:
   - Description of the issue
   - WCAG criterion violated
   - Steps to reproduce
   - Suggested fix (if known)

### Getting Help

- Review the implementation guide: `ACCESSIBILITY_IMPLEMENTATION_GUIDE.md`
- Check the checklist: `ACCESSIBILITY_CHECKLIST.md`
- Ask in the team's accessibility channel
- Consult with the accessibility specialist

## Roadmap

### Phase 1: Foundation (Weeks 1-2) ✅
- [x] Set up testing tools
- [x] Create documentation
- [x] Implement skip navigation
- [x] Add semantic landmarks

### Phase 2: Core Improvements (Weeks 3-4) 🚧
- [ ] Fix color contrast issues
- [ ] Improve form accessibility
- [ ] Enhance keyboard navigation
- [ ] Add focus management

### Phase 3: Advanced Features (Weeks 5-6)
- [ ] Make charts accessible
- [ ] Implement reduced motion
- [ ] Add live announcements
- [ ] Enhance error handling

### Phase 4: Testing & Refinement (Weeks 7-8)
- [ ] Screen reader testing
- [ ] User testing
- [ ] Final audit
- [ ] Documentation updates

## Maintenance

### Regular Tasks

- Run accessibility tests before each release
- Review new components for accessibility
- Update documentation as features change
- Conduct quarterly accessibility audits
- Stay updated on WCAG guidelines

### Continuous Improvement

- Gather user feedback
- Monitor accessibility issues
- Update patterns and components
- Train team members
- Share learnings

---

**Last updated:** February 23, 2026  
**Maintained by:** Development Team  
**Contact:** accessibility@stellarinsights.com
