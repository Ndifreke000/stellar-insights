import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';

// ─── Stub performance API ─────────────────────────────────────────────────────
const marks: Record<string, number> = {};
const measures: { name: string; duration: number }[] = [];

const perfStub = {
  now: vi.fn(() => Date.now()),
  mark: vi.fn((name: string) => { marks[name] = Date.now(); }),
  measure: vi.fn((_name: string, start: string, _end: string) => {
    const duration = marks[_end] !== undefined && marks[start] !== undefined
      ? marks[_end] - marks[start]
      : 5;
    const entry = { name: _name, duration, startTime: 0, entryType: 'measure', toJSON: () => ({}) };
    measures.push({ name: _name, duration });
    return [entry];
  }),
  getEntriesByType: vi.fn((type: string) => {
    if (type === 'navigation') {
      return [{
        requestStart: 10, responseStart: 50,
        domContentLoadedEventEnd: 300, startTime: 0,
        loadEventEnd: 500,
        domainLookupStart: 0, domainLookupEnd: 5,
        connectStart: 5, connectEnd: 10,
      }];
    }
    return [];
  }),
  memory: {
    usedJSHeapSize: 10 * 1_048_576,
    totalJSHeapSize: 20 * 1_048_576,
    jsHeapSizeLimit: 100 * 1_048_576,
  },
};

vi.stubGlobal('performance', perfStub);
vi.stubGlobal('PerformanceObserver', vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  disconnect: vi.fn(),
})));

// Import AFTER stubs are in place
import { performanceProfiler, measureAsync, measureSync } from '@/lib/performance';

// ─── Tests ────────────────────────────────────────────────────────────────────

describe('performanceProfiler', () => {
  beforeEach(() => {
    performanceProfiler.reset();
    Object.keys(marks).forEach((k) => delete marks[k]);
    measures.length = 0;
  });

  afterEach(() => vi.clearAllMocks());

  describe('startRender / endRender', () => {
    it('returns a RenderProfile with the component name', () => {
      const mark = performanceProfiler.startRender('TestComp');
      const profile = performanceProfiler.endRender('TestComp', mark);
      expect(profile.component).toBe('TestComp');
    });

    it('marks slow renders when renderMs exceeds threshold', () => {
      // Force a large duration by manipulating the measure stub
      perfStub.measure.mockReturnValueOnce([
        { name: 'render:SlowComp', duration: 50, startTime: 0, entryType: 'measure', toJSON: () => ({}) },
      ]);
      const mark = performanceProfiler.startRender('SlowComp');
      const profile = performanceProfiler.endRender('SlowComp', mark);
      expect(profile.slow).toBe(true);
    });

    it('does not mark fast renders as slow', () => {
      perfStub.measure.mockReturnValueOnce([
        { name: 'render:FastComp', duration: 2, startTime: 0, entryType: 'measure', toJSON: () => ({}) },
      ]);
      const mark = performanceProfiler.startRender('FastComp');
      const profile = performanceProfiler.endRender('FastComp', mark);
      expect(profile.slow).toBe(false);
    });
  });

  describe('getSummary', () => {
    it('returns empty renders on fresh instance', () => {
      const summary = performanceProfiler.getSummary();
      expect(summary.renders).toHaveLength(0);
      expect(summary.slowRenders).toHaveLength(0);
    });

    it('accumulates render profiles', () => {
      const m1 = performanceProfiler.startRender('A');
      performanceProfiler.endRender('A', m1);
      const m2 = performanceProfiler.startRender('B');
      performanceProfiler.endRender('B', m2);
      expect(performanceProfiler.getSummary().renders).toHaveLength(2);
    });

    it('computes avgRenderMs per component', () => {
      perfStub.measure
        .mockReturnValueOnce([{ name: 'r', duration: 10, startTime: 0, entryType: 'measure', toJSON: () => ({}) }])
        .mockReturnValueOnce([{ name: 'r', duration: 20, startTime: 0, entryType: 'measure', toJSON: () => ({}) }]);

      const m1 = performanceProfiler.startRender('Card');
      performanceProfiler.endRender('Card', m1);
      const m2 = performanceProfiler.startRender('Card');
      performanceProfiler.endRender('Card', m2);

      const { avgRenderMs } = performanceProfiler.getSummary();
      expect(avgRenderMs['Card']).toBe(15);
    });

    it('includes memory snapshot', () => {
      const { memory } = performanceProfiler.getSummary();
      expect(memory).not.toBeNull();
      expect(memory!.usedJSHeapMB).toBeCloseTo(10, 0);
    });
  });

  describe('snapshotMemory', () => {
    it('returns MB values', () => {
      const snap = performanceProfiler.snapshotMemory();
      expect(snap).not.toBeNull();
      expect(snap!.usedJSHeapMB).toBeGreaterThan(0);
      expect(snap!.limitJSHeapMB).toBeGreaterThan(snap!.usedJSHeapMB);
    });
  });

  describe('getNavigationTiming', () => {
    it('returns ttfb and other timing keys', () => {
      const timing = performanceProfiler.getNavigationTiming();
      expect(timing).not.toBeNull();
      expect(timing).toHaveProperty('ttfb');
      expect(timing).toHaveProperty('domContentLoaded');
      expect(timing).toHaveProperty('loadComplete');
    });

    it('ttfb equals responseStart - requestStart', () => {
      const timing = performanceProfiler.getNavigationTiming()!;
      expect(timing.ttfb).toBe(40); // 50 - 10
    });
  });

  describe('reset', () => {
    it('clears accumulated renders', () => {
      const m = performanceProfiler.startRender('X');
      performanceProfiler.endRender('X', m);
      performanceProfiler.reset();
      expect(performanceProfiler.getSummary().renders).toHaveLength(0);
    });
  });
});

