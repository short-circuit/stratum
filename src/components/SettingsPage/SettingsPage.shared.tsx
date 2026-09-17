//! Shared state, types, and logic for SettingsPage.
//! Barrel that re-exports the `useSettingsPage()` hook (implementation in
//! ./useSettingsPage) for the desktop and mobile variants.
export { useSettingsPage } from './useSettingsPage';
export type { SettingsTab, SettingsData } from './useSettingsPage';
