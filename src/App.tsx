import { useEffect, useMemo } from 'react';
import { Routes, Route } from 'react-router-dom';
import { ThemeProvider } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import Box from '@mui/material/Box';
import Alert from '@mui/material/Alert';
import CircularProgress from '@mui/material/CircularProgress';
import Typography from '@mui/material/Typography';
import { useStore } from './stores/appStore';
import { createMuiTheme } from './lib/muiTheme';
import Sidebar from './components/Sidebar';
import PageView from './components/PageView';
import JournalPanel from './components/JournalPanel';
import PagesHome from './components/PagesHome';
import SearchPanel from './components/SearchPanel';
import QueryPanel from './components/QueryPanel';
import TemplatesPanel from './components/TemplatesPanel';
import FlashcardsPanel from './components/FlashcardsPanel';
import WhiteboardPanel from './components/WhiteboardPanel';
import GraphPanel from './components/GraphPanel';
import SettingsPage from './components/SettingsPage';
import VaultPicker from './components/VaultPicker';

function AppContent() {
  const { vault, loading, loadVault, loadPages, error } = useStore();

  useEffect(() => {
    if (!vault) {
      loadVault();
    }
  }, [loadVault, vault]);

  useEffect(() => {
    if (vault) {
      loadPages();
    }
  }, [vault, loadPages]);

  if (loading && !vault) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', width: '100vw', bgcolor: 'background.default' }}>
        <CircularProgress size={20} sx={{ mr: 1.5 }} />
        <Typography variant="body2" color="text.secondary">Loading vault...</Typography>
      </Box>
    );
  }

  if (!vault) {
    return <VaultPicker />;
  }

  return (
    <Box sx={{ display: 'flex', height: '100vh', width: '100vw', bgcolor: 'background.default', color: 'text.primary' }} className="safe-area-container">
      <Sidebar />
      <Box component="main" sx={{ flexGrow: 1, overflow: 'auto' }} className="safe-area-main">
        {error && (
          <Alert severity="error" sx={{ borderRadius: 0 }}>{error}</Alert>
        )}
        <Routes>
          <Route path="/" element={<PagesHome />} />
          <Route path="/journal" element={<JournalPanel />} />
          <Route path="/page/:pagePath" element={<PageView />} />
          <Route path="/search" element={<SearchPanel />} />
          <Route path="/query" element={<QueryPanel />} />
          <Route path="/templates" element={<TemplatesPanel />} />
          <Route path="/flashcards" element={<FlashcardsPanel />} />
          <Route path="/whiteboards" element={<WhiteboardPanel />} />
          <Route path="/graph" element={<GraphPanel />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </Box>
    </Box>
  );
}

export default function App() {
  const themeConfig = useStore(s => s.themeConfig);

  const muiTheme = useMemo(
    () => createMuiTheme(themeConfig.primaryHex, themeConfig.secondaryHex, themeConfig.dark, themeConfig.fontSize),
    [themeConfig],
  );

  return (
    <ThemeProvider theme={muiTheme}>
      <CssBaseline />
      <AppContent />
    </ThemeProvider>
  );
}
