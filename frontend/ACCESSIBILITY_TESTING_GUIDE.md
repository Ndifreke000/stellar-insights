# Accessibility Testing Guide

This guide provides instructions for testing the accessibility improvements made to the application.

## Automated Testing

### Install Testing Tools

```bash
# Install axe-core for accessibility testing
npm install --save-dev @axe-core/cli @axe-core/react jest-axe

# Install testing library for React components
npm install --save-dev @testing-library/react @testing-library/jest-dom
```

### Run Automated Tests

```bash
# Run axe accessibility audit on running application
npx axe http://localhost:3000

# Run specific page audits
npx axe http://localhost:3000/corridors
npx axe http://localhost:3000/dashboard
npx axe http://localhost:3000/governance
```

## Manual Testing with Screen Readers

### macOS - VoiceOver

1. Enable VoiceOver: `Cmd + F5`
2. Navigate through the application:
   - Use `Tab` to move between interactive elements
   - Use `Shift + Tab` to move backwards
   - Use `Cmd + Left/Right Arrow` to navigate by element
3. Test key components:
   - Sidebar navigation (all links should be announced)
   - Modal dialogs (focus should trap inside modal)
   - Cards and data displays (metrics should be announced)
   - Forms (labels should be associated with inputs)

### Windows - NVDA (Free)

1. Download NVDA from https://www.nvaccess.org/
2. Start NVDA
3. Navigate using:
   - `Tab` for interactive elements
   - `H` for headings
   - `B` for buttons
   - `L` for links
   - `F` for form fields

### Windows - JAWS

1. Start JAWS
2. Navigate using similar commands to NVDA
3. Test all interactive elements

## Keyboard Navigation Testing

Test that all functionality is accessible via keyboard only:

### Sidebar Navigation
- [ ] Tab through all navigation items
- [ ] Press Enter to activate links
- [ ] Collapse/expand button works with keyboard
- [ ] Current page is announced with aria-current

### Modal Dialogs
- [ ] Focus moves to modal when opened
- [ ] Tab cycles through modal elements only (focus trap)
- [ ] Shift+Tab moves backwards through modal
- [ ] Escape key closes modal
- [ ] Focus returns to trigger element when closed

### Forms
- [ ] Tab moves between form fields
- [ ] Labels are announced for each field
- [ ] Required fields are indicated
- [ ] Error messages are announced
- [ ] Submit button is keyboard accessible

### Cards and Data Displays
- [ ] All interactive cards are keyboard accessible
- [ ] Metrics and status information is announced
- [ ] Progress bars announce current values

## WCAG 2.1 Level AA Compliance Checklist

### Perceivable

#### 1.1.1 Non-text Content (Level A)
- [x] All icons have `aria-hidden="true"`
- [x] All images have alt text or are decorative
- [x] All interactive elements have text labels

#### 1.3.1 Info and Relationships (Level A)
- [x] Proper heading hierarchy
- [x] Form labels associated with inputs
- [x] Lists use proper markup (`<ul>`, `<ol>`, `role="list"`)

#### 1.4.3 Contrast (Level AA)
- [ ] Text has minimum 4.5:1 contrast ratio
- [ ] Large text has minimum 3:1 contrast ratio
- [ ] Use browser extensions to verify contrast

### Operable

#### 2.1.1 Keyboard (Level A)
- [x] All functionality available via keyboard
- [x] No keyboard traps (except intentional focus traps in modals)

#### 2.4.3 Focus Order (Level A)
- [x] Focus order is logical and intuitive
- [x] Tab order follows visual layout

#### 2.4.4 Link Purpose (Level A)
- [x] All links have descriptive text or aria-label
- [x] Link purpose is clear from context

#### 2.4.7 Focus Visible (Level AA)
- [x] Focus indicators are visible
- [x] CSS includes focus styles

### Understandable

#### 3.2.1 On Focus (Level A)
- [x] No unexpected context changes on focus

#### 3.2.2 On Input (Level A)
- [x] No unexpected context changes on input

#### 3.3.1 Error Identification (Level A)
- [x] Form errors are identified and described
- [x] Error messages use `role="alert"`

#### 3.3.2 Labels or Instructions (Level A)
- [x] All form fields have labels
- [x] Required fields are indicated

### Robust

#### 4.1.2 Name, Role, Value (Level A)
- [x] All UI components have accessible names
- [x] Roles are properly assigned
- [x] States are communicated (aria-expanded, aria-current, etc.)

#### 4.1.3 Status Messages (Level AA)
- [x] Status messages use `role="status"` or `role="alert"`
- [x] Live regions are properly configured

## Component-Specific Testing

### Sidebar Component
```typescript
// Test checklist:
- Navigation has aria-label="Main navigation"
- Each link has aria-label describing destination
- Active page has aria-current="page"
- Icons have aria-hidden="true"
- Collapse button has aria-label and aria-expanded
- System status has role="status" and aria-live="polite"
```

### Modal Components
```typescript
// Test checklist:
- Modal has role="dialog" and aria-modal="true"
- Modal has aria-labelledby pointing to title
- Modal has aria-describedby pointing to content
- Focus trap works correctly
- Escape key closes modal
- Close button has aria-label
```

### Card Components
```typescript
// Test checklist:
- Cards use <article> or appropriate semantic element
- Headings have unique IDs for aria-labelledby
- Status indicators have role="status"
- Progress bars have role="progressbar" with aria-valuenow
- Metrics have descriptive aria-labels
```

## Browser Testing

Test in multiple browsers with accessibility tools:

### Chrome DevTools
1. Open DevTools (F12)
2. Go to Lighthouse tab
3. Select "Accessibility" category
4. Run audit

### Firefox Accessibility Inspector
1. Open DevTools (F12)
2. Go to Accessibility tab
3. Inspect element tree
4. Check for issues

### axe DevTools Extension
1. Install axe DevTools browser extension
2. Open extension in DevTools
3. Run full page scan
4. Review and fix issues

## Common Issues to Check

1. **Missing ARIA labels**: All interactive elements without visible text need aria-label
2. **Incorrect heading hierarchy**: Headings should not skip levels (h1 → h2 → h3)
3. **Missing form labels**: All inputs need associated labels
4. **Poor focus indicators**: Focus should be clearly visible
5. **Color contrast**: Text must meet minimum contrast ratios
6. **Keyboard traps**: Users should be able to navigate away from all elements
7. **Missing alt text**: Images need descriptive alt text or aria-hidden
8. **Improper ARIA usage**: Don't use ARIA when semantic HTML works

## Continuous Testing

Add accessibility tests to your CI/CD pipeline:

```json
// package.json
{
  "scripts": {
    "test:a11y": "jest --testMatch='**/*.a11y.test.ts'",
    "test:axe": "npx axe http://localhost:3000 --exit"
  }
}
```

## Resources

- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [MDN Accessibility](https://developer.mozilla.org/en-US/docs/Web/Accessibility)
- [WebAIM](https://webaim.org/)
- [A11y Project Checklist](https://www.a11yproject.com/checklist/)
- [axe DevTools](https://www.deque.com/axe/devtools/)
