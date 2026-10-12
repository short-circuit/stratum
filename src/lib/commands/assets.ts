import { invoke } from '@tauri-apps/api/core';
import type { AssetRefDto, AssetBytesDto } from '../types';

export async function importAssetPath(
  path: string,
  destName?: string | null,
): Promise<AssetRefDto> {
  return invoke('import_asset_path', { path, destName });
}

export async function importAssetBytes(
  dataBase64: string,
  originalName: string,
  mime?: string | null,
  destName?: string | null,
): Promise<AssetRefDto> {
  return invoke('import_asset_bytes', { dataBase64, originalName, mime, destName });
}

export async function readAsset(refPath: string): Promise<AssetBytesDto> {
  return invoke('read_asset', { refPath });
}

export async function deleteAssetRef(refPath: string): Promise<void> {
  return invoke('delete_asset', { refPath });
}

export async function listAssetRefs(): Promise<AssetRefDto[]> {
  return invoke('list_asset_refs');
}

export async function getPageAssetRefs(pagePath: string): Promise<AssetRefDto[]> {
  return invoke('get_page_asset_refs', { pagePath });
}
