# Keyboard Shortcuts Migration Guide

This guide helps you integrate keyboard shortcuts into existing components and migrate from manual keyboard event handling.

## Quick Start

### 1. Basic Integration

Replace manual keyboard event listeners with the `useShortcut` hook:

**Before:**
```tsx
function MyComponent() {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'k' && e.ctrlKey) {
        e.preventDefault();
        openSearch();
      }
    };
    
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);
  
  return <div>...</div>;
}
```

**After:**
```tsx
import { useShortcut } from '@/hooks/useShortcut';

function MyComponent() {
  useShortcut({
    id: 'open-search',
    name: 'Open Search',
    description: 'Open the search dialog',
    category: 'search',
    defaultBinding: { 
      key: 'k', 
      modifiers: ['ctrl'],
      mac: { key: 'k', modifiers: ['meta'] }
    },
    handler: openSearch,
  });
  
  return <div>...</div>;
}
```

### 2. Modal/Dialog Escape Key

**Before:**
```tsx
function Modal({ onClose }: { onClose: () => void }) {
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    
    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [onClose]);
  
  return <div>...</div>;
}
```

**After:**
```tsx
import { useShortcut } from '@/hooks/useShortcut';

function Modal({ onClose }: { onClose: () => void }) {
  useShortcut({
    id: 'close-modal',
    name: 'Close Modal',
    description: 'Close the current modal',
    category: 'ui',
    defaultBinding: { key: 'Escape' },
    handler: onClose,
    enabled: true, // Only active when modal is mounted
  });
  
  return <div>...</div>;
}
```

### 3. Navigation Shortcuts

**Before:**
```tsx
function Navigation() {
  const router = useRouter();
  
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.altKey && e.key === 'd') {
        e.preventDefault();
        router.push('/dashboard');
      }
    };
    
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [router]);
  
  return <nav>...</nav>;
}
```

**After:**
```tsx
import { useShortcut } from '@/hooks/useShortcut';

function Navigation() {
  const router = useRouter();
  
  useShortcut({
    id: 'go-to-dashboard',
    name: 'Go to Dashboard',
    description: 'Navigate to the dashboard page',
    category: 'navigation',
    defaultBinding: { 
      key: 'd', 
      modifiers: ['alt'],
      mac: { key: 'd', modifiers: ['ctrl'] }
    },
    handler: () => router.push('/dashboard'),
  });
  
  return <nav>...</nav>;
}
```

## Common Patterns

### Multiple Shortcuts in One Component

```tsx
import { useShortcuts } from '@/hooks/useShortcut';

function Editor() {
  const [content, setContent] = useState('');
  
  useShortcuts([
    {
      id: 'save-document',
      name: 'Save Document',
      description: 'Save the current document',
      category: 'actions',
      defaultBinding: { 
        key: 's', 
        modifiers: ['ctrl'],
        mac: { key: 's', modifiers: ['meta'] }
      },
      handler: handleSave,
    },
    {
      id: 'undo',
      name: 'Undo',
      description: 'Undo last action',
      category: 'actions',
      defaultBinding: { 
        key: 'z', 
        modifiers: ['ctrl'],
        mac: { key: 'z', modifiers: ['meta'] }
      },
      handler: handleUndo,
    },
    {
      id: 'redo',
      name: 'Redo',
      description: 'Redo last undone action',
      category: 'actions',
      defaultBinding: { 
        key: 'y', 
        modifiers: ['ctrl'],
        mac: { key: 'z', modifiers: ['meta', 'shift'] }
      },
      handler: handleRedo,
    },
  ]);
  
  return <div>...</div>;
}
```

### Conditional Shortcuts

```tsx
function DataTable({ data }: { data: any[] }) {
  const [selectedRow, setSelectedRow] = useState<number | null>(null);
  
  // Only register delete shortcut when a row is selected
  useShortcut({
    id: 'delete-row',
    name: 'Delete Row',
    description: 'Delete the selected row',
    category: 'actions',
    defaultBinding: { key: 'Delete' },
    handler: handleDelete,
    enabled: selectedRow !== null, // Conditional enabling
  });
  
  return <table>...</table>;
}
```

### Dynamic Shortcuts

```tsx
function DynamicShortcuts() {
  const { registerShortcut, unregisterShortcut } = useKeyboardShortcuts();
  const [features, setFeatures] = useState<string[]>([]);
  
  useEffect(() => {
    // Register shortcuts based on available features
    features.forEach((feature, index) => {
      registerShortcut({
        id: `feature-${feature}`,
        name: `Open ${feature}`,
        description: `Open the ${feature} panel`,
        category: 'actions',
        defaultBinding: { key: String(index + 1), modifiers: ['ctrl'] },
        handler: () => openFeature(feature),
      });
    });
    
    return () => {
      features.forEach(feature => {
        unregisterShortcut(`feature-${feature}`);
      });
    };
  }, [features, registerShortcut, unregisterShortcut]);
  
  return <div>...</div>;
}
```

