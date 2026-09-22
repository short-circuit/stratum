// Global lifecycle wiring for the recents store — subscribes to backend
// "pages changed" notifications and routes them into the store's debounced
// refresh entry point.
//
// The backend file watcher keeps the SQLite block store converged with
// external `.md` changes: edits made outside the app, git pulls/sync, or any
// other process writing into the vault. Those changes can reorder or alter
// the sidebar "Recent" list, but they never reach the frontend as a Tauri
// command result — so this module bridges the backend event into the store.
//
// The store's refresh() is debounced (trailing 500ms) and idempotent, so any
// burst of watcher events coalesces into at most one reload and unchanged
// data is never re-published.

/** Backend event name emitted after the file watcher syncs an external change
 *  into the block store (create / modify / rename / delete of a vault file). */
export const PAGES_CHANGED_EVENT = 'pages-changed';

/** Subscribe to backend "pages changed" notifications. Returns an unsubscribe
 *  function. The `@tauri-apps/api/event` module is imported lazily so this
 *  module stays importable in plain-browser/jsdom test environments that do
 *  not provide the Tauri runtime. */
export async function subscribePagesChanged(
  onChange: () => void,
): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen(PAGES_CHANGED_EVENT, () => {
    onChange();
  });
}
