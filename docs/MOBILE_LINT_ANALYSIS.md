# Mobile Lint Analysis and Action Plan

**Issue #1883:** Mobile lint pass parallel to the frontend ESLint cleanup

## Scope

**244 TypeScript/TSX files** in `mobile/src/` requiring linting and fixes.

## Environment Note

Linting execution requires `npm install` and `npm run lint` which depend on working network connectivity. This document provides the analysis framework and action plan for execution when environment is ready.

## Lint Configuration

**ESLint Config:** `mobile/.eslintrc.js`
- Extends: `@react-native`
- Parser: `@typescript-eslint/parser`
- Active rules:
  - `react-native/no-inline-styles: warn` — Catches inline style objects
  - `@typescript-eslint/no-unused-vars: error` — Catches unused variables, with `argsIgnorePattern: '^_'`

**TypeScript Config:** `mobile/tsconfig.json`
- `strict: true` — Enables all type checking
- Path aliases for imports: `@components/*`, `@screens/*`, `@services/*`, etc.

## Execution Plan

### Phase 1: Run Lint and Capture Findings

```bash
cd mobile
npm install
npm run lint > ../lint-findings.txt 2>&1
npm run lint 2>&1 | grep -E "error|warning" | sort | uniq -c | sort -rn
```

Record:
- Total error count
- Total warning count
- Breakdown by rule (which rules fire most frequently)
- Files with most violations

### Phase 2: Fix Unused Variables/Imports First (Mechanical Approach)

Following the same approach as frontend cleanup (#1880-series):

1. **Identify unused variables:** Files with `@typescript-eslint/no-unused-vars` errors
   - Search pattern: Variables declared but never referenced
   - Quick fix: Delete or prefix with `_` if used as required parameter

2. **Identify unused imports:** Files with unreferenced imports
   - Search pattern: `import X from 'Y'` where X is never used in the file
   - Quick fix: Remove import line

3. **Fix inline styles:** `react-native/no-inline-styles` warnings
   - Search pattern: `style={{ ... }}` style objects defined inline
   - Medium fix: Extract to StyleSheet.create() at module level

### Phase 3: Triage Remaining Findings

Non-mechanical fixes:
- Type errors requiring schema updates
- Complex refactorings
- Component API changes

**File individual issues** for each file/rule combination if finding volume warrants it (mirror frontend's per-file pattern).

## Analysis of Likely Findings

Based on code inspection of `mobile/src/navigation/MainNavigator.tsx`:

### Example 1: Unused Imports

```typescript
// MainNavigator.tsx lines 1-46
import { VRSupportComponent } from '@components/VRSupportComponent';
import { NFCSupportComponent } from '@components/NFCSupportComponent';
import { BluetoothSupportComponent } from '@components/BluetoothSupportComponent';
// ... etc (many imports, but components are used in Tab.Screen elements)
```

All components are actually used, but the pattern suggests potential for unused imports in other files.

### Example 2: Unused Variables

Common patterns likely to be found:
```typescript
// Unused state setter
const [value, setValue] = React.useState(false);  // if setValue never called

// Unused loop variable
data.map((item, index) => item.name)  // index unused; should be _ or remove parameter
```

### Example 3: Inline Styles (Warnings)

```typescript
<View style={{ flex: 1, justifyContent: 'center', alignItems: 'center' }}>
  // Should extract to:
  const styles = StyleSheet.create({
    container: { flex: 1, justifyContent: 'center', alignItems: 'center' },
  });
  // Then: <View style={styles.container}>
```

## Frontend Lint Cleanup Reference

Per issue #1880 series (frontend), the mechanical cleanup found:
- ~170 unused-vars findings (fixed via auto-fix or deletion)
- ~55 files with remaining findings (filed as individual follow-up issues)

**Expected mobile scope:** 100-250 findings across 244 files (43% file coverage)

## Fix Commands (Automated Where Possible)

```bash
# ESLint auto-fix for mechanical violations
cd mobile
npx eslint . --ext .js,.jsx,.ts,.tsx --fix

# TypeScript type-check after fixes
npx tsc --noEmit

# Re-run lint to see remaining issues
npm run lint
```

## Commit Strategy

**Single commit per category:**

1. **Commit 1:** Unused variables/imports (mechanical fixes)
   ```
   lint: remove unused variables and imports in mobile (#1883)
   - Fixed @typescript-eslint/no-unused-vars: N findings
   - Fixed unused imports across M files
   ```

2. **Commit 2:** Inline styles (if volume warrants)
   ```
   lint: extract inline styles to StyleSheet.create() in mobile
   - Fixed react-native/no-inline-styles: N warnings
   - Created style objects in X files
   ```

3. **Follow-up issues:** For complex/blocking fixes
   - One issue per file/rule if >10 similar findings
   - Example: #1886-mobile-type-safety-X, #1887-mobile-complex-refactor-Y

## Success Criteria

- [ ] `npm run lint` returns 0 errors (warnings acceptable per ESLint config)
- [ ] `npx tsc --noEmit` passes without type errors
- [ ] All commits reference #1883 in message
- [ ] Frontend Lint workflow passes on PR
- [ ] No regressions in existing functionality

## Related Issues

- #1880 series: Frontend lint cleanup (170 unused-vars fixes, 55 remaining files)
- #1883 mobile: Lint pass (this issue)
- #1883-mobile-*: Follow-up per-file issues (if needed)

## Timeline

- **Week 1:** Run lint, capture findings, create action items
- **Week 2:** Mechanical fixes (unused vars/imports)
- **Week 3:** Style extraction and complex refactors
- **Week 4:** Verify all fixes, create follow-up issues, close #1883

## Appendix: Mobile File Structure

```
mobile/src/
├── config/           (5 files) — app configuration
├── store/            (2 files) — Zustand state stores
├── navigation/       (4 files) — navigation structure
├── components/       (200+ files) — React Native components
├── screens/          (15+ files) — screen components
├── services/         (10+ files) — API and services
├── hooks/            (5+ files) — custom hooks
├── utils/            (3+ files) — utility functions
├── types/            (1 file) — TypeScript type definitions
└── App.tsx          (1 file) — root component
```

**Highest risk areas for lint findings:**
1. `components/` (200+ files) — most violations expected here
2. `services/` — API integration code
3. `screens/` — screen logic
4. `App.tsx`, `navigation/` — potential unused imports

## Tools & Resources

- ESLint: https://eslint.org/docs/latest/use/configure/
- React Native ESLint: https://github.com/Intellicode/eslint-plugin-react-native
- TypeScript ESLint: https://typescript-eslint.io/
- Frontend cleanup reference: Issue #1880 (similar approach, 170 findings fixed)
