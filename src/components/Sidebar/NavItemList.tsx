import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonth';
import ArticleIcon from '@mui/icons-material/Article';
import HubIcon from '@mui/icons-material/Hub';
import SearchIcon from '@mui/icons-material/Search';
import CodeIcon from '@mui/icons-material/Code';
import DescriptionIcon from '@mui/icons-material/Description';
import QuizIcon from '@mui/icons-material/Quiz';
import ChecklistIcon from '@mui/icons-material/Checklist';
import DrawIcon from '@mui/icons-material/Draw';
import SettingsIcon from '@mui/icons-material/Settings';

const NAV_ITEMS = [
  { id: 'journal', label: 'Journal', path: '/journal', icon: <CalendarMonthIcon /> },
  { id: 'pages', label: 'Pages', path: '/' as const, icon: <ArticleIcon /> },
  { id: 'graph', label: 'Graph', path: '/graph', icon: <HubIcon /> },
  { id: 'kanban', label: 'Kanban', path: '/kanban', icon: <ChecklistIcon /> },
  { id: 'search', label: 'Search', path: '/search', icon: <SearchIcon /> },
  { id: 'query', label: 'Query', path: '/query', icon: <CodeIcon /> },
  { id: 'templates', label: 'Templates', path: '/templates', icon: <DescriptionIcon /> },
  { id: 'flashcards', label: 'Flashcards', path: '/flashcards', icon: <QuizIcon /> },
  { id: 'whiteboards', label: 'Whiteboards', path: '/whiteboards', icon: <DrawIcon /> },
  { id: 'settings', label: 'Settings', path: '/settings', icon: <SettingsIcon /> },
] as const;

type TabId = (typeof NAV_ITEMS)[number]['id'];

interface Props {
  collapsed: boolean;
  activeTab: TabId;
  onNavigate: (tab: TabId, path: string) => void;
}

export default function NavItemList({ collapsed, activeTab, onNavigate }: Props) {
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
            <ListItemText primary={item.label} slotProps={{ primary: { variant: 'body2', noWrap: true } }} />
          )}
        </ListItemButton>
      ))}
    </>
  );
}

export { NAV_ITEMS };
export type { TabId };