// ─── measureAsync ─────────────────────────────────────────────────────────────

describe('measureAsync', () => {
  it('returns the result of the async function', async () => {
    const { result } = await measureAsync('fetch', async () => 42);
    expect(result).toBe(42);
  });

  it('returns a non-negative durationMs', async () => {
    const { durationMs } = await measureAsync('op', async () => 'ok');
    expect(durationMs).toBeGreaterThanOrEqual(0);
  });
});

// ─── measureSync ──────────────────────────────────────────────────────────────

describe('measureSync', () => {
  it('returns the result of the sync function', () => {
    const { result } = measureSync('calc', () => 1 + 1);
    expect(result).toBe(2);
  });

  it('returns a non-negative durationMs', () => {
    const { durationMs } = measureSync('noop', () => {});
    expect(durationMs).toBeGreaterThanOrEqual(0);
  });
});

// ─── usePerformanceProfile hook ───────────────────────────────────────────────

describe('usePerformanceProfile', () => {
  beforeEach(() => performanceProfiler.reset());

  it('markRenderStart and markRenderEnd return a profile', async () => {
    const { usePerformanceProfile } = await import('@/hooks/usePerformanceProfile');
    const { result } = renderHook(() => usePerformanceProfile('HookComp', { report: false }));

    let profile: ReturnType<typeof result.current.markRenderEnd> = null;
    act(() => {
      result.current.markRenderStart();
      profile = result.current.markRenderEnd();
    });

    expect(profile).not.toBeNull();
    expect(profile!.component).toBe('HookComp');
  });

  it('markRenderEnd returns null when markRenderStart was not called', async () => {
    const { usePerformanceProfile } = await import('@/hooks/usePerformanceProfile');
    const { result } = renderHook(() => usePerformanceProfile('NoStart', { report: false }));

    let profile: ReturnType<typeof result.current.markRenderEnd> = null;
    act(() => { profile = result.current.markRenderEnd(); });
    expect(profile).toBeNull();
  });

  it('snapshotMemory does not throw', async () => {
    const { usePerformanceProfile } = await import('@/hooks/usePerformanceProfile');
    const { result } = renderHook(() => usePerformanceProfile('MemComp', { report: false }));
    expect(() => act(() => result.current.snapshotMemory())).not.toThrow();
  });
});
