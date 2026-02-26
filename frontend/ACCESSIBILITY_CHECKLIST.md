# WCAG 2.1 AA Accessibility Checklist

Quick reference checklist for ensuring accessibility compliance.

## 🎯 Quick Wins (Do These First)

- [ ] Install `eslint-plugin-jsx-a11y`
- [ ] Add `lang` attribute to `<html>` element
- [ ] Verify all images have alt text
- [ ] Ensure all form inputs have labels
- [ ] Add skip navigation links
- [ ] Test keyboard navigation on all pages
- [ ] Run WAVE or axe DevTools on all pages

---

## 📋 Perceivable

### Text Alternatives
- [ ] All images have alt text or are marked decorative
- [ ] Complex images have long descriptions
- [ ] Charts have text alternatives or data tables
- [ ] Icons have aria-label or are aria-hidden
- [ ] SVGs have title and desc elements

### Time-based Media
- [ ] Videos have captions
- [ ] Audio has transcripts
- [ ] Media players are keyboard accessible

### Adaptable
- [ ] Semantic HTML used throughout
- [ ] Heading hierarchy is logical (h1 → h2 → h3)
- [ ] Lists use proper markup (ul, ol, dl)
- [ ] Tables have proper headers and scope
- [ ] Form inputs have associated labels
- [ ] Landmark regions defined (header, nav, main, aside, footer)
- [ ] Reading order is logical

### Distinguishable
- [ ] Color contrast ratio ≥ 4.5:1 for normal text
- [ ] Color contrast ratio ≥ 3:1 for large text (18pt+)
- [ ] Color contrast ratio ≥ 3:1 for UI components
- [ ] Information not conveyed by color alone
- [ ] Text can be resized to 200% without loss of content
- [ ] Images of text avoided (use real text)
- [ ] Focus indicators visible and ≥ 3:1 contrast
- [ ] Minimum font size 12px (0.75rem)

---

## ⌨️ Operable

### Keyboard Accessible
- [ ] All functionality available via keyboard
- [ ] No keyboard traps
- [ ] Skip navigation links present
- [ ] Focus order is logical
- [ ] Focus visible on all interactive elements
- [ ] Keyboard shortcuts documented
- [ ] Custom widgets keyboard accessible

### Enough Time
- [ ] No time limits, or users can extend/disable
- [ ] Auto-updating content can be paused
- [ ] Session timeouts have warnings
- [ ] Notifications don't disappear too quickly

### Seizures and Physical Reactions
- [ ] No content flashes more than 3 times per second
- [ ] Animations respect prefers-reduced-motion
- [ ] Parallax effects can be disabled
- [ ] Auto-play videos can be paused

### Navigable
- [ ] Skip links to main content
- [ ] Page titles are descriptive and unique
- [ ] Focus order is logical
- [ ] Link purpose clear from text or context
- [ ] Multiple ways to find pages (nav, search, sitemap)
- [ ] Headings and labels are descriptive
- [ ] Current location indicated (breadcrumbs, active nav)
- [ ] Focus visible on all interactive elements

### Input Modalities
- [ ] Touch targets ≥ 44x44px
- [ ] Pointer gestures have keyboard alternatives
- [ ] Pointer cancellation (action on mouseup)
- [ ] Label in name matches visible label
- [ ] Motion actuation has alternatives

---

## 🧠 Understandable

### Readable
- [ ] Page language set with lang attribute
- [ ] Language changes marked with lang
- [ ] Unusual words defined
- [ ] Abbreviations expanded on first use
- [ ] Reading level appropriate (or simplified version available)

### Predictable
- [ ] Navigation consistent across pages
- [ ] Components identified consistently
- [ ] Focus doesn't trigger unexpected changes
- [ ] Input doesn't trigger unexpected changes
- [ ] Changes on request (not automatic)

### Input Assistance
- [ ] Form errors identified and described
- [ ] Labels and instructions provided
- [ ] Error suggestions provided
- [ ] Error prevention for legal/financial transactions
- [ ] Confirmation for data submission
- [ ] Form validation is helpful and clear

---

## 🔧 Robust

### Compatible
- [ ] HTML validates
- [ ] ARIA used correctly
- [ ] Name, role, value for all UI components
- [ ] Status messages use role="status" or aria-live
- [ ] Tested with multiple browsers
- [ ] Tested with multiple screen readers

---

## 🧪 Testing Checklist

### Automated Testing
- [ ] ESLint jsx-a11y plugin configured
- [ ] axe DevTools run on all pages (0 violations)
- [ ] WAVE run on all pages (0 errors)
- [ ] Lighthouse accessibility score ≥ 95
- [ ] HTML validation passes

### Keyboard Testing
- [ ] Tab through entire page
- [ ] All interactive elements reachable
- [ ] Focus order is logical
- [ ] No keyboard traps
- [ ] Escape closes modals/menus
- [ ] Enter/Space activates buttons
- [ ] Arrow keys work in custom widgets

