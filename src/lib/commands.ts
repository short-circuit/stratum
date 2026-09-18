// Barrel re-export for all Tauri IPC command wrappers.
//
// Consumers import via `import * as api from '../lib/commands'` (or named imports).
// The domain implementations live in sibling modules under `./commands/`:
//   - pages.ts     — page/block, reindex, normalize
//   - knowledge.ts — search/graph/query, backlinks, link resolution
//   - sync.ts      — sync/git operations
//   - settings.ts  — vault/settings
//   - features.ts  — AI, dictation/TTS, export, flashcards, kanban, whiteboards, templates
//   - plugins.ts   — WASM plugin lifecycle + host-function test commands
export * from './commands/pages';
export * from './commands/knowledge';
export * from './commands/sync';
export * from './commands/settings';
export * from './commands/features';
export * from './commands/plugins';
