import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Drawer from '@mui/material/Drawer';
import List from '@mui/material/List';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import { useStore } from '../../stores/appStore';
import * as api from '../../lib/commands';
import StratumIcon from '../StratumIcon';
import NavItemList, { type TabId } from './NavItemList';
import PageTree from './PageTree';
import SidebarFooter from './SidebarFooter';
import { useResponsive } from '../../lib/hooks/useResponsive';

const DRAWER_WIDTH = 224;
const DRAWER_COLLAPSED = 52;

export default function Sidebar() {
  const { pages, vault, loadPages, createPage, deletePage } = useStore();
  const navigate = useNavigate();
  const [collapsed, setCollapsed] = useState(false);
  const [showNew, setShowNew] = useState(false);
  const [newPath, setNewPath] = useState('');
  const [newTitle, setNewTitle] = useState('');
  const [activeTab, setActiveTab] = useState<TabId>('journal');
  const [exporting, setExporting] = useState(false);

  const { isMobile } = useResponsive();
  if (isMobile) return null;

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
            <StratumIcon style={{ width: 48, height: 48, color: 'var(--primary-500)', flexShrink: 0 }} />
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

      <Box sx={{ overflow: 'auto', flex: '0 0 auto' }}>
        <List dense sx={{ py: 0.5 }}>
          <NavItemList collapsed={collapsed} activeTab={activeTab} onNavigate={navigateTab} />
        </List>
      </Box>

      <Box sx={{ borderTop: 1, borderColor: 'divider', mx: 2 }} />

      <Box sx={{ overflow: 'auto', flex: 1 }}>
        <PageTree
          pages={pages}
          collapsed={collapsed}
          showNew={showNew}
          newPath={newPath}
          newTitle={newTitle}
          onShowNewChange={setShowNew}
          onNewPathChange={setNewPath}
          onNewTitleChange={setNewTitle}
          onCreatePage={handleCreate}
          onDeletePage={path => deletePage(path)}
          onNavigate={navigate}
          onNavigateHome={() => navigate('/')}
        />
      </Box>

      <SidebarFooter collapsed={collapsed} exporting={exporting} onRefresh={loadPages} onExport={handleExport} />
    </Drawer>
  );
}

export { DRAWER_WIDTH, DRAWER_COLLAPSED };
