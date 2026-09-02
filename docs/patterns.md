# State Management Patterns & Conventions

This document outlines the state management patterns and conventions for the PayRaider frontend application.

## Architecture Overview

We use a hybrid state management approach:

- **React Query** - Server state (API calls, caching, synchronization)
- **Zustand** - Client state (UI state, form state, user preferences)
- **React Context** - Theme and existing context providers (gradually migrating)

## State Types

### Server State (React Query)
- API responses
- Remote data that can become stale
- Data that should be cached
- Data that needs background updates

### Client State (Zustand)
- UI state (sidebar, modals, loading states)
- Form data and validation errors
- User session and preferences
- WebSocket connection state
- Navigation state

## Key Patterns

### 1. Query Keys Pattern

```typescript
// Use consistent query keys
export const queryKeys = {
  anchors: ['anchors'] as const,
  anchor: (id: string) => ['anchors', id] as const,
  anchorAssets: (id: string) => ['anchors', id, 'assets'] as const,
} as const;
```

### 2. Custom Hooks Pattern

```typescript
// Create domain-specific hooks
export function useAnchors() {
  return useApiQuery(
    queryKeys.anchors,
    () => fetchAnchors(),
    { staleTime: 5 * 60 * 1000 }
  );
}
```

### 3. Store Slices Pattern

```typescript
// Organize Zustand store by domain
export const useAppStore = create<AppState & AppActions>()(
  devtools(
    persist(
      immer((set) => ({
        // UI slice
        sidebarCollapsed: false,
        setSidebarCollapsed: (collapsed) => set((state) => {
          state.sidebarCollapsed = collapsed;
        }),
        
        // Form slice
        formData: {},
        setFormData: (key, data) => set((state) => {
          state.formData[key] = data;
        }),
      }))
    )
  )
);
```

### 4. Selector Hooks Pattern

```typescript
// Create focused selectors for components
export const useUIState = () => useAppStore((state) => ({
  sidebarCollapsed: state.sidebarCollapsed,
  activeModal: state.activeModal,
}));

export const useFormState = (formKey?: string) => useAppStore((state) => ({
  data: formKey ? state.formData[formKey] : state.formData,
  errors: formKey ? state.formErrors[formKey] : state.formErrors,
}));
```

## Conventions

### File Organization

```
src/lib/
├── react-query/
│   ├── provider.tsx      # React Query setup
│   ├── hooks.ts          # Custom hooks
│   └── logger.tsx        # Query logging
├── zustand/
│   ├── store.ts          # Main store
│   └── middleware.ts     # Store middleware
└── state-management/
    └── patterns.md       # This documentation
```

### Naming Conventions

- **Query Keys**: `resource` or `resource:id` pattern
- **Store Actions**: `setXxx` for setters, `updateXxx` for partial updates
- **Custom Hooks**: `useXxx` pattern
- **Selectors**: `useXxxState` for focused selectors

### Error Handling

```typescript
// React Query errors
export function useApiQuery<T>(queryKey, queryFn) {
  return useQuery({
    queryKey,
    queryFn: async () => {
      try {
        return await queryFn();
      } catch (error) {
        logger.error('Query failed', { queryKey, error });
        throw error;
      }
    },
  });
}

// Zustand error handling
export const useAppStore = create((set) => ({
  setError: (error) => {
    logger.error('Store error', error);
    set((state) => { state.error = error; });
  },
}));
```

### Loading States

```typescript
// React Query loading
const { data, isLoading, error } = useAnchors();

// Zustand loading
const { setLoading, loading } = useAppStore();
setLoading('fetch-anchors', true);
```

### Cache Strategies

```typescript
// Short-lived data (real-time)
useQuery({
  queryKey: ['websocket-status'],
  queryFn: fetchWebSocketStatus,
  staleTime: 0,
  refetchInterval: 1000,
});

// Medium-lived data (user preferences)
useQuery({
  queryKey: ['user-preferences'],
  queryFn: fetchUserPreferences,
  staleTime: 10 * 60 * 1000, // 10 minutes
});

// Long-lived data (static data)
useQuery({
  queryKey: ['app-config'],
  queryFn: fetchAppConfig,
  staleTime: 24 * 60 * 60 * 1000, // 24 hours
});
```

## Migration Strategy

### Phase 1: Setup Infrastructure
- ✅ Install React Query and Zustand
- ✅ Create provider setup
- ✅ Define query keys and store structure

### Phase 2: Migrate Server State
- Replace `useState` API calls with React Query
- Add query hooks for existing API calls
- Update components to use new hooks

### Phase 3: Migrate Client State
- Move form state to Zustand
- Move UI state to Zustand
- Update components to use store hooks

### Phase 4: Optimize & Clean Up
- Remove old context providers
- Add devtools integration
- Add comprehensive error handling

## Best Practices

### 1. Keep Queries Co-located

