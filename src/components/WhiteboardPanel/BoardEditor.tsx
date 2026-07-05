import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Chip from '@mui/material/Chip';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import { Excalidraw, MainMenu, exportToCanvas } from '@excalidraw/excalidraw';
import type { ExcalidrawImperativeAPI, SceneData, LibraryItems } from '@excalidraw/excalidraw/types';

interface BoardEditorProps {
  activeBoard: string;
  sceneData: SceneData | null;
  libraryItems: LibraryItems | null;
  dirty: boolean;
  excalidrawRef: React.MutableRefObject<ExcalidrawImperativeAPI | null>;
  onChange: () => void;
  onLibraryChange: (items: LibraryItems) => void;
  onNavigateBack: () => void;
}

const THUMBNAIL_MAX = 300;

function invertHexColor(hex: string): string {
  const clean = hex.replace('#', '');
  if (clean.length !== 6) return hex;
  const r = (255 - parseInt(clean.substring(0, 2), 16)).toString(16).padStart(2, '0');
  const g = (255 - parseInt(clean.substring(2, 4), 16)).toString(16).padStart(2, '0');
  const b = (255 - parseInt(clean.substring(4, 6), 16)).toString(16).padStart(2, '0');
  return `#${r}${g}${b}`;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function adaptElementsTheme(elements: readonly any[], toDark: boolean): any[] {
  return elements.map(el => {
    if (!el || typeof el !== 'object') return el;
    const copy = { ...el };
    // Determine if this element likely uses dark-on-light or light-on-dark colors
    // by checking stroke and background lightness
    if (copy.strokeColor && copy.strokeColor.startsWith('#')) {
      const r = parseInt(copy.strokeColor.slice(1, 3), 16);
      const g = parseInt(copy.strokeColor.slice(3, 5), 16);
      const b = parseInt(copy.strokeColor.slice(5, 7), 16);
      const luminance = (r * 0.299 + g * 0.587 + b * 0.114) / 255;
      const isDarkStroke = luminance < 0.5;
      // If we need dark theme but element has dark strokes → invert
      // If we need light theme but element has light strokes → invert
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

async function generateThumbnail(
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

export { generateThumbnail, adaptElementsTheme, invertHexColor };
export type { BoardEditorProps };

export default function BoardEditor({
  activeBoard,
  sceneData,
  libraryItems,
  dirty,
  excalidrawRef,
  onChange,
  onLibraryChange,
  onNavigateBack,
}: BoardEditorProps) {
  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 1.5, py: 1, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.default' }}>
        <IconButton size="small" onClick={onNavigateBack}>
          <ArrowBackIcon fontSize="small" />
        </IconButton>
        <Typography variant="body2" sx={{ fontWeight: 500 }}>{activeBoard}</Typography>
        {dirty && (
          <Chip label="Unsaved" size="small" color="warning" variant="outlined" sx={{ fontSize: '0.65rem', height: 20 }} />
        )}
        <Box sx={{ flex: 1 }} />
      </Box>

      <Box sx={{ flex: 1, position: 'relative', minHeight: 0 }}>
        {sceneData && libraryItems && (
          <Excalidraw
            key={activeBoard}
            theme={document.documentElement.classList.contains('dark') ? 'dark' : 'light'}
            initialData={{ ...sceneData, libraryItems }}
            excalidrawAPI={(api: ExcalidrawImperativeAPI) => { excalidrawRef.current = api; }}
            onChange={onChange}
            onLibraryChange={onLibraryChange}
          >
            <MainMenu>
              <MainMenu.DefaultItems.Help />
              <MainMenu.DefaultItems.ClearCanvas />
              <MainMenu.DefaultItems.ToggleTheme />
              <MainMenu.DefaultItems.ChangeCanvasBackground />
              <MainMenu.DefaultItems.Export />
              <MainMenu.DefaultItems.SaveAsImage />
              <MainMenu.DefaultItems.SearchMenu />
            </MainMenu>
          </Excalidraw>
        )}
      </Box>
    </Box>
  );
}