## Best Practices

### 1. Use Descriptive IDs

```tsx
// ❌ Bad
id: 'shortcut1'

// ✅ Good
id: 'open-search-dialog'
```

### 2. Provide Clear Descriptions

```tsx
// ❌ Bad
description: 'Opens search'

// ✅ Good
description: 'Open the global search dialog to find corridors, anchors, and transactions'
```

### 3. Choose Appropriate Categories

```tsx
// Navigation shortcuts
category: 'navigation'

// Search functionality
category: 'search'

// User actions (save, delete, etc.)
category: 'actions'

// UI controls (toggle sidebar, theme, etc.)
category: 'ui'

// Accessibility features
category: 'accessibility'

// System-level (help, settings, etc.)
category: 'system'
```

### 4. Platform-Specific Bindings

Always provide Mac alternatives for Ctrl shortcuts:

```tsx
defaultBinding: {
  key: 'k',
  modifiers: ['ctrl'], // Windows/Linux
  mac: { key: 'k', modifiers: ['meta'] } // macOS uses Cmd
}
```

### 5. Avoid Browser Conflicts

Don't use these common browser shortcuts:
- `Ctrl+R` / `Cmd+R` - Reload
- `Ctrl+T` / `Cmd+T` - New tab
- `Ctrl+W` / `Cmd+W` - Close tab
- `Ctrl+N` / `Cmd+N` - New window
- `Ctrl+F` / `Cmd+F` - Find in page

Instead, add additional modifiers:
```tsx
// ❌ Bad - conflicts with browser
{ key: 'r', modifiers: ['ctrl'] }

// ✅ Good - no conflict
{ key: 'r', modifiers: ['ctrl', 'shift'] }
```

### 6. Handle Async Operations

```tsx
useShortcut({
  id: 'save-data',
  name: 'Save Data',
  description: 'Save data to server',
  category: 'actions',
  defaultBinding: { key: 's', modifiers: ['ctrl'] },
  handler: async () => {
    try {
      await saveToServer();
      showNotification('Saved successfully');
    } catch (error) {
      showNotification('Save failed', 'error');
    }
  },
});
```

## Testing

### Unit Tests

```tsx
import { renderHook } from '@testing-library/react';
import { useShortcut } from '@/hooks/useShortcut';
import { KeyboardShortcutsProvider } from '@/contexts/KeyboardShortcutsContext';

describe('MyComponent shortcuts', () => {
  it('should trigger handler on key press', () => {
    const handler = vi.fn();
    
    renderHook(
      () => useShortcut({
        id: 'test',
        name: 'Test',
        description: 'Test shortcut',
        category: 'actions',
        defaultBinding: { key: 'k', modifiers: ['ctrl'] },
        handler,
      }),
      { wrapper: KeyboardShortcutsProvider }
    );
    
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true });
    
    expect(handler).toHaveBeenCalled();
  });
});
```

### Integration Tests

```tsx
import { render, fireEvent } from '@testing-library/react';
import { KeyboardShortcutsProvider } from '@/contexts/KeyboardShortcutsContext';

describe('Search integration', () => {
  it('should open search on Ctrl+K', () => {
    const { getByTestId } = render(
      <KeyboardShortcutsProvider>
        <App />
      </KeyboardShortcutsProvider>
    );
    
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true });
    
    expect(getByTestId('search-dialog')).toBeVisible();
  });
});
```

## Troubleshooting

### Shortcut Not Working

1. **Check if input is focused**: Shortcuts don't work in input fields by default
2. **Verify shortcut is enabled**: Check settings page
3. **Look for conflicts**: Use the help overlay (Shift+?) to see all shortcuts
4. **Check browser console**: Look for error messages

### Conflicts with Existing Code

If you have existing keyboard event listeners:

1. Remove the old event listeners
2. Register shortcuts using the new system
3. Test thoroughly to ensure no functionality is lost

### Performance Issues

If you have many shortcuts:

1. Use `useShortcuts` instead of multiple `useShortcut` calls
2. Conditionally enable/disable shortcuts based on context
3. Unregister shortcuts when components unmount

## Migration Checklist

- [ ] Identify all keyboard event listeners in your codebase
- [ ] Replace with `useShortcut` or `useShortcuts` hooks
- [ ] Add platform-specific bindings for Mac
- [ ] Choose appropriate categories
- [ ] Write descriptive names and descriptions
- [ ] Test on all platforms (Mac, Windows, Linux)
- [ ] Update documentation
- [ ] Add tests for new shortcuts
- [ ] Remove old event listener code
- [ ] Verify no conflicts with browser shortcuts

## Support

For questions or issues:
1. Check the main [KEYBOARD_SHORTCUTS.md](./KEYBOARD_SHORTCUTS.md) documentation
2. Review the test files for examples
3. Open an issue in the project repository