```typescript
// services/anchors.ts
export function useAnchors() {
  return useApiQuery(queryKeys.anchors, fetchAnchors);
}

export function useAnchor(id: string) {
  return useApiQuery(queryKeys.anchor(id), () => fetchAnchor(id));
}
```

### 2. Use Selectors for Performance

```typescript
// ❌ Bad - causes unnecessary re-renders
const store = useAppStore();
return <div>{store.sidebarCollapsed}</div>;

// ✅ Good - only re-renders when sidebarCollapsed changes
const { sidebarCollapsed } = useUIState();
return <div>{sidebarCollapsed}</div>;
```

### 3. Handle Loading and Error States

```typescript
export function useAnchors() {
  const query = useApiQuery(queryKeys.anchors, fetchAnchors);
  
  return {
    ...query,
    // Add derived state
    hasData: query.data && query.data.length > 0,
    isEmpty: !query.isLoading && (!query.data || query.data.length === 0),
  };
}
```

### 4. Use Mutation Helpers

```typescript
export function useCreateAnchor() {
  return useApiMutation(
    createAnchor,
    {
      onSuccess: () => {
        // Invalidate related queries
        queryClient.invalidateQueries({ queryKey: queryKeys.anchors });
        // Show success notification
        useAppStore.getState().addNotification({
          type: 'success',
          message: 'Anchor created successfully',
        });
      },
    }
  );
}
```

### 5. Persist Important State

```typescript
// Only persist essential user preferences
persist(
  (state) => ({
    sidebarCollapsed: state.sidebarCollapsed,
    formData: state.formData,
    filters: state.filters,
  }),
  {
    name: 'app-store',
    version: 1,
  }
);
```

## Testing Patterns

### Testing React Query Hooks

```typescript
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useAnchors } from '@/hooks/useAnchors';

const createTestQueryClient = () => new QueryClient({
  defaultOptions: {
    queries: { retry: false },
    mutations: { retry: false },
  },
});

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={createTestQueryClient()}>
    {children}
  </QueryClientProvider>
);

test('useAnchors returns data', async () => {
  const { result } = renderHook(() => useAnchors(), { wrapper });
  
  await waitFor(() => expect(result.current.isSuccess).toBe(true));
  expect(result.current.data).toBeDefined();
});
```

### Testing Zustand Store

```typescript
import { act, renderHook } from '@testing-library/react';
import { useAppStore } from '@/lib/zustand/store';

test('setSidebarCollapsed updates state', () => {
  const { result } = renderHook(() => useAppStore());
  
  act(() => {
    result.current.setSidebarCollapsed(true);
  });
  
  expect(result.current.sidebarCollapsed).toBe(true);
});
```

## DevTools Integration

### React Query DevTools
- Automatically enabled in development
- Inspect query cache, mutations, and subscriptions
- Debug query keys and data flow

### Zustand DevTools
- Automatically enabled in development
- Inspect state changes and actions
- Time travel debugging

### Browser DevTools
```typescript
// Expose debugging functions in development
if (process.env.NODE_ENV === 'development') {
  (window as any).__queryDevtools = {
    logCacheStats: () => console.log(queryClient.getQueryCache().getAll()),
    invalidateAll: () => queryClient.invalidateQueries(),
  };
  
  (window as any).__storeDevtools = {
    getState: () => console.log(useAppStore.getState()),
    reset: () => useAppStore.getState().resetState(),
  };
}
```

## Performance Considerations

### 1. Query Deduplication
```typescript
// Multiple components using same data
const Component1 = () => {
  const { data: anchors } = useAnchors(); // ✅ Single request
};

const Component2 = () => {
  const { data: anchors } = useAnchors(); // ✅ Same data, no new request
};
```

### 2. Selective Updates
```typescript
// Only update specific parts of state
const updateFormData = (key, updates) => {
  useAppStore.setState((state) => {
    state.formData[key] = { ...state.formData[key], ...updates };
  });
};
```

### 3. Lazy Loading
```typescript
// Load data only when needed
const useDetailedAnalytics = (enabled: boolean) => {
  return useQuery({
    queryKey: ['analytics', 'detailed'],
    queryFn: fetchDetailedAnalytics,
    enabled,
  });
};
```

## Troubleshooting

### Common Issues

1. **Stale Data**: Check `staleTime` and `refetchInterval` settings
2. **Memory Leaks**: Ensure proper cleanup in useEffect
3. **Infinite Loops**: Avoid calling state setters in render
4. **Race Conditions**: Use proper loading states and error boundaries

### Debugging Tools

```typescript
// Enable verbose logging
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      onSuccess: (data) => console.log('Query success:', data),
      onError: (error) => console.error('Query error:', error),
    },
  },
});

// Store subscription for debugging
useAppStore.subscribe(
  (state) => state.sidebarCollapsed,
  (collapsed) => console.log('Sidebar collapsed:', collapsed)
);
```

This pattern documentation should be kept up to date as the state management evolves.
