# Frontend Development Guide

This document covers the React + TypeScript frontend architecture, component patterns, state management, styling conventions, and development workflow for Stratum.

---

## Architecture Overview

Stratum's frontend is a **React 19** single-page application built with **TypeScript 6** (strict mode), bundled with **Vite**, and styled with **MUI v6** (Material UI) using the `sx` prop — no Tailwind CSS, no CSS-in-JS libraries beyond MUI's built-in system.

### Tech Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| Framework | React 19 | Concurrent features, `use()` hook |
| Language | TypeScript 6 | Strict mode, no `as any`, no `@ts-ignore` |
| Bundler | Vite 6 | HMR, lazy route splitting |
| UI Library | MUI v6 (`@mui/material`) | `sx` prop only, no `styled()` API |
| State | Zustand 5 | Lightweight, no boilerplate |
| Routing | React Router v7 | File-adjacent route definitions |
| Icons | MUI Icons (`@mui/icons-material`) | Material Design icons |
| Editor | BlockNote (`@blocknote/react`) | ProseMirror-based block editor |
| Desktop Bridge | Tauri v2 (`@tauri-apps/api`) | `invoke()` for Rust commands |
| Testing | Vitest + React Testing Library | Component + hook tests |

### File Layout

```
src/
├── main.tsx                 # Bootstrap, settings load, React root
├── App.tsx                  # Root layout, routes, close handler
├── global.css               # CSS variables, safe-area, overrides
├── lib/
│   ├── types.ts             # TypeScript DTOs (mirror Rust structs)
│   ├── commands.ts          # 64+ Tauri invoke() wrappers
│   ├── theme.ts             # CSS variable generation (--primary-* shades)
│   ├── muiTheme.ts          # MUI Theme creation from config
│   ├── wikiLinks.ts         # Wiki-link parsing/serialization
│   ├── libraryStore.ts      # Module-level library JSON cache
│   ├── useCtrlHeld.ts       # Hook: Ctrl/Meta key tracking
│   ├── useMathInline.tsx    # Hook: ProseMirror inline KaTeX plugin
│   └── hooks/
│       ├── useAsyncData.ts
│       ├── useDebounce.ts
│       ├── useAutoSave.ts
│       ├── useResponsive.ts
│       └── index.ts
├── stores/
│   ├── appStore.ts          # Core Zustand store
│   ├── settingsStore.ts     # Theme + AI + sync config
│   ├── graphStore.ts        # Graph data + settings
│   └── syncStore.ts         # Sync status + commits
├── components/
│   ├── ui/                  # Reusable UI primitives
│   ├── Sidebar/
│   ├── PageView.tsx
│   └── ... (panel components)
└── test/                    # Test utilities
```

### Data Flow

All data operations follow a strict unidirectional flow:

```
Component
  → src/lib/commands.ts (typed invoke wrapper)
    → Tauri IPC (serialized as JSON)
      → Rust command handler (src-tauri/src/commands/)
        → Crate business logic (pkm-block, pkm-index, pkm-sync, etc.)
          → SQLite / filesystem / Tantivy
```

Components **never** call `invoke()` directly — always through `commands.ts`. Zustand stores wrap `commands.ts` calls and expose loading/error states.

---

## Component Hierarchy

### Panel Components (route-mapped)

| Component | Route | Purpose |
|-----------|-------|---------|
| `PagesHome` | `/` | Page list with block counts |
| `JournalPanel` | `/journal` | Calendar + daily journal creation |
| `PageView` | `/page/:pagePath` | Block editor + backlinks + connections |
| `SearchPanel` | `/search` | Full-text + tag search |
| `QueryPanel` | `/query` | Datalog query input + results table |
| `GraphPanel` | `/graph` | 3D force-directed graph |
| `TemplatesPanel` | `/templates` | Template list + apply with variables |
| `FlashcardsPanel` | `/flashcards` | SRS card review (SM-2) |
| `KanbanPanel` | `/kanban` | Drag-and-drop Kanban board |
| `WhiteboardPanel` | `/whiteboards` | Excalidraw spatial canvas |
| `SettingsPage` | `/settings` | 6-tab app configuration |

### Editor Sub-components

