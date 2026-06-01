# Accessibility Quick Reference

Quick reference for implementing accessible components in the application.

## Common Patterns

### Navigation Links
```tsx
<nav aria-label="Main navigation">
  <Link
    href="/path"
    aria-label="Navigate to Dashboard"
    aria-current={isActive ? 'page' : undefined}
  >
    <Icon aria-hidden="true" />
    <span>Dashboard</span>
  </Link>
</nav>
```

### Buttons with Icons Only
```tsx
<button
  onClick={handleClick}
  aria-label="Close modal"
>
  <X aria-hidden="true" />
</button>
```

### Modal Dialogs
```tsx
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby="modal-title"
  aria-describedby="modal-description"
>
  <h2 id="modal-title">Modal Title</h2>
  <div id="modal-description">
    {/* Modal content */}
  </div>
  <button onClick={onClose} aria-label="Close modal">
    Close
  </button>
</div>
```

### Form Fields
```tsx
<div>
  <label htmlFor="email-input">Email Address</label>
  <input
    id="email-input"
    type="email"
    required
    aria-required="true"
    aria-describedby="email-help"
  />
  <span id="email-help">We'll never share your email</span>
</div>
```

### Progress Bars
```tsx
<div
  role="progressbar"
  aria-valuenow={75}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-label="Upload progress: 75%"
>
  <div style={{ width: '75%' }} />
</div>
```

### Status Messages
```tsx
{/* Polite announcement (non-critical) */}
<div role="status" aria-live="polite">
  Data updated successfully
</div>

{/* Assertive announcement (critical) */}
<div role="alert" aria-live="assertive">
  Error: Failed to save changes
</div>
```

### Cards/Articles
```tsx
<article aria-labelledby="card-title">
  <h3 id="card-title">Card Title</h3>
  <div role="status" aria-label="Health score: 95%">
    <ProgressBar value={95} />
  </div>
</article>
```

### Lists
```tsx
<ul role="list">
  <li>Item 1</li>
  <li>Item 2</li>
</ul>
```

### Decorative Icons
```tsx
{/* Always hide decorative icons from screen readers */}
<Icon aria-hidden="true" />
```

### Interactive Icons
```tsx
{/* Icons with meaning need labels */}
<button aria-label="Delete item">
  <Trash aria-hidden="true" />
</button>
```

## ARIA Attributes Cheat Sheet

| Attribute | Purpose | Example |
|-----------|---------|---------|
| `aria-label` | Provides accessible name | `<button aria-label="Close">` |
| `aria-labelledby` | References element(s) for name | `<div aria-labelledby="title">` |
| `aria-describedby` | References element(s) for description | `<input aria-describedby="help">` |
| `aria-hidden` | Hides from screen readers | `<Icon aria-hidden="true">` |
| `aria-current` | Indicates current item | `<Link aria-current="page">` |
| `aria-expanded` | Indicates expanded state | `<button aria-expanded={isOpen}>` |
| `aria-controls` | References controlled element | `<button aria-controls="menu">` |
| `aria-live` | Announces dynamic changes | `<div aria-live="polite">` |
| `aria-modal` | Indicates modal dialog | `<div aria-modal="true">` |
| `aria-required` | Indicates required field | `<input aria-required="true">` |
| `aria-invalid` | Indicates validation error | `<input aria-invalid="true">` |

## Keyboard Navigation

### Focus Management
```tsx
// Focus trap in modal
useEffect(() => {
  const focusableElements = modalRef.current?.querySelectorAll(
    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
  );
  
  const firstElement = focusableElements?.[0] as HTMLElement;
  const lastElement = focusableElements?.[focusableElements.length - 1] as HTMLElement;
  
  firstElement?.focus();
  
  const handleTab = (e: KeyboardEvent) => {
    if (e.key === 'Tab') {
      if (e.shiftKey && document.activeElement === firstElement) {
        e.preventDefault();
        lastElement?.focus();
      } else if (!e.shiftKey && document.activeElement === lastElement) {
        e.preventDefault();
        firstElement?.focus();
      }
    }
    
    if (e.key === 'Escape') {
      onClose();
    }
  };
  
  document.addEventListener('keydown', handleTab);
  return () => document.removeEventListener('keydown', handleTab);
}, [onClose]);
```

### Keyboard Event Handlers
```tsx
// Handle Enter and Space for custom interactive elements
const handleKeyDown = (e: React.KeyboardEvent) => {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    handleClick();
  }
};

<div
  role="button"
  tabIndex={0}
  onClick={handleClick}
  onKeyDown={handleKeyDown}
>
  Custom Button
</div>
```

## Semantic HTML

Use semantic HTML elements when possible:

| Instead of | Use |
|------------|-----|
| `<div>` for navigation | `<nav>` |
| `<div>` for main content | `<main>` |
| `<div>` for articles | `<article>` |
| `<div>` for sections | `<section>` |
| `<div>` for headers | `<header>` |
| `<div>` for footers | `<footer>` |
| `<div>` for asides | `<aside>` |
| `<div role="button">` | `<button>` |
| `<div role="link">` | `<a>` |

## Testing Checklist

- [ ] All interactive elements keyboard accessible
- [ ] All icons either have labels or aria-hidden
- [ ] All form fields have associated labels
- [ ] All images have alt text or are decorative
- [ ] Focus indicators visible
- [ ] Color contrast meets WCAG AA (4.5:1 for text)
- [ ] Heading hierarchy is logical (no skipped levels)
- [ ] ARIA attributes used correctly
- [ ] Screen reader announces all important information
- [ ] No keyboard traps (except intentional in modals)

## Common Mistakes to Avoid

❌ **Don't**: Use `aria-label` on non-interactive elements
```tsx
<div aria-label="Card">Content</div>
```

✅ **Do**: Use semantic HTML or aria-labelledby
```tsx
<article aria-labelledby="card-title">
  <h3 id="card-title">Card Title</h3>
</article>
```

---

❌ **Don't**: Forget to hide decorative icons
```tsx
<button>
  <Icon /> Delete
</button>
```

✅ **Do**: Always hide decorative icons
```tsx
<button>
  <Icon aria-hidden="true" /> Delete
</button>
```

---

❌ **Don't**: Use placeholder as label
```tsx
<input placeholder="Email address" />
```

✅ **Do**: Always provide a label
```tsx
<label htmlFor="email">Email address</label>
<input id="email" placeholder="you@example.com" />
```

---

❌ **Don't**: Use div for buttons
```tsx
<div onClick={handleClick}>Click me</div>
```

✅ **Do**: Use button element
```tsx
<button onClick={handleClick}>Click me</button>
```

## Resources

- [ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/)
- [WebAIM WCAG Checklist](https://webaim.org/standards/wcag/checklist)
- [MDN Accessibility](https://developer.mozilla.org/en-US/docs/Web/Accessibility)
- [A11y Project](https://www.a11yproject.com/)

## Tools

- [axe DevTools](https://www.deque.com/axe/devtools/) - Browser extension
- [WAVE](https://wave.webaim.org/) - Web accessibility evaluation tool
- [Lighthouse](https://developers.google.com/web/tools/lighthouse) - Built into Chrome DevTools
- [Color Contrast Checker](https://webaim.org/resources/contrastchecker/)
