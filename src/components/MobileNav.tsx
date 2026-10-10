import { useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import Paper from '@mui/material/Paper';
import BottomNavigation from '@mui/material/BottomNavigation';
import BottomNavigationAction from '@mui/material/BottomNavigationAction';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonth';
import HubIcon from '@mui/icons-material/Hub';
import SearchIcon from '@mui/icons-material/Search';
import MoreHorizIcon from '@mui/icons-material/MoreHoriz';
import ChecklistIcon from '@mui/icons-material/Checklist';
import CodeIcon from '@mui/icons-material/Code';
import DescriptionIcon from '@mui/icons-material/Description';
import QuizIcon from '@mui/icons-material/Quiz';
import DrawIcon from '@mui/icons-material/Draw';
import AutoAwesomeIcon from '@mui/icons-material/AutoAwesome';
import ExtensionIcon from '@mui/icons-material/Extension';
import SettingsIcon from '@mui/icons-material/Settings';
import { useTranslation } from 'react-i18next';

const PRIMARY_TABS = [
  { label: 'mobile_nav.journal', value: 'journal', path: '/journal', icon: <CalendarMonthIcon /> },
  { label: 'mobile_nav.search', value: 'search', path: '/search', icon: <SearchIcon /> },
  { label: 'mobile_nav.graph', value: 'graph', path: '/graph', icon: <HubIcon /> },
  { label: 'mobile_nav.more', value: 'more', path: null, icon: <MoreHorizIcon /> },
] as const;

const MORE_ITEMS = [
  { label: 'mobile_nav.kanban', path: '/kanban', icon: <ChecklistIcon /> },
  { label: 'mobile_nav.query', path: '/query', icon: <CodeIcon /> },
  { label: 'mobile_nav.ask_notes', path: '/ask-notes', icon: <AutoAwesomeIcon /> },
  { label: 'mobile_nav.templates', path: '/templates', icon: <DescriptionIcon /> },
  { label: 'mobile_nav.flashcards', path: '/flashcards', icon: <QuizIcon /> },
  { label: 'mobile_nav.whiteboards', path: '/whiteboards', icon: <DrawIcon /> },
  { label: 'mobile_nav.plugins', path: '/plugins', icon: <ExtensionIcon /> },
  { label: 'mobile_nav.settings', path: '/settings', icon: <SettingsIcon /> },
] as const;

export default function MobileNav() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const [moreAnchor, setMoreAnchor] = useState<HTMLElement | null>(null);

  const currentPath = location.pathname;
  const getActiveValue = () => {
    if (currentPath.startsWith('/journal')) return 'journal';
    if (currentPath.startsWith('/search')) return 'search';
    if (currentPath.startsWith('/graph')) return 'graph';
    return 'more';
  };
  const activeValue = getActiveValue();

  const handleTabChange = (_: React.SyntheticEvent, value: string) => {
    if (value === 'more') return;
    const tab = PRIMARY_TABS.find(t => t.value === value);
    if (tab?.path) navigate(tab.path);
  };

  const handleMoreItem = (path: string) => {
    setMoreAnchor(null);
    navigate(path);
  };

  return (
    <Paper style={{ position: 'fixed', bottom: 0, left: 0, right: 0, zIndex: 1200, paddingBottom: 'var(--safe-area-bottom)' }} elevation={3}>
      <BottomNavigation
        value={activeValue}
        onChange={handleTabChange}
        showLabels
      >
        {PRIMARY_TABS.map(tab => (
          <BottomNavigationAction
            key={tab.value}
            label={t(tab.label)}
            value={tab.value}
            icon={tab.icon}
            onClick={tab.value === 'more' ? (e) => setMoreAnchor(e.currentTarget) : undefined}
          />
        ))}
      </BottomNavigation>

      <Menu
        anchorEl={moreAnchor}
        open={Boolean(moreAnchor)}
        onClose={() => setMoreAnchor(null)}
        anchorOrigin={{ vertical: 'top', horizontal: 'center' }}
        transformOrigin={{ vertical: 'bottom', horizontal: 'center' }}
        slotProps={{
          paper: {
            sx: { width: 200, maxHeight: 300 },
          },
        }}
      >
        {MORE_ITEMS.map(item => (
          <MenuItem key={item.label} onClick={() => handleMoreItem(item.path)} dense>
            <ListItemIcon sx={{ minWidth: 36 }}>{item.icon}</ListItemIcon>
            <ListItemText>{t(item.label)}</ListItemText>
          </MenuItem>
        ))}
      </Menu>
    </Paper>
  );
}