| Component | Parent | Purpose |
|-----------|--------|---------|
| `OutlinerEditor` | `PageView` | BlockNote-based outliner with auto-save, markers, wiki-links |
| `BacklinksPanel` | `PageView` | Linked references + unlinked mentions + hover preview |
| `SuggestedConnectionsPanel` | `PageView` | AI-suggested wiki-link connections |
| `MermaidBlock` | `OutlinerEditor` | Custom BlockNote block for Mermaid diagrams |
| `AISlashMenu` | `OutlinerEditor` | Slash menu with AI actions |
| `AIFormattingToolbar` | `OutlinerEditor` | Formatting toolbar with AI buttons |
| `AutocompletePopup` | `OutlinerEditor` | Popover for wiki-link autocomplete |
| `LinkPreviewPopup` | `OutlinerEditor` | Hover preview for wiki-links |
| `MathEditorModal` | `OutlinerEditor` | LaTeX editor with live KaTeX preview |
| `MathSymbolPalette` | `MathEditorModal` | Tabbed symbol palette |
| `MarkerBadge` | `OutlinerEditor` | Colored chip for task markers |
| `KanbanEditDialog` | `KanbanPanel` | Edit card content, marker, priority |

### Navigation & Utility

| Component | Purpose |
|-----------|---------|
| `Sidebar` | Collapsible navigation + page tree + create/delete/export |
| `VaultPicker` | Landing page when no vault is configured |
| `StratumIcon` | App icon SVG renderer |

---

## State Management

### Zustand Stores

Stratum uses four domain-specific Zustand stores, each in `src/stores/`:

#### `appStore` — Core Application State

```typescript
// src/stores/appStore.ts
import { create } from 'zustand';
import type { VaultInfo, PageDto } from '../lib/types';
import * as api from '../lib/commands';

interface AppState {
  vault: VaultInfo | null;
  pages: PageDto[];
  currentPage: PageDto | null;
  loading: boolean;
  error: string | null;

  loadVault: () => Promise<void>;
  loadPages: () => Promise<void>;
  openPage: (path: string) => Promise<void>;
  createPage: (path: string, title?: string) => Promise<void>;
  deletePage: (path: string) => Promise<void>;
}

export const useStore = create<AppState>((set, get) => ({
  vault: null,
  pages: [],
  currentPage: null,
  loading: false,
  error: null,

  loadVault: async () => {
    try {
      set({ loading: true, error: null });
      const vault = await api.getVaultInfo();
      set({ vault });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  openPage: async (path: string) => {
    try {
      set({ loading: true });
      const page = await api.openPage(path);
      set({ currentPage: page });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },
  // ...
}));
```

**Usage in a component:**

```typescript
// src/components/PageView.tsx
import { useStore } from '../stores/appStore';

export default function PageView() {
  const { currentPage, openPage, deletePage } = useStore();

  useEffect(() => {
    if (pagePath) {
      openPage(decodeURIComponent(pagePath));
    }
  }, [pagePath, openPage]);
  // ...
}
```

#### `settingsStore` — Theme, AI, and Sync Configuration

Manages user preferences including theme colors, AI provider settings, research configuration, and sync intervals. Persisted to disk via Tauri commands.

#### `graphStore` — Graph Data and View Settings

Holds the node/edge data for the force-directed graph, connected components, orphans, and interactive settings (repulsion, link distance, visibility toggles).

#### `syncStore` — Git Sync Status

Tracks sync state (idle/syncing/error), commit log, and conflict resolution state.

### Async Data Flow Pattern

Every async operation in a store follows the same pattern:

```typescript
actionName: async () => {
  try {
    set({ loading: true, error: null });
    const result = await api.someCommand();
    set({ data: result });
  } catch (e) {
    set({ error: String(e) });
  } finally {
    set({ loading: false });
  }
}
```

Components then use Zustand selectors to subscribe to the specific slice they need:

```typescript
const { vault, loading, error } = useStore(state => ({
  vault: state.vault,
  loading: state.loading,
  error: state.error,
}));
```

---

## UI Primitives (`src/components/ui/`)

The `ui/` directory holds reusable, presentation-only components. They have no business logic and accept all behavior through props.

### Available Components

