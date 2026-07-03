import { StateCreator } from 'zustand';

/**
 * Forwards to an overloaded zustand `setState` with the exact args tuple it
 * was called with. TypeScript can't resolve an overloaded function type
 * through a generic `(...args) => original(...args)` passthrough — the two
 * `setState` overloads (`(partial, replace?: false)` vs `(state, replace:
 * true)`) don't recombine into the union `args` was inferred as. The
 * forwarding itself is safe: `args` is always the untouched tuple this
 * function was called with.
 */
function forwardSet(fn: (...args: never[]) => void, args: unknown[]): void {
  (fn as (...a: unknown[]) => void)(...args);
}

/**
 * Logging middleware for Zustand store
 */
export const loggerMiddleware = <T extends object>(
  config: { name: string; enabled?: boolean }
) => (f: StateCreator<T>): StateCreator<T> => {
  return (set, get, api) => {
    const loggedSet: typeof set = (...args) => {
      if (config.enabled !== false && process.env.NODE_ENV === 'development') {
        console.log(`[${config.name}] State update:`, args);
      }
      return forwardSet(set, args);
    };

    return f(loggedSet, get, api);
  };
};

/**
 * Analytics middleware for tracking state changes
 */
export const analyticsMiddleware = <T extends object>(
  config: { trackChanges?: boolean; events?: string[] }
) => (f: StateCreator<T>): StateCreator<T> => {
  return (set, get, api) => {
    const originalSet = set;

    const trackedSet: typeof set = (...args) => {
      // Track specific state changes
      if (config.trackChanges && process.env.NODE_ENV === 'development') {
        const [partialState] = args;
        if (typeof partialState === 'object' && partialState !== null) {
          const state = partialState as Record<string, unknown>;
          Object.keys(state).forEach(key => {
            if (!config.events || config.events.includes(key)) {
              console.log(`[Analytics] State change: ${key}`, state[key]);
            }
          });
        }
      }

      return forwardSet(originalSet, args);
    };

    return f(trackedSet, get, api);
  };
};

/**
 * Performance monitoring middleware
 */
export const performanceMiddleware = <T extends object>(
  config: { enabled?: boolean }
) => (f: StateCreator<T>): StateCreator<T> => {
  return (set, get, api) => {
    const originalSet = set;

    const performanceSet: typeof set = (...args) => {
      if (config.enabled !== false) {
        const start = performance.now();
        forwardSet(originalSet, args);
        const end = performance.now();

        if (end - start > 1) { // Only log if update takes more than 1ms
          console.warn(`[Performance] Slow state update: ${(end - start).toFixed(2)}ms`);
        }
      }

      return forwardSet(originalSet, args);
    };

    return f(performanceSet, get, api);
  };
};

/**
 * Validation middleware for state updates
 */
export const validationMiddleware = <T extends object>(
  config: { validator?: (state: Partial<T>) => boolean | string }
) => (f: StateCreator<T>): StateCreator<T> => {
  return (set, get, api) => {
    const validatedSet: typeof set = (...args) => {
      const [partialState, ...rest] = args;

      if (config.validator && typeof partialState === 'object' && partialState !== null) {
        const result = config.validator(partialState as Partial<T>);
        if (result === false) {
          console.error('[Validation] State update rejected');
          return;
        }
        if (typeof result === 'string') {
          console.error(`[Validation] State update rejected: ${result}`);
          return;
        }
      }

      return forwardSet(set, [partialState, ...rest]);
    };

    return f(validatedSet, get, api);
  };
};

/**
 * Undo/Redo middleware for state changes
 */
type UndoRedoState<T> = T & {
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
};

export const undoRedoMiddleware = <T extends object>(
  config: { maxSize?: number }
) => (f: StateCreator<T>): StateCreator<UndoRedoState<T>> => {
  return (set, get, api) => {
    let history: T[] = [];
    let currentIndex = -1;
    const maxSize = config.maxSize || 50;

    const undo = () => {
      if (currentIndex > 0) {
        currentIndex--;
        const prevState = history[currentIndex];
        set(prevState as Partial<UndoRedoState<T>>);
        return true;
      }
      return false;
    };

    const redo = () => {
      if (currentIndex < history.length - 1) {
        currentIndex++;
        const nextState = history[currentIndex];
        set(nextState as Partial<UndoRedoState<T>>);
        return true;
      }
      return false;
    };

    const canUndo = () => currentIndex > 0;
    const canRedo = () => currentIndex < history.length - 1;

    const originalSet = set;

    const trackedSet: typeof set = (...args) => {
      const [partialState, ...rest] = args;

      // Add to history
      const currentState = get();
      const newState = { ...currentState, ...partialState } as T;

      // Remove any states after current index
      history = history.slice(0, currentIndex + 1);

      // Add new state
      history.push(newState);

      // Limit history size
      if (history.length > maxSize) {
        history = history.slice(-maxSize);
        currentIndex = history.length - 1;
      } else {
        currentIndex = history.length - 1;
      }

      return forwardSet(originalSet, [partialState, ...rest]);
    };

    return { ...f(trackedSet, get, api), undo, redo, canUndo, canRedo };
  };
};

/**
 * Sync with localStorage middleware
 */
export const localStorageMiddleware = <T extends object>(
  config: { key: string; whitelist?: (keyof T)[]; blacklist?: (keyof T)[] }
) => (f: StateCreator<T>): StateCreator<T> => {
  return (set, get, api) => {
    // Load from localStorage on init
    if (typeof window !== 'undefined') {
      try {
        const stored = localStorage.getItem(config.key);
        if (stored) {
          const parsed = JSON.parse(stored);
          set(parsed as Partial<T>);
        }
      } catch (error) {
        console.error(`[LocalStorage] Failed to load state for ${config.key}:`, error);
      }
    }

    const originalSet = set;

    const syncedSet: typeof set = (...args) => {
      const [partialState, ...rest] = args;

      const result = forwardSet(originalSet, [partialState, ...rest]);

      // Save to localStorage
      if (typeof window !== 'undefined') {
        try {
          const currentState = get();
          let stateToSave: Partial<T> = currentState;

          // Apply whitelist if provided
          if (config.whitelist) {
            stateToSave = {} as Partial<T>;
            config.whitelist.forEach(key => {
              if (key in currentState) {
                (stateToSave as any)[key] = currentState[key];
              }
            });
          }
          
          // Apply blacklist if provided
          if (config.blacklist) {
            const temp = { ...stateToSave };
            config.blacklist.forEach(key => {
              delete (temp as any)[key];
            });
            stateToSave = temp;
          }
          
          localStorage.setItem(config.key, JSON.stringify(stateToSave));
        } catch (error) {
          console.error(`[LocalStorage] Failed to save state for ${config.key}:`, error);
        }
      }
      
      return result;
    };

    return f(syncedSet, get, api);
  };
};
