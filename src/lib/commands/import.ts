import { invoke } from '@tauri-apps/api/core';
import type { ImportOptionsDto, ImportPreview, ImportReport } from '../types';

/** Preview what importing `path` would do (never writes). */
export async function previewImport(
  path: string,
  opts?: Partial<ImportOptionsDto>,
): Promise<ImportPreview> {
  return invoke('preview_import', { path, opts });
}

/** Import `path` into the active vault (respecting `opts.dry_run`). */
export async function importVault(
  path: string,
  opts?: Partial<ImportOptionsDto>,
): Promise<ImportReport> {
  return invoke('import_vault', { path, opts });
}
