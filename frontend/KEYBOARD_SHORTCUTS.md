# Keyboard Shortcuts System

A comprehensive, accessible, and cross-platform keyboard shortcuts system for the Stellar Insights application.

## Features

- ✅ **Cross-platform support** - Automatic detection and handling for macOS, Windows, and Linux
- ✅ **Accessible** - Full keyboard navigation, ARIA attributes, and screen reader support
- ✅ **Customizable** - Users can customize any shortcut with conflict detection
- ✅ **Persistent** - Settings saved to localStorage and synced across sessions
- ✅ **Non-intrusive** - Respects input fields and doesn't interfere with native browser shortcuts
- ✅ **Help overlay** - Built-in keyboard shortcuts reference (press `Shift+?`)
- ✅ **Conflict detection** - Prevents duplicate bindings and browser shortcut conflicts
- ✅ **Type-safe** - Full TypeScript support with comprehensive types

## Architecture

### Core Components

```
frontend/src/
├── types/
│   └── keyboard-shortcuts.ts          # TypeScript type definitions
├── lib/keyboard-shortcuts/
│   ├── utils.ts                       # Platform detection, key matching, formatting
│   ├── registry.ts                    # Central shortcut registry
│   ├── default-shortcuts.ts           # Default application shortcuts
│   └── index.ts                       # Public API exports
├── contexts/
│   └── KeyboardShortcutsContext.tsx   # React context and provider
├── hooks/
│   └── useShortcut.ts                 # Convenient hooks for components
└── components/keyboard-shortcuts/
    ├── ShortcutHelpOverlay.tsx        # Help dialog (Shift+?)
    ├── ShortcutCustomizer.tsx         # Settings UI for customization
    └── ShortcutsInitializer.tsx       # Registers default shortcuts
```

## Usage

### Basic Setup

The keyboard shortcuts system is already integrated into the app layout. No additional setup is required.

### Using Shortcuts in Components

```tsx
import { useShortcut } from '@/hooks/useShortcut';

function MyComponent() {
  const [isOpen, setIsOpen] = useState(false);

  useShortcut({
    id: 'open-my-dialog',
    name: 'Open My Dialog',
    description: 'Opens the custom dialog',
    category: 'actions',
    defaultBinding: { 
      key: 'm', 
      modifiers: ['ctrl'],
      mac: { key: 'm', modifiers: ['meta'] } // Cmd+M on Mac
    },
    handler: () => setIsOpen(true),
  });

  return <div>...</div>;
}
```

### Registering Multiple Shortcuts

```tsx
import { useShortcuts } from '@/hooks/useShortcut';

function MyComponent() {
  useShortcuts([
    {
      id: 'action-1',
      name: 'Action 1',
      description: 'First action',
      category: 'actions',
      defaultBinding: { key: '1', modifiers: ['ctrl'] },
      handler: () => console.log('Action 1'),
    },
    {
      id: 'action-2',
      name: 'Action 2',
      description: 'Second action',
      category: 'actions',
      defaultBinding: { key: '2', modifiers: ['ctrl'] },
      handler: () => console.log('Action 2'),
    },
  ]);

  return <div>...</div>;
}
```

### Using the Context Directly

```tsx
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';

function MyComponent() {
  const { 
    platform,
    getShortcuts,
    customizeBinding,
    showHelp,
  } = useKeyboardShortcuts();

  return (
    <div>
      <p>Platform: {platform}</p>
      <button onClick={showHelp}>Show Shortcuts</button>
    </div>
  );
}
```

## Default Shortcuts

### System
- `Shift+?` - Show keyboard shortcuts help

### Navigation
- `Alt+D` (Ctrl+D on Mac) - Go to Dashboard
- `Alt+C` (Ctrl+C on Mac) - Go to Corridors
- `Alt+A` (Ctrl+A on Mac) - Go to Anchors
- `Alt+Y` (Ctrl+Y on Mac) - Go to Analytics

### Search
- `Ctrl+K` (Cmd+K on Mac) - Open search

