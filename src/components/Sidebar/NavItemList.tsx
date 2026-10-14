import { memo } from 'react';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonth';
import HubIcon from '@mui/icons-material/Hub';
import SearchIcon from '@mui/icons-material/Search';
import CodeIcon from '@mui/icons-material/Code';
import DescriptionIcon from '@mui/icons-material/Description';
import QuizIcon from '@mui/icons-material/Quiz';
import ChecklistIcon from '@mui/icons-material/Checklist';
import DrawIcon from '@mui/icons-material/Draw';
import AutoAwesomeIcon from '@mui/icons-material/AutoAwesome';
import ExtensionIcon from '@mui/icons-material/Extension';
import AccessAlarmIcon from '@mui/icons-material/AccessAlarm';
import SettingsIcon from '@mui/icons-material/Settings';
import { useTranslation } from 'react-i18next';

const NAV_ITEMS = [
  { id: 'journal', label: 'mobile_nav.journal', path: '/journal', icon: <CalendarMonthIcon /> },
  { id: 'graph', label: 'mobile_nav.graph', path: '/graph', icon: <HubIcon /> },
  { id: 'kanban', label: 'mobile_nav.kanban', path: '/kanban', icon: <ChecklistIcon /> },
  { id: 'reminders', label: 'mobile_nav.reminders', path: '/reminders', icon: <AccessAlarmIcon /> },
  { id: 'search', label: 'mobile_nav.search', path: '/search', icon: <SearchIcon /> },
  { id: 'query', label: 'mobile_nav.query', path: '/query', icon: <CodeIcon /> },
  { id: 'asknotes', label: 'mobile_nav.ask_notes', path: '/ask-notes', icon: <AutoAwesomeIcon /> },
  { id: 'templates', label: 'mobile_nav.templates', path: '/templates', icon: <DescriptionIcon /> },
  { id: 'flashcards', label: 'mobile_nav.flashcards', path: '/flashcards', icon: <QuizIcon /> },
  { id: 'whiteboards', label: 'mobile_nav.whiteboards', path: '/whiteboards', icon: <DrawIcon /> },
  { id: 'plugins', label: 'mobile_nav.plugins', path: '/plugins', icon: <ExtensionIcon /> },
  { id: 'settings', label: 'mobile_nav.settings', path: '/settings', icon: <SettingsIcon /> },
] as const;

type TabId = (typeof NAV_ITEMS)[number]['id'];

interface Props {
  collapsed: boolean;
  activeTab: TabId;
  onNavigate: (tab: TabId, path: string) => void;
}

const NavItemList = memo(function NavItemList({ collapsed, activeTab, onNavigate }: Props) {
  const { t } = useTranslation();
  return (
    <>
      {NAV_ITEMS.map(item => (
        <ListItemButton
          key={item.id}
          selected={activeTab === item.id}
          onClick={() => onNavigate(item.id, item.path)}
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
          {!collapsed && (
            <ListItemText primary={t(item.label)} slotProps={{ primary: { variant: 'body2', noWrap: true } }} />
          )}
        </ListItemButton>
      ))}
    </>
  );
});

export default NavItemList;
export { NAV_ITEMS };
export type { TabId };
