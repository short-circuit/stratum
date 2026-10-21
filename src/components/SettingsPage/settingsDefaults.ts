//! Pure default objects for the Settings page form state.
//!
//! Extracted from useSettingsPage.ts during the shared-settings-primitives
//! refactor to keep the hook module under the 500-line file gate. These are
//! plain data constants (no logic) and are referenced whenever a settings
//! slice is absent from the fetched settings payload.

export const DEFAULT_BACKUP = {
  enabled: false,
  mode: 'folder',
  interval_minutes: 60,
  exclude: ['.git', '.pkm/blocks.db', '.pkm/search', '.pkm/backup_state.json', '.pkm/sync_state.json', '.DS_Store'],
  webdav_url: null,
  webdav_username: null,
  webdav_password: null,
  folder_path: null,
};

export const DEFAULT_RESEARCH = {
  searxng_endpoint: 'http://localhost:8888',
  max_results: 3,
  max_depth: 2,
};

export const DEFAULT_THEME = {
  dark_mode: true,
  primary_color: '#f97316',
  secondary_color: '#6b7280',
  font_size: 16,
};

export const DEFAULT_SYNC = {
  mode: 'manual',
  remote_url: null,
  branch: 'main',
  auto_commit_interval_secs: 300,
  auto_sync_interval_secs: 1800,
  ssh_key_path: null,
  commit_template:
    'stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted',
};

export const DEFAULT_TTS = {
  endpoint: '',
  api_key: null,
  voice: 'alloy',
  format: 'mp3',
  speed: 1.0,
  use_llm_gateway_and_auth: false,
};

export const DEFAULT_TRAY = {
  enabled: true,
  close_to_tray: true,
  quick_capture_shortcut: 'CmdOrCtrl+Shift+Space',
  capture_target: 'journal',
  inbox_path: 'Inbox.md',
};

export const DEFAULT_VOICE = { enabled: true, target: 'journal:today' };

export const DEFAULT_REMINDERS = { enabled: true, sound: true };

export const DEFAULT_LAYOUT = { pages_view: 'flat' };
