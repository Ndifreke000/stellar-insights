# Keyboard Shortcuts - Quick Reference

## 🚀 Quick Start

### 1. Register a Shortcut

```tsx
import { useShortcut } from '@/hooks/useShortcut';

useShortcut({
  id: 'my-action',
  name: 'My Action',
  description: 'Does something cool',
  category: 'actions',
  defaultBinding: { 
    key: 'k', 
    modifiers: ['ctrl'],
    mac: { key: 'k', modifiers: ['meta'] }
  },
  handler: () => console.log('Triggered!'),
});
```

### 2. Show Help Overlay

```tsx
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';

const { showHelp } = useKeyboardShortcuts();
showHelp(); // or press Shift+?
```

### 3. Customize in Settings

Navigate to: **Settings → Keyboard Shortcuts**

## 📋 Default Shortcuts

| Action | Windows/Linux | macOS | Category |
|--------|--------------|-------|----------|
| Show Help | `Shift+?` | `Shift+?` | System |
| Dashboard | `Alt+D` | `Ctrl+D` | Navigation |
| Corridors | `Alt+C` | `Ctrl+C` | Navigation |
| Anchors | `Alt+A` | `Ctrl+A` | Navigation |
| Analytics | `Alt+Y` | `Ctrl+Y` | Navigation |
| Search | `Ctrl+K` | `Cmd+K` | Search |
| Toggle Sidebar | `Ctrl+B` | `Cmd+B` | UI |
| Toggle Theme | `Ctrl+Shift+D` | `Cmd+Shift+D` | UI |
| Notifications | `Ctrl+Shift+N` | `Cmd+Shift+N` | UI |
| Refresh | `Ctrl+Shift+R` | `Cmd+Shift+R` | Actions |
| Skip to Content | `Alt+M` | `Ctrl+M` | Accessibility |

## 🎯 Common Patterns

### Single Shortcut
```tsx
useShortcut({
  id: 'save',
  name: 'Save',
  description: 'Save document',
  category: 'actions',
  defaultBinding: { key: 's', modifiers: ['ctrl'] },
  handler: handleSave,
});
```

### Multiple Shortcuts
```tsx
useShortcuts([
  { id: 'undo', name: 'Undo', /* ... */ },
  { id: 'redo', name: 'Redo', /* ... */ },
]);
```

### Conditional Shortcut
```tsx
useShortcut({
  id: 'delete',
  name: 'Delete',
  /* ... */
  enabled: selectedItem !== null,
});
```

### Async Handler
```tsx
useShortcut({
  id: 'save',
  name: 'Save',
  /* ... */
  handler: async () => {
    await saveToServer();
  },
});
```

## 🔧 API Cheat Sheet

### useShortcut Hook
```tsx
useShortcut({
  id: string,              // Unique identifier
  name: string,            // Display name
  description: string,     // Help text
  category: string,        // Category for grouping
  defaultBinding: {
    key: string,           // Key name
    modifiers?: string[],  // ['ctrl', 'alt', 'shift', 'meta']
    mac?: { /* ... */ },   // Mac override
  },
  handler: (e) => void,    // Handler function
  enabled?: boolean,       // Enable/disable
  preventDefault?: boolean,// Prevent default (default: true)
});
```

### useKeyboardShortcuts Context
```tsx
const {
  platform,              // 'mac' | 'windows' | 'linux'
  config,                // Current configuration
  registerShortcut,      // Register new shortcut
  unregisterShortcut,    // Remove shortcut
  getShortcuts,          // Get all shortcuts
  customizeBinding,      // Change key binding
  toggleShortcut,        // Enable/disable
  showHelp,              // Show help overlay
  hideHelp,              // Hide help overlay
  isHelpVisible,         // Help overlay state
} = useKeyboardShortcuts();
```

## 📦 Categories

- `navigation` - Page navigation
- `search` - Search functionality
- `actions` - User actions (save, delete, etc.)
- `ui` - UI controls (sidebar, theme, etc.)
- `accessibility` - Accessibility features
- `system` - System shortcuts (help, settings, etc.)

## 🎨 Key Names

### Letters & Numbers
`a-z`, `0-9`

### Special Keys
`Enter`, `Escape`, `Space`, `Tab`, `Backspace`, `Delete`

### Arrow Keys
`ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`

### Function Keys
`F1-F12`

### Modifiers
`ctrl`, `alt`, `shift`, `meta` (Cmd on Mac, Win on Windows)

## ⚠️ Avoid These (Browser Conflicts)

- `Ctrl+R` / `Cmd+R` - Reload
- `Ctrl+T` / `Cmd+T` - New tab
- `Ctrl+W` / `Cmd+W` - Close tab
- `Ctrl+N` / `Cmd+N` - New window
- `Ctrl+F` / `Cmd+F` - Find

## 🧪 Testing

```tsx
import { renderHook } from '@testing-library/react';
import { useShortcut } from '@/hooks/useShortcut';
import { KeyboardShortcutsProvider } from '@/contexts/KeyboardShortcutsContext';

const handler = vi.fn();
renderHook(
  () => useShortcut({ /* ... */ handler }),
  { wrapper: KeyboardShortcutsProvider }
);

fireEvent.keyDown(document, { key: 'k', ctrlKey: true });
expect(handler).toHaveBeenCalled();
```

## 🐛 Troubleshooting

### Shortcut not working?
1. Check if input is focused (shortcuts disabled in inputs)
2. Verify shortcut is enabled in settings
3. Check for conflicts (press `Shift+?`)
4. Look for errors in console

### Conflict detected?
1. Choose different key combination
2. Add more modifiers
3. Use platform-specific override

### Not persisting?
1. Check localStorage is enabled
2. Clear cache and try again
3. Check browser console for errors

## 📚 Documentation

- **Full Docs**: `frontend/KEYBOARD_SHORTCUTS.md`
- **Migration Guide**: `frontend/KEYBOARD_SHORTCUTS_MIGRATION.md`
- **Implementation**: `KEYBOARD_SHORTCUTS_IMPLEMENTATION.md`

## 💡 Tips

1. **Use Cmd on Mac**: Always provide Mac alternative with `meta` modifier
2. **Descriptive IDs**: Use kebab-case like `open-search-dialog`
3. **Clear Descriptions**: Help users understand what shortcuts do
4. **Test on All Platforms**: Verify on Mac, Windows, and Linux
5. **Avoid Browser Conflicts**: Add extra modifiers if needed
6. **Handle Errors**: Wrap handlers in try-catch
7. **Document Custom Shortcuts**: Add comments explaining choices

## 🎓 Examples

See `frontend/src/components/keyboard-shortcuts/ShortcutExample.tsx` for live examples.

## 🔗 Related Files

```
frontend/src/
├── types/keyboard-shortcuts.ts
├── lib/keyboard-shortcuts/
│   ├── utils.ts
│   ├── registry.ts
│   ├── default-shortcuts.ts
│   └── index.ts
├── contexts/KeyboardShortcutsContext.tsx
├── hooks/useShortcut.ts
└── components/keyboard-shortcuts/
    ├── ShortcutHelpOverlay.tsx
    ├── ShortcutCustomizer.tsx
    └── ShortcutsInitializer.tsx
```

---

**Need help?** Press `Shift+?` to see all available shortcuts!
