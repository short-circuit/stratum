import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Drawer from '@mui/material/Drawer';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import IconButton from '@mui/material/IconButton';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import Tooltip from '@mui/material/Tooltip';
import Divider from '@mui/material/Divider';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonth';
import ArticleIcon from '@mui/icons-material/Article';
import HubIcon from '@mui/icons-material/Hub';
import SearchIcon from '@mui/icons-material/Search';
import CodeIcon from '@mui/icons-material/Code';
import DescriptionIcon from '@mui/icons-material/Description';
import QuizIcon from '@mui/icons-material/Quiz';
import DrawIcon from '@mui/icons-material/Draw';
import SettingsIcon from '@mui/icons-material/Settings';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import RefreshIcon from '@mui/icons-material/Refresh';
import FileUploadIcon from '@mui/icons-material/FileUpload';
import AddIcon from '@mui/icons-material/Add';
import DeleteIcon from '@mui/icons-material/Delete';
import { useStore } from '../stores/appStore';
import * as api from '../lib/commands';
import StratumIcon from './StratumIcon';

const NAV_ITEMS = [
  { id: 'journal', label: 'Journal', path: '/journal', icon: <CalendarMonthIcon /> },
  { id: 'pages', label: 'Pages', path: '/' as const, icon: <ArticleIcon /> },
  { id: 'graph', label: 'Graph', path: '/graph', icon: <HubIcon /> },
  { id: 'search', label: 'Search', path: '/search', icon: <SearchIcon /> },
  { id: 'query', label: 'Query', path: '/query', icon: <CodeIcon /> },
  { id: 'templates', label: 'Templates', path: '/templates', icon: <DescriptionIcon /> },
  { id: 'flashcards', label: 'Flashcards', path: '/flashcards', icon: <QuizIcon /> },
  { id: 'whiteboards', label: 'Whiteboards', path: '/whiteboards', icon: <DrawIcon /> },
  { id: 'settings', label: 'Settings', path: '/settings', icon: <SettingsIcon /> },
] as const;

type TabId = (typeof NAV_ITEMS)[number]['id'];

const DRAWER_WIDTH = 224;
const DRAWER_COLLAPSED = 52;