### UI Actions
- `Ctrl+B` (Cmd+B on Mac) - Toggle sidebar
- `Ctrl+Shift+D` (Cmd+Shift+D on Mac) - Toggle theme
- `Ctrl+Shift+N` (Cmd+Shift+N on Mac) - Open notifications

### Actions
- `Ctrl+Shift+R` (Cmd+Shift+R on Mac) - Refresh data

### Accessibility
- `Alt+M` (Ctrl+M on Mac) - Skip to main content

## Customization

Users can customize shortcuts in two ways:

### 1. Settings Page

Navigate to Settings → Keyboard Shortcuts section to:
- View all available shortcuts
- Click on a shortcut to record a new key combination
- Enable/disable individual shortcuts
- Reset shortcuts to defaults

### 2. Programmatically

```tsx
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';

function MyComponent() {
  const { customizeBinding, toggleShortcut } = useKeyboardShortcuts();

  const handleCustomize = () => {
    customizeBinding('open-search', { 
      key: 'f', 
      modifiers: ['ctrl', 'shift'] 
    });
  };

  const handleDisable = () => {
    toggleShortcut('open-search', false);
  };

  return <div>...</div>;
}
```

## Platform-Specific Bindings

Define different bindings for different platforms:

```tsx
{
  id: 'my-action',
  name: 'My Action',
  description: 'Does something',
  category: 'actions',
  defaultBinding: {
    key: 'k',
    modifiers: ['ctrl'], // Default for Windows/Linux
    mac: { key: 'k', modifiers: ['meta'] }, // Cmd+K on Mac
    windows: { key: 'k', modifiers: ['ctrl', 'alt'] }, // Ctrl+Alt+K on Windows
  },
  handler: () => {},
}
```

## Categories

Shortcuts are organized into categories:

- `navigation` - Page navigation shortcuts
- `search` - Search-related shortcuts
- `actions` - User actions (save, delete, etc.)
- `ui` - UI controls (toggle sidebar, theme, etc.)
- `accessibility` - Accessibility features
- `system` - System-level shortcuts (help, settings, etc.)

## Conflict Detection

The system automatically detects and prevents:

1. **Duplicate bindings** - Two shortcuts with the same key combination
2. **Browser conflicts** - Shortcuts that conflict with native browser shortcuts (Ctrl+R, Ctrl+T, etc.)

When customizing a shortcut, the UI will display an error if a conflict is detected.

## Accessibility Features

### Focus Management
- Help overlay traps focus within the dialog
- Focus returns to previously focused element when closed
- Tab navigation cycles through focusable elements

### ARIA Attributes
- `role="dialog"` on help overlay
- `aria-modal="true"` for modal behavior
- `aria-label` on interactive elements
- `aria-labelledby` for dialog title

### Keyboard Navigation
- `Escape` closes the help overlay
- `Tab` and `Shift+Tab` navigate through shortcuts
- Shortcuts don't trigger when focus is in input fields

### Screen Reader Support
- Semantic HTML structure
- Proper heading hierarchy
- Descriptive labels for all shortcuts

## Input Field Handling

Shortcuts are automatically disabled when focus is in:
- `<input>` elements
- `<textarea>` elements
- `<select>` elements
- Elements with `contenteditable="true"`

This prevents shortcuts from interfering with text input.

## Persistence

Shortcut customizations are stored in localStorage under the key `stellar-keyboard-shortcuts`:

```json
{
  "customBindings": {
    "open-search": {
      "key": "f",
      "modifiers": ["ctrl", "shift"]
    }
  },
  "disabledShortcuts": ["refresh-data"],
  "enabled": true
}
```

## Testing

Comprehensive test coverage includes:

```bash
# Run all tests
pnpm test

# Run keyboard shortcuts tests only
pnpm test keyboard-shortcuts
```

Test files:
- `__tests__/keyboard-shortcuts/utils.test.ts` - Utility functions
- `__tests__/keyboard-shortcuts/registry.test.ts` - Shortcut registry
- `__tests__/keyboard-shortcuts/KeyboardShortcutsContext.test.tsx` - Context and hooks
- `__tests__/keyboard-shortcuts/ShortcutHelpOverlay.test.tsx` - Help overlay component