| Component | Props | Purpose |
|-----------|-------|---------|
| `LoadingOverlay` | `message?`, `overlay?` | Centered spinner, absolute overlay or inline |
| `AILoadingOverlay` | `loading: boolean`, `message?` | Fullscreen portal overlay for AI operations |
| `ErrorAlert` | `message`, `onDismiss?` | Dismissable error Alert |
| `EmptyState` | `icon?`, `message`, `description?`, `actionLabel?`, `onAction?` | Centered empty state with optional CTA |
| `PageHeader` | `title`, `actions?`, `onBack?` | Consistent header bar |
| `ConfirmDialog` | `open`, `title`, `message`, `onConfirm`, `onCancel` | Confirmation dialog |
| `SliderRow` | `label`, `value`, `min`, `max`, `onChange` | Label + slider + display value |
| `PassphraseModal` | — | SSH key passphrase input |
| `ConflictModal` | — | Git conflict resolution |

### Usage Examples

**LoadingOverlay** — wraps any section that needs a loading state:

```typescript
import LoadingOverlay from '../ui/LoadingOverlay';

function MyPanel() {
  const [loading, setLoading] = useState(true);
  return (
    <Box sx={{ position: 'relative', height: 200 }}>
      {loading && <LoadingOverlay message="Loading data..." />}
      {/* content */}
    </Box>
  );
}
```

**EmptyState** — shown when a panel has no content:

```typescript
import EmptyState from '../ui/EmptyState';

function SearchPanel() {
  if (results.length === 0 && !searching) {
    return (
      <EmptyState
        icon={<SearchIcon />}
        message="No results found"
        description="Try a different search term"
      />
    );
  }
  // ...
}
```

**ConfirmDialog** — destructive or important actions:

```typescript
import ConfirmDialog from '../ui/ConfirmDialog';

function PageView() {
  const [deleteOpen, setDeleteOpen] = useState(false);

  return (
    <>
      <ConfirmDialog
        open={deleteOpen}
        title="Delete Page"
        message={`Delete "${currentPage.title}"? This cannot be undone.`}
        confirmLabel="Delete"
        confirmColor="error"
        onConfirm={handleDelete}
        onCancel={() => setDeleteOpen(false)}
      />
    </>
  );
}
```

**AILoadingOverlay** — fullscreen portal for AI operations (replaces duplicated inline portal code):

```typescript
import AILoadingOverlay from '../ui/AILoadingOverlay';

function AIFeature() {
  const [busy, setBusy] = useState<string | null>(null);

  return (
    <>
      <AILoadingOverlay loading={!!busy} message={busy ? `AI ${busy}...` : undefined} />
      <button onClick={() => setBusy('rewrite')}>Rewrite</button>
    </>
  );
}
```

### Adding a New UI Primitive

1. Create the component in `src/components/ui/` (keep it < 50 lines)
2. Every component must define an `interface Props` at the top
3. Export it from `src/components/ui/index.ts`
4. Add it to the component table in `AGENTS.md`

---

## Custom Hooks

All hooks live in `src/lib/hooks/` and use **named exports** (not default exports).

### `useAsyncData<T>`

Generic async fetcher with loading/error/data states. Ideal for one-shot data fetching.

```typescript
// src/lib/hooks/useAsyncData.ts
export function useAsyncData<T>(
  fetcher: () => Promise<T>,
  deps: unknown[] = [],
): { data: T | null; loading: boolean; error: string | null; refresh: () => Promise<void> }
```

**Usage:**

```typescript
import { useAsyncData } from '../lib/hooks/useAsyncData';

function MyPanel() {
  const { data, loading, error, refresh } = useAsyncData(
    () => api.listPages(),
    [], // re-fetch when these change
  );

  if (loading) return <LoadingOverlay />;
  if (error) return <ErrorAlert message={error} />;
  if (!data) return <EmptyState message="No pages" />;

  return <List>{data.map(page => <ListItem key={page.path}>{page.title}</ListItem>)}</List>;
}
```

### `useDebounce<T>`

Debounces a value by a given delay. Useful for search-as-you-type.

```typescript
// src/lib/hooks/useDebounce.ts
export function useDebounce<T>(value: T, delay: number): T
```

**Usage:**

```typescript
import { useDebounce } from '../lib/hooks/useDebounce';

function SearchPanel() {
  const [query, setQuery] = useState('');
  const debouncedQuery = useDebounce(query, 300);

  useEffect(() => {
    if (debouncedQuery.trim()) {
      api.searchBlocks(debouncedQuery, 20);
    }
  }, [debouncedQuery]);
  // ...
}
```

