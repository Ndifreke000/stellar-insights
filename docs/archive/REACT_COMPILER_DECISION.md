# React Compiler Decision: Keep Disabled (For Now)

**Date:** 2026-07-25  
**Decision:** Disable React Compiler / Don't enable it at this time  
**Status:** Active

## Executive Summary

The React Compiler (experimental feature) offers automatic memoization but requires strict adherence to rules-of-hooks and immutability patterns. Analysis of this codebase shows **6 diagnostic categories that would conflict** with current patterns, concentrated in specific areas (chart libraries, external state). Rather than refactor extensive code to satisfy the compiler, we've decided to keep it disabled and rely on manual, targeted memoization where needed.

## Diagnostic Categories Found

If React Compiler were enabled, these 6 issues would surface:

| Issue | Count | Locations | Description |
|-------|-------|-----------|-------------|
| `react-hooks/preserve-manual-memoization` | 2 | Chart/analytics components | Existing `useMemo` that compiler can't automatically preserve |
| `react-hooks/immutability` | 2 | State management layers | Local state mutation patterns the compiler rejects |
| `react-hooks/purity` | 1 | Render-phase side effects | Impure function calls during component render |
| `react-hooks/incompatible-library` | 1 | External library usage | Third-party library patterns the compiler can't analyze |

**Total: 6 diagnostics** across chart libraries (recharts, d3-force-3d), state management (Zustand), and external dependencies.

## The Conflict: Architecture vs. Compiler

This codebase uses three patterns that conflict with React Compiler's constraints:

### 1. Chart Libraries (recharts, d3-force-3d)
- **Issue:** Complex components with refs, external canvas state, and side-effect-laden render paths
- **Why it conflicts:** Compiler requires pure renders; chart libraries do imperative DOM manipulation
- **Cost to fix:** Significant refactoring of chart components, possible loss of library features
- **Current state:** Manual memoization via `useMemo` works well

### 2. State Management (Zustand + Context)
- **Issue:** Zustand stores allow direct mutation (by design); compiler expects immutability
- **Why it conflicts:** Compiler can't verify immutability across store boundaries
- **Cost to fix:** Rewrite store selectors, add immutability wrappers, or switch state libraries
- **Current state:** Manual API-level memoization prevents re-renders effectively

### 3. External Libraries (Analytics, Tracking)
- **Issue:** Third-party libraries make external API calls during component mount/update
- **Why it conflicts:** Compiler can't analyze external library code, flags as "incompatible"
- **Cost to fix:** Abstract libraries into controlled hooks with explicit dependencies, or disable compiler for those modules
- **Current state:** Effects are properly defined; no correctness issues

## Tradeoffs

### Enabling React Compiler: Pros
- ✅ Automatic memoization eliminates manual `useMemo`/`useCallback` chains
- ✅ Compiler catches accidentally impure render code
- ✅ Future-proofs for React optimizations

### Enabling React Compiler: Cons
- ❌ Requires refactoring chart components (high effort, uncertain benefit)
- ❌ Incompatible with current Zustand usage patterns (state library choice)
- ❌ Need to disable or scope compiler around third-party integrations anyway
- ❌ Adds build-time complexity for incremental returns (most perf wins already achieved via manual memoization)

## Current Performance Status

We **already achieve good performance** without the compiler:
- Chart components memoized manually via `useMemo`
- Zustand selectors naturally prevent re-renders
- Route-level code splitting keeps initial bundle lean
- No reported re-render storms or performance complaints

The compiler would not provide measurable additional gains without the architectural changes above.

## Decision: Keep Disabled

**Why:** Refactoring cost (chart library compatibility, state management rewrites, external library abstractions) exceeds the benefit of automatic memoization when manual memoization already works well and performance is good.

**When to revisit:**
- If React Compiler matures and supports more patterns (refs, external libs)
- If team adopts immutable-first state management (Immer, new state library)
- If perf profiling reveals re-render bottlenecks manual memoization can't address
- If chart libraries release Compiler-compatible versions

## Documentation for Developers

If you want to check whether code would be Compiler-compatible, keep these patterns in mind:

### ✅ Compiler-friendly patterns (no changes needed)
```typescript
// Pure render, dependencies explicit
function Dashboard() {
  const data = useMemo(() => computeData(), [dependency]);
  return <Chart data={data} />;
}

// Immutable state updates
const [state, setState] = useState({});
setState(prev => ({ ...prev, field: newValue }));

// Effects with clear dependencies
useEffect(() => {
  api.track('event');
}, [userId]); // explicit dependency
```

### ⚠️ Compiler would flag these (but currently allowed)
```typescript
// Impure render (external API call in JSX)
function Header() {
  analytics.track('render'); // ❌ Side effect during render
  return <h1>Header</h1>;
}

// Direct mutation (Zustand pattern)
const store = create(set => ({
  count: 0,
  increment: () => set(state => { state.count++; }) // ❌ Mutates
}));

// Incompatible library (third-party ref manipulation)
function Graph() {
  const ref = useRef();
  useEffect(() => {
    d3.select(ref.current).render(data); // ❌ External lib does DOM traversal
  }, [data]);
}
```

### 🔧 Workarounds if you want Compiler-compatible code now
1. **Never call side effects during render** — move to `useEffect`
2. **Use immutable state update syntax** — `setState(s => ({...s, x: y}))`
3. **Wrap external libraries** — abstract into effect hooks with clear dependencies
4. **Use Compiler-compatible state** — consider Immer or refactor state management later

## See Also
- [React Compiler Docs](https://react.dev/learn/react-compiler)
- [rules-of-hooks ESLint plugin](https://www.npmjs.com/package/eslint-plugin-react-hooks)
- Current memoization approach: `src/components/dynamic-imports.ts` (code splitting strategy)