export default function Sidebar() {
  const { pages, vault, loadPages, createPage, deletePage } = useStore();
  const navigate = useNavigate();
  const [collapsed, setCollapsed] = useState(false);
  const [showNew, setShowNew] = useState(false);
  const [newPath, setNewPath] = useState('');
  const [newTitle, setNewTitle] = useState('');
  const [activeTab, setActiveTab] = useState<TabId>('pages');
  const [exporting, setExporting] = useState(false);

  const handleExport = async () => {
    setExporting(true);
    try {
      const dir = '/tmp/stratum-export';
      const result = await api.exportHtml(dir);
      alert(`Exported ${result.pages_exported} pages to ${dir}`);
    } catch (e) {
      alert(`Export failed: ${e}`);
    } finally {
      setExporting(false);
    }
  };

  const handleCreate = async () => {
    if (!newPath) return;
    await createPage(newPath, newTitle || undefined);
    setShowNew(false);
    setNewPath('');
    setNewTitle('');
  };

  const navigateTab = (tab: TabId, path: string) => {
    setActiveTab(tab);
    navigate(path);
  };

  const drawerWidth = collapsed ? DRAWER_COLLAPSED : DRAWER_WIDTH;

  return (
    <Drawer
      variant="permanent"
      sx={{
        width: drawerWidth,
        flexShrink: 0,
        transition: 'width 0.2s ease',
        '& .MuiDrawer-paper': {
          width: drawerWidth,
          transition: 'width 0.2s ease',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
          borderRight: 1,
          borderColor: 'divider',
          bgcolor: 'background.paper',
        },
      }}
    >
      {/* Header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          px: collapsed ? 1 : 2,
          py: 1.5,
          borderBottom: 1,
          borderColor: 'divider',
          minHeight: 56,
        }}
      >
            {!collapsed && (
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, flex: 1, minWidth: 0 }}>
                <Box sx={{ color: 'primary.main', width: 20, height: 20, display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}>
                  <StratumIcon />
                </Box>
                <Box sx={{ minWidth: 0, lineHeight: 1.2 }}>
                  <Typography variant="h6" noWrap sx={{ fontWeight: 700, fontSize: '1rem' }}>
                    stratum
                  </Typography>
                  {vault && (
                    <Typography variant="caption" color="text.secondary" noWrap sx={{ display: 'block', mt: 0.1 }}>
                      {vault.block_count}b · {vault.page_count}p
                    </Typography>
                  )}
                </Box>
              </Box>
            )}
        <IconButton
          size="small"
          onClick={() => setCollapsed(c => !c)}
          sx={{ color: 'text.secondary', flexShrink: 0 }}
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          {collapsed ? <ChevronRightIcon fontSize="small" /> : <ChevronLeftIcon fontSize="small" />}
        </IconButton>
      </Box>

      {/* Navigation */}
      <List dense sx={{ flex: 1, overflow: 'auto', py: 0.5 }}>
        {NAV_ITEMS.map(item => (
          <ListItemButton
            key={item.id}
            selected={activeTab === item.id}
            onClick={() => navigateTab(item.id, item.path)}
            sx={{
              minHeight: 40,
              justifyContent: collapsed ? 'center' : undefined,
              px: collapsed ? 1 : 2,
              borderRadius: collapsed ? 0 : '4px',
              mx: collapsed ? 0 : 0.5,
            }}
          >
            <ListItemIcon
              sx={{
                minWidth: collapsed ? 0 : 36,
                justifyContent: 'center',
                color: activeTab === item.id ? 'primary.main' : undefined,
              }}
            >
              {item.icon}
            </ListItemIcon>
            {!collapsed && <ListItemText primary={item.label} slotProps={{ primary: { variant: 'body2', noWrap: true } }} />}
          </ListItemButton>
        ))}

        {!collapsed && (
          <>
            <Divider sx={{ mx: 2, my: 1 }} />

            {/* Page list */}
            <Box sx={{ px: 1 }}>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1, py: 0.5 }}>
                <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', textTransform: 'uppercase' }}>
                  Pages
                </Typography>
                <Tooltip title="New page" arrow>
                  <IconButton size="small" onClick={() => setShowNew(!showNew)} sx={{ color: 'text.secondary' }}>
                    <AddIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
              </Box>

              {showNew && (
                <Box sx={{ p: 1, mb: 1, bgcolor: 'action.hover', borderRadius: 1 }}>
                  <TextField
                    size="small"
                    placeholder="Path (e.g., pages/my-note.md)"
                    value={newPath}
                    onChange={e => setNewPath(e.target.value)}
                    fullWidth
                    sx={{ mb: 0.5, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
                  />
                  <TextField
                    size="small"
                    placeholder="Title (optional)"
                    value={newTitle}
                    onChange={e => setNewTitle(e.target.value)}
                    fullWidth
                    sx={{ mb: 0.75, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
                  />
                  <Box sx={{ display: 'flex', gap: 0.5 }}>
                    <Button size="small" variant="contained" onClick={handleCreate}>
                      Create
                    </Button>
                    <Button size="small" onClick={() => setShowNew(false)}>
                      Cancel
                    </Button>
                  </Box>
                </Box>
              )}

              <List dense disablePadding>
                {pages.map(page => (
                  <ListItemButton
                    key={page.path}
                    dense
                    onClick={() => navigate(`/page/${encodeURIComponent(page.path)}`)}
                    sx={{ borderRadius: 1, '&:hover .delete-btn': { opacity: 1 } }}
                  >
                    <ListItemText
                      primary={page.title || page.slug}
                      slotProps={{
                        primary: { variant: 'body2', noWrap: true },
                        secondary: { variant: 'caption', color: 'text.disabled' as const },
                      }}
                      secondary={page.block_count ? `${page.block_count}b` : undefined}
                    />
                    <IconButton
                      size="small"
                      className="delete-btn"
                      sx={{ opacity: 0, color: 'error.main', ml: 0.5, p: 0.25, '&:hover': { bgcolor: 'error.light', color: 'error.contrastText' } }}
                      onClick={e => {
                        e.stopPropagation();
                        if (confirm(`Delete ${page.path}?`)) deletePage(page.path);
                      }}
                    >
                      <DeleteIcon fontSize="inherit" />
                    </IconButton>
                  </ListItemButton>
                ))}
                {pages.length === 0 && (
                  <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'center', py: 2 }}>
                    No pages yet.
                  </Typography>
                )}
              </List>
            </Box>
          </>
        )}
      </List>

      {/* Footer */}
      <Box
        sx={{
          borderTop: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          justifyContent: collapsed ? 'center' : 'space-between',
          flexDirection: collapsed ? 'column' : 'row',
          gap: collapsed ? 0.25 : 0,
          px: collapsed ? 0.5 : 1.5,
          py: collapsed ? 0.75 : 1,
        }}
      >
        {collapsed ? (
          <>
            <Tooltip title="Refresh" arrow>
              <IconButton size="small" onClick={() => loadPages()} sx={{ color: 'text.secondary' }}>
                <RefreshIcon fontSize="small" />
              </IconButton>
            </Tooltip>
            <Tooltip title="Export HTML" arrow>
              <span>
                <IconButton size="small" onClick={handleExport} disabled={exporting} sx={{ color: 'text.secondary' }}>
                  <FileUploadIcon fontSize="small" />
                </IconButton>
              </span>
            </Tooltip>
          </>
        ) : (
          <>
            <Button size="small" onClick={() => loadPages()} startIcon={<RefreshIcon />} sx={{ color: 'text.secondary', textTransform: 'none', fontSize: '0.75rem' }}>
              Refresh
            </Button>
            <Button size="small" onClick={handleExport} disabled={exporting} startIcon={<FileUploadIcon />} sx={{ color: 'text.secondary', textTransform: 'none', fontSize: '0.75rem' }}>
              {exporting ? '...' : 'Export'}
            </Button>
            <Typography variant="caption" color="text.disabled">
              v{__APP_VERSION__}
            </Typography>
          </>
        )}
      </Box>
    </Drawer>
  );
}