### `useAutoSave`

Debounced save with manual flush support.

```typescript
// src/lib/hooks/useAutoSave.ts
export function useAutoSave(
  saveFn: () => Promise<void>,
  delay?: number,
): { scheduleSave: () => void; flush: () => Promise<void>; saving: boolean }
```

**Usage:**

```typescript
import { useAutoSave } from '../lib/hooks/useAutoSave';

function Editor({ content, pagePath }: { content: string; pagePath: string }) {
  const { scheduleSave, flush, saving } = useAutoSave(async () => {
    await api.savePage(pagePath, content);
  }, 500);

  useEffect(() => {
    // Flush pending saves before navigation
    return () => { flush(); };
  }, [flush]);

  return (
    <>
      {saving && <Typography variant="caption">Saving...</Typography>}
      <textarea value={content} onChange={scheduleSave} />
    </>
  );
}
```

### `useResponsive`

Breakpoint detection for mobile vs desktop layouts.

```typescript
// src/lib/hooks/useResponsive.ts
export function useResponsive(): { isMobile: boolean; isDesktop: boolean; width: number }
```

The breakpoint is **768px** (tablet portrait width).

**Usage:**

```typescript
import { useResponsive } from '../lib/hooks/useResponsive';

function MyPanel() {
  const { isMobile, isDesktop } = useResponsive();

  if (isMobile) return <MobileVariant />;
  return <DesktopVariant />;
}
```

### `useCtrlHeld`

Tracks whether Ctrl (or Meta on macOS) is currently held. Used for preview-on-hover behavior.

```typescript
// src/lib/useCtrlHeld.ts
import { useEffect, useRef } from 'react';

export function useCtrlHeld() {
  const ctrlHeld = useRef(false);

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') ctrlHeld.current = true;
    };
    const up = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') ctrlHeld.current = false;
    };
    const blur = () => { ctrlHeld.current = false; };
    window.addEventListener('keydown', down);
    window.addEventListener('keyup', up);
    window.addEventListener('blur', blur);
    return () => {
      window.removeEventListener('keydown', down);
      window.removeEventListener('keyup', up);
      window.removeEventListener('blur', blur);
    };
  }, []);

  return ctrlHeld;
}
```

**Usage (from BacklinksPanel):**

```typescript
import { useCtrlHeld } from '../lib/useCtrlHeld';

function BacklinksPanel() {
  const ctrlHeld = useCtrlHeld();

  const handleMouseEnter = (item, e) => {
    setTimeout(async () => {
      if (!ctrlHeld.current) return; // Only preview when Ctrl is held
      const page = await api.openPage(item.source_page);
      // show preview popover
    }, 200);
  };
  // ...
}
```

### `useMathInline`

ProseMirror plugin for inline KaTeX rendering within the BlockNote editor.

---

## Component Sizing Rules

These rules are **enforced by convention** — no file or component should grow past these limits:

| Metric | Limit | Action |
|--------|-------|--------|
| File lines | < 500 | Split into sub-modules |
| Component JSX | < 50 lines | Extract sub-components |
| Inline function | < 20 lines | Extract to module-level |
| Same logic in 2+ files | 0 duplicates | Extract to `src/lib/` or `hooks/` |
| Props interface | Required | Every component must define `interface Props` |

### Duplicate Detection Workflow

When you see identical logic in two or more files:

1. Extract the shared logic into a reusable hook or utility in `src/lib/hooks/` or `src/lib/`
2. If it's a presentational element with no business logic, add it to `src/components/ui/`
3. If it's a larger component (50+ lines of JSX), promote it to a shared component

**Real example** — AI loading overlays were duplicated in both `AISlashMenu.tsx` and `AIFormattingToolbar.tsx`:

```typescript
// Before: inline createPortal in both files (duplicated ~8 lines each)
{loading && createPortal(
  <Box sx={{ position: 'fixed', inset: 0, zIndex: 9999, ... }}>
    <CircularProgress size={20} />
    <Typography variant="body2">{message}</Typography>
  </Box>,
  document.body,
)}

// After: extracted to src/components/ui/AILoadingOverlay.tsx
<AILoadingOverlay loading={!!loading} message={message} />
```

---

## Desktop + Mobile Pattern

