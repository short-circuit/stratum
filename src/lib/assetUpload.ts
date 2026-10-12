import * as api from './commands';
import { isAssetHref, extractAssetRefPath } from './wikiLinks';
import i18n from '../i18n';

export const ASSET_MAX_BYTES = 8 * 1024 * 1024;

export function isAssetOversized(file: File): boolean {
  return file.size > ASSET_MAX_BYTES;
}

export async function fileToBase64(file: File | Blob): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = '';
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const slice = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode(...slice);
  }
  return btoa(binary);
}

// Downscale a large image to max 2048px and re-encode at q0.85. Returns the
// original file when it cannot be re-encoded smaller.
export async function compressImage(file: File): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error);
    reader.onload = () => {
      const img = new Image();
      img.onload = () => {
        const scale = Math.min(1, 2048 / Math.max(img.width, img.height));
        const w = Math.max(1, Math.round(img.width * scale));
        const h = Math.max(1, Math.round(img.height * scale));
        const canvas = document.createElement('canvas');
        canvas.width = w;
        canvas.height = h;
        const ctx = canvas.getContext('2d');
        if (!ctx) {
          resolve(file);
          return;
        }
        ctx.drawImage(img, 0, 0, w, h);
        canvas.toBlob(blob => {
          if (blob && blob.size < file.size) resolve(blob);
          else resolve(file);
        }, 'image/jpeg', 0.85);
      };
      img.onerror = () => resolve(file);
      img.src = String(reader.result);
    };
    reader.readAsDataURL(file);
  });
}

// Produce a base64 payload to upload, compressing oversized images. Throws for
// oversized non-image files (callers surface the "file too large" message).
export async function assetFileToBase64(file: File): Promise<{ b64: string; mime: string }> {
  const isImage = (file.type || '').startsWith('image/');
  if (isAssetOversized(file) && !isImage) {
    throw new Error('file_too_large');
  }
  let payload: Blob = file;
  let mime = file.type || '';
  if (isAssetOversized(file) && isImage) {
    payload = await compressImage(file);
    mime = 'image/jpeg';
  }
  return { b64: await fileToBase64(payload), mime };
}

// Upload a file into the vault and return a stratum-asset: URL for embedding.
export async function uploadAssetFile(file: File): Promise<{ url: string }> {
  const { b64, mime } = await assetFileToBase64(file);
  const ref = await api.importAssetBytes(b64, file.name, mime || undefined, undefined);
  return { url: 'stratum-asset:' + ref.vault_relative_path };
}

// BlockNote editor options that translate between stratum-asset: URLs and
// real asset bytes. uploadFile is invoked by BlockNote when a file/image is
// picked or pasted; resolveFileUrl is invoked on render to fetch bytes.
export function createAssetEditorOptions(): {
  uploadFile: (file: File) => Promise<string | Record<string, any>>;
  resolveFileUrl: (url: string) => Promise<string>;
} {
  return {
    uploadFile: async (file: File) => {
      try {
        return await uploadAssetFile(file);
      } catch (e) {
        console.error(i18n.t('asset.upload_failed'), e);
        return { url: '' };
      }
    },
    resolveFileUrl: async (url: string) => {
      if (!isAssetHref(url)) return url;
      try {
        const ref = extractAssetRefPath(url).replace(/^\/\//, '');
        const bytes = await api.readAsset(ref);
        return `data:${bytes.mime};base64,${bytes.data_base64}`;
      } catch (e) {
        console.error(i18n.t('asset.upload_failed'), e);
        return url;
      }
    },
  };
}
