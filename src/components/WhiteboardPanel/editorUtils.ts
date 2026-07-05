import { exportToCanvas } from '@excalidraw/excalidraw';

const THUMBNAIL_MAX = 300;

export function invertHexColor(hex: string): string {
  const clean = hex.replace('#', '');
  if (clean.length !== 6) return hex;
  const r = (255 - parseInt(clean.substring(0, 2), 16)).toString(16).padStart(2, '0');
  const g = (255 - parseInt(clean.substring(2, 4), 16)).toString(16).padStart(2, '0');
  const b = (255 - parseInt(clean.substring(4, 6), 16)).toString(16).padStart(2, '0');
  return `#${r}${g}${b}`;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function adaptElementsTheme(elements: readonly any[], toDark: boolean): any[] {
  return elements.map(el => {
    if (!el || typeof el !== 'object') return el;
    const copy = { ...el };
    if (copy.strokeColor && copy.strokeColor.startsWith('#')) {
      const r = parseInt(copy.strokeColor.slice(1, 3), 16);
      const g = parseInt(copy.strokeColor.slice(3, 5), 16);
      const b = parseInt(copy.strokeColor.slice(5, 7), 16);
      const luminance = (r * 0.299 + g * 0.587 + b * 0.114) / 255;
      const isDarkStroke = luminance < 0.5;
      if ((toDark && isDarkStroke) || (!toDark && !isDarkStroke)) {
        copy.strokeColor = invertHexColor(copy.strokeColor);
        if (copy.backgroundColor && copy.backgroundColor.startsWith('#')) {
          copy.backgroundColor = invertHexColor(copy.backgroundColor);
        }
      }
    }
    return copy;
  });
}

export async function generateThumbnail(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  elements: readonly any[],
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  appState: any,
  isDark: boolean,
): Promise<{ dataUrl: string; theme: 'dark' | 'light' } | null> {
  try {
    const bgColor = isDark ? '#232329' : '#ffffff';
    const adaptedElements = adaptElementsTheme(elements, isDark);
    const canvas = await exportToCanvas({
      elements: adaptedElements,
      appState: { ...appState, viewBackgroundColor: bgColor, exportWithDarkMode: isDark },
      files: null,
      maxWidthOrHeight: THUMBNAIL_MAX,
      exportPadding: 8,
    });
    return { dataUrl: canvas.toDataURL('image/webp', 0.6), theme: isDark ? 'dark' : 'light' };
  } catch (e) {
    console.warn('Failed to generate thumbnail:', e);
    return null;
  }
}