## API Reference

### Types

```typescript
type ModifierKey = 'ctrl' | 'alt' | 'shift' | 'meta';
type Platform = 'mac' | 'windows' | 'linux' | 'unknown';
type ShortcutCategory = 'navigation' | 'search' | 'actions' | 'ui' | 'accessibility' | 'system';

interface KeyBinding {
  key: string;
  modifiers?: ModifierKey[];
  mac?: { key: string; modifiers?: ModifierKey[] };
  windows?: { key: string; modifiers?: ModifierKey[] };
  linux?: { key: string; modifiers?: ModifierKey[] };
}

interface ShortcutAction {
  id: string;
  name: string;
  description: string;
  category: ShortcutCategory;
  defaultBinding: KeyBinding;
  handler: (event: KeyboardEvent) => void | Promise<void>;
  enabled?: boolean;
  preventDefault?: boolean;
  stopPropagation?: boolean;
}
```

### Context API

```typescript
interface KeyboardShortcutsContextType {
  platform: Platform;
  config: ShortcutConfig;
  setConfig: (config: Partial<ShortcutConfig>) => void;
  resetConfig: () => void;
  registerShortcut: (action: ShortcutAction) => void;
  unregisterShortcut: (actionId: string) => void;
  getShortcuts: () => ShortcutAction[];
  getShortcutsByCategory: (category: string) => ShortcutAction[];
  customizeBinding: (actionId: string, binding: KeyBinding) => void;
  toggleShortcut: (actionId: string, enabled: boolean) => void;
  showHelp: () => void;
  hideHelp: () => void;
  isHelpVisible: boolean;
}
```

### Utility Functions

```typescript
// Platform detection
function detectPlatform(): Platform;

// Get platform-specific binding
function getPlatformBinding(binding: KeyBinding, platform: Platform): KeyBinding;

// Normalize key name
function normalizeKey(key: string): string;

// Check if event matches binding
function matchesBinding(event: KeyboardEvent, binding: KeyBinding, platform: Platform): boolean;

// Check if bindings conflict
function bindingsConflict(binding1: KeyBinding, binding2: KeyBinding, platform: Platform): boolean;

// Format binding for display
function formatBinding(binding: KeyBinding, platform: Platform): string;

// Check if input is focused
function isInputFocused(): boolean;

// Check if binding conflicts with browser shortcuts
function conflictsWithBrowser(binding: KeyBinding, platform: Platform): boolean;
```

## Best Practices

1. **Use semantic IDs** - Use descriptive, kebab-case IDs like `open-search` or `toggle-sidebar`

2. **Provide clear descriptions** - Help users understand what each shortcut does

3. **Choose appropriate categories** - Group related shortcuts together

4. **Avoid browser conflicts** - Don't use common browser shortcuts like Ctrl+R, Ctrl+T, Ctrl+W

5. **Use platform-specific bindings** - Provide Mac alternatives using Cmd instead of Ctrl

6. **Test on all platforms** - Verify shortcuts work correctly on macOS, Windows, and Linux

7. **Document custom shortcuts** - Add comments explaining why specific keys were chosen

8. **Handle errors gracefully** - Wrap handlers in try-catch to prevent crashes

## Troubleshooting

### Shortcut not triggering

1. Check if focus is in an input field
2. Verify the shortcut is enabled in settings
3. Check for conflicts with other shortcuts
4. Ensure the correct modifiers are pressed

### Conflict with browser shortcut

1. Choose a different key combination
2. Add additional modifiers (e.g., Ctrl+Shift+K instead of Ctrl+K)
3. Use platform-specific overrides

### Customization not persisting

1. Check browser localStorage is enabled
2. Verify no errors in browser console
3. Try clearing localStorage and reconfiguring

## Contributing

When adding new shortcuts:

1. Add the shortcut definition to `default-shortcuts.ts`
2. Update this documentation with the new shortcut
3. Add tests for the new functionality
4. Ensure accessibility requirements are met
5. Test on multiple platforms

## License

Part of the Stellar Insights application.