Stratum components that need platform-specific implementations follow this folder convention:

```
src/components/FeaturePanel/
├── index.tsx                    # Desktop/web implementation
├── FeaturePanel.mobile.tsx      # Mobile variant
├── FeaturePanel.shared.tsx      # Shared logic, hooks, types
└── FeaturePanel.test.tsx        # Tests
```

The `index.tsx` uses `useResponsive` to conditionally render:

```typescript
import { useResponsive } from '../../lib/hooks/useResponsive';
import { FeaturePanelDesktop } from './index';
import { FeaturePanelMobile } from './FeaturePanel.mobile';

export default function FeaturePanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <FeaturePanelMobile />;
  return <FeaturePanelDesktop />;
}
```

The `.shared.tsx` file holds:
- Type definitions
- Shared hooks and data transformations
- Pure rendering helpers that don't depend on layout

This keeps mobile-specific code from bloating the desktop bundle and prevents responsive conditionals from spreading through component logic.

---

## Adding a New Feature

Follow this step-by-step process when adding a new panel or feature:

### 1. Create the Component

```typescript
// src/components/MyFeature.tsx
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useStore } from '../stores/appStore';

interface Props {
  /* document all props */
}

export default function MyFeature() {
  const { vault } = useStore();
  return (
    <Box sx={{ p: 2 }}>
      <Typography variant="h5">My Feature</Typography>
    </Box>
  );
}
```

**Rules:**
- Define `interface Props` at the top of every component (even if empty initially)
- Keep JSX under 50 lines — extract sub-components early
- Use MUI `sx` prop exclusively for styling — no CSS modules, no styled-components
- Use `useStore` selectors to subscribe to Zustand state

### 2. Add the Route

```typescript
// src/App.tsx
import MyFeature from './components/MyFeature';

// Inside <Routes>
<Route path="/my-feature" element={<MyFeature />} />
```

Add the route link to the Sidebar component.

### 3. Create the Store (if needed)

```typescript
// src/stores/myFeatureStore.ts
import { create } from 'zustand';
import * as api from '../lib/commands';

interface MyFeatureState {
  data: MyType[];
  loading: boolean;
  fetchData: () => Promise<void>;
}

export const useMyFeatureStore = create<MyFeatureState>((set) => ({
  data: [],
  loading: false,
  fetchData: async () => {
    set({ loading: true });
    try {
      const result = await api.someCommand();
      set({ data: result });
    } finally {
      set({ loading: false });
    }
  },
}));
```

### 4. Register Tauri Commands (if needed)

Add a typed wrapper in `src/lib/commands.ts`:

```typescript
// src/lib/commands.ts
export async function myFeatureCommand(param: string): Promise<MyResult> {
  return invoke('my_feature_command', { param });
}
```

Then implement the Rust command in `src-tauri/src/commands/`.

### 5. Add the User Guide

Create `docs/guide/my-feature.md` documenting the feature from the user's perspective.

### 6. Update AGENTS.md

Add the new component to the `Frontend Components` table and crate to the `Crate Dependency Graph` if applicable.

### 7. Add Tests

```typescript
// src/components/MyFeature.test.tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import MyFeature from './MyFeature';

describe('MyFeature', () => {
  it('renders the feature title', () => {
    render(<MyFeature />);
    expect(screen.getByText('My Feature')).toBeDefined();
  });
});
```

---

## Code Style

### TypeScript Strictness

- **`tsconfig.json`** enables `strict: true` with `noUncheckedIndexedAccess`
- **No `as any`** — use proper type narrowing or `unknown` + type guards
- **No `@ts-ignore`** or `@ts-expect-error` — fix the type instead
- **Named exports** for hooks and utilities, default exports for components

### Import Order

```typescript
// 1. React / framework
import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';

// 2. Third-party libraries
import Box from '@mui/material/Box';
import { create } from 'zustand';

// 3. Internal modules (sorted by depth)
import { useStore } from '../stores/appStore';
import * as api from '../lib/commands';
import type { PageDto } from '../lib/types';
import LoadingOverlay from './ui/LoadingOverlay';
```

### Styling with MUI `sx`

Use the `sx` prop for all styles. Avoid `styled()` API or CSS modules entirely.

