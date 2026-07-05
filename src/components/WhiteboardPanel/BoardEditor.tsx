import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Chip from '@mui/material/Chip';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import { Excalidraw, MainMenu } from '@excalidraw/excalidraw';
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