### Screen Reader Testing
- [ ] Test with NVDA (Windows)
- [ ] Test with JAWS (Windows)
- [ ] Test with VoiceOver (Mac/iOS)
- [ ] Test with TalkBack (Android)
- [ ] All content announced correctly
- [ ] Form labels announced
- [ ] Error messages announced
- [ ] Dynamic content changes announced

### Visual Testing
- [ ] Zoom to 200% (no horizontal scroll)
- [ ] Zoom to 400% (mobile view)
- [ ] High contrast mode (Windows)
- [ ] Dark mode
- [ ] Light mode
- [ ] Color blindness simulation

### Mobile Testing
- [ ] Touch targets ≥ 44x44px
- [ ] Pinch to zoom enabled
- [ ] Orientation changes work
- [ ] Mobile screen readers work

---

## 📊 Component-Specific Checks

### Forms
- [ ] All inputs have labels
- [ ] Required fields marked
- [ ] Error messages associated with inputs
- [ ] aria-invalid on invalid inputs
- [ ] aria-describedby for help text
- [ ] Fieldsets for related inputs
- [ ] Clear submit button

### Buttons
- [ ] Descriptive text (not just "Click here")
- [ ] aria-label for icon-only buttons
- [ ] Disabled state clear
- [ ] Loading state announced
- [ ] Minimum 44x44px touch target

### Links
- [ ] Descriptive text
- [ ] External links indicated
- [ ] Opens in new tab announced
- [ ] Visited state distinguishable

### Modals/Dialogs
- [ ] role="dialog"
- [ ] aria-modal="true"
- [ ] aria-labelledby for title
- [ ] aria-describedby for description
- [ ] Focus trapped inside
- [ ] Escape closes modal
- [ ] Focus returns to trigger on close

### Tables
- [ ] Caption or aria-label
- [ ] th elements for headers
- [ ] scope attribute on headers
- [ ] Complex tables have ids/headers

### Charts/Graphs
- [ ] role="img" with aria-label
- [ ] Text alternative or data table
- [ ] Color not sole indicator
- [ ] Patterns in addition to colors

### Carousels
- [ ] Keyboard accessible
- [ ] Pause/play controls
- [ ] Current slide announced
- [ ] Navigation buttons labeled

### Menus
- [ ] role="menu" and role="menuitem"
- [ ] Arrow key navigation
- [ ] Escape closes menu
- [ ] Focus management

---

## 🎨 Design Checklist

### Color & Contrast
- [ ] Text contrast ≥ 4.5:1
- [ ] Large text contrast ≥ 3:1
- [ ] UI component contrast ≥ 3:1
- [ ] Focus indicator contrast ≥ 3:1
- [ ] Color not sole indicator

### Typography
- [ ] Minimum 12px font size
- [ ] Line height ≥ 1.5 for body text
- [ ] Paragraph spacing ≥ 2x font size
- [ ] Letter spacing adjustable
- [ ] Text alignment left (not justified)

### Layout
- [ ] Responsive design
- [ ] No horizontal scroll at 200% zoom
- [ ] Touch targets ≥ 44x44px
- [ ] Adequate spacing between elements
- [ ] Consistent layout across pages

### Interactive Elements
- [ ] Clear hover states
- [ ] Clear focus states
- [ ] Clear active states
- [ ] Clear disabled states
- [ ] Loading states indicated

---

## 📱 Mobile-Specific

- [ ] Touch targets ≥ 44x44px
- [ ] Pinch to zoom enabled
- [ ] Orientation changes supported
- [ ] Mobile screen readers tested
- [ ] Swipe gestures have alternatives
- [ ] No hover-only interactions

---

## 🚀 Performance & Accessibility

- [ ] Page loads in < 3 seconds
- [ ] Images optimized
- [ ] Lazy loading for images
- [ ] Skeleton screens for loading
- [ ] Error states handled gracefully
- [ ] Offline functionality (if applicable)

---

## 📚 Documentation

- [ ] Accessibility statement published
- [ ] Keyboard shortcuts documented
- [ ] Contact for accessibility issues
- [ ] Known issues documented
- [ ] Roadmap for improvements
- [ ] User guides for assistive tech users

---

## ✅ Sign-Off

### Development Team
- [ ] Code reviewed for accessibility
- [ ] Automated tests passing
- [ ] Manual testing completed
- [ ] Documentation updated

### QA Team
- [ ] Keyboard testing completed
- [ ] Screen reader testing completed
- [ ] Visual testing completed
- [ ] Mobile testing completed

### Accessibility Specialist
- [ ] WCAG 2.1 AA compliance verified
- [ ] User testing with assistive tech users
- [ ] Final audit completed
- [ ] Sign-off provided

---

## 🔗 Resources

- [WCAG 2.1 Quick Reference](https://www.w3.org/WAI/WCAG21/quickref/)
- [ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/)
- [WebAIM Checklist](https://webaim.org/standards/wcag/checklist)
- [A11y Project Checklist](https://www.a11yproject.com/checklist/)
- [Deque axe DevTools](https://www.deque.com/axe/devtools/)
- [WAVE Browser Extension](https://wave.webaim.org/extension/)

---

**Last updated:** February 23, 2026
