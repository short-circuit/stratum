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
import SettingsIcon from '@mui/icons-material/Settings';

const PRIMARY_TABS = [
  { label: 'Journal', value: 'journal', path: '/journal', icon: <CalendarMonthIcon /> },
  { label: 'Search', value: 'search', path: '/search', icon: <SearchIcon /> },
  { label: 'Graph', value: 'graph', path: '/graph', icon: <HubIcon /> },
  { label: 'More', value: 'more', path: null, icon: <MoreHorizIcon /> },
] as const;

const MORE_ITEMS = [
  { label: 'Kanban', path: '/kanban', icon: <ChecklistIcon /> },
  { label: 'Query', path: '/query', icon: <CodeIcon /> },
  { label: 'Templates', path: '/templates', icon: <DescriptionIcon /> },
  { label: 'Flashcards', path: '/flashcards', icon: <QuizIcon /> },
  { label: 'Whiteboards', path: '/whiteboards', icon: <DrawIcon /> },
  { label: 'Settings', path: '/settings', icon: <SettingsIcon /> },
] as const;

export default function MobileNav() {
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
    <Paper sx={{ position: 'fixed', bottom: 0, left: 0, right: 0, zIndex: 1200 }} elevation={3}>
      <BottomNavigation
        value={activeValue}
        onChange={handleTabChange}
        showLabels
      >
        {PRIMARY_TABS.map(tab => (
          <BottomNavigationAction
            key={tab.value}
            label={tab.label}
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
            <ListItemText>{item.label}</ListItemText>
          </MenuItem>
        ))}
      </Menu>
    </Paper>
  );
}