```typescript
// ✅ Correct
<Box sx={{ display: 'flex', gap: 1.5, p: 2, bgcolor: 'background.paper' }}>

// ❌ Avoid
const StyledBox = styled(Box)(({ theme }) => ({
  display: 'flex',
  gap: theme.spacing(1.5),
}));
```

The `sx` prop accepts all MUI theme tokens (`background.paper`, `text.secondary`, `divider`, etc.) and supports shorthand properties (`p: 2` = padding 16px, `gap: 1.5` = gap 12px).

### Component Patterns

**Props interface (required at top of every component):**

```typescript
interface Props {
  pagePath: string;
  onNavigate?: (path: string) => void;
}
```

**Conditional rendering:**

```typescript
// Loading state
if (loading) return <LoadingOverlay message="Loading..." />;

// Error state
if (error) return <ErrorAlert message={error} onDismiss={clearError} />;

// Empty state
if (data.length === 0) return <EmptyState message="No items found" />;

// Happy path
return <List>{data.map(item => <ListItem key={item.id}>{item.name}</ListItem>)}</List>;
```

**Event handlers:**

```typescript
const handleClick = useCallback(async () => {
  try {
    setLoading(true);
    await api.someCommand();
  } catch (e) {
    console.error('[MyFeature] action failed:', e);
  } finally {
    setLoading(false);
  }
}, [deps]);
```

Always wrap async event handlers in try/catch/finally and log errors with a `[ComponentName]` prefix.

---

## Testing

### Setup

Tests use **Vitest** with **React Testing Library**. Test files co-locate with components:

```
src/components/FeaturePanel/
├── FeaturePanel.tsx
├── FeaturePanel.mobile.tsx
├── FeaturePanel.shared.tsx
└── FeaturePanel.test.tsx
```

### Writing Tests

**Component tests:**

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import MyComponent from './MyComponent';

describe('MyComponent', () => {
  it('renders the title', () => {
    render(<MyComponent title="Hello" />);
    expect(screen.getByText('Hello')).toBeDefined();
  });

  it('calls onClick when button is pressed', () => {
    const onClick = vi.fn();
    render(<MyComponent onClick={onClick} />);
    fireEvent.click(screen.getByRole('button'));
    expect(onClick).toHaveBeenCalledOnce();
  });
});
```

**Hook tests:**

```typescript
import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useDebounce } from './useDebounce';

describe('useDebounce', () => {
  it('returns initial value immediately', () => {
    const { result } = renderHook(() => useDebounce('hello', 300));
    expect(result.current).toBe('hello');
  });
});
```

### Running Tests

```bash
npm run test        # Run all tests
npm run test:watch  # Watch mode
npm run test:coverage  # With coverage report
```

---

## Vite Configuration

The Vite config (`vite.config.ts`) handles:

- **React plugin** with Fast Refresh
- **Path aliases** (`@/` maps to `src/`)
- **Build splitting** for route-based lazy loading
- **Environment variables** via `import.meta.env.VITE_*`

```typescript
// vite.config.ts (simplified)
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { '@': path.resolve(__dirname, 'src') },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom', '@mui/material'],
          editor: ['@blocknote/react', '@blocknote/core'],
          graph: ['react-force-graph-2d', 'd3-force'],
        },
      },
    },
  },
});
```

---

## Performance Guidelines

| Metric | Target |
|--------|--------|
| Component render | < 10ms (DevTools Profiler) |
| Re-render scope | Minimize via Zustand selectors, `React.memo` sparingly |
| Bundle contribution | Each panel < 50KB gzipped |
| Images/Assets | Lazy-loaded, WebP format |
| Event handlers | Debounce at 300ms (search), 500ms (save) |

### Zustand Selector Best Practices

```typescript
// ✅ Good — subscribes only to vault
const vault = useStore(state => state.vault);

// ✅ Good — subscribes to two specific values
const { loading, error } = useStore(state => ({
  loading: state.loading,
  error: state.error,
}));

// ❌ Avoid — subscribes to entire store, re-renders on every change
const store = useStore();
```

### Avoiding Unnecessary Re-renders

- Extract `useCallback` wrappers for function props passed to children
- Use `startTransition` for non-urgent state updates:

```typescript
import { startTransition } from 'react';

startTransition(() => {
  setResults(newResults);
});
```

- Memoize expensive computations with `useMemo`
- Keep context values narrow — avoid putting entire state objects in context
