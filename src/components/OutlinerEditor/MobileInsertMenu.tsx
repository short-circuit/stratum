/**
 * Mobile floating insert menu for the Outliner editor.
 *
 * Extracted verbatim from the insert `<Menu>` block in MobileEditorOverlays.tsx
 * during the t_dc58c24c sizing-gate decomposition. Pure presentational move — no
 * behaviour change.
 *
 * @module OutlinerEditor/MobileInsertMenu
 */

import { useTranslation } from 'react-i18next';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import TextFieldsIcon from '@mui/icons-material/TextFields';
import LooksOneIcon from '@mui/icons-material/LooksOne';
import LooksTwoIcon from '@mui/icons-material/LooksTwo';
import Looks3Icon from '@mui/icons-material/Looks3';
import FunctionsIcon from '@mui/icons-material/Functions';
import CodeIcon from '@mui/icons-material/Code';
import DiagramIcon from '@mui/icons-material/Schema';

export function MobileInsertMenu({
  anchor,
  onClose,
  onInsert,
  onInsertMath,
}: {
  anchor: HTMLElement | null;
  onClose: () => void;
  onInsert: (type: string, level?: number) => void;
  onInsertMath: () => void;
}) {
  const { t } = useTranslation();
  return (
    <Menu
      open={Boolean(anchor)}
      anchorEl={anchor}
      onClose={onClose}
      anchorOrigin={{ vertical: 'top', horizontal: 'left' }}
      transformOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      slotProps={{ paper: { sx: { minWidth: 180 } } }}
    >
      <MenuItem onClick={() => onInsert('paragraph')}>
        <ListItemIcon><TextFieldsIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.paragraph')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={() => onInsert('heading', 1)}>
        <ListItemIcon><LooksOneIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.heading_1')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={() => onInsert('heading', 2)}>
        <ListItemIcon><LooksTwoIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.heading_2')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={() => onInsert('heading', 3)}>
        <ListItemIcon><Looks3Icon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.heading_3')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={() => onInsert('codeBlock')}>
        <ListItemIcon><CodeIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.code_block')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={() => onInsert('mermaid')}>
        <ListItemIcon><DiagramIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.mermaid_diagram')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={onInsertMath}>
        <ListItemIcon><FunctionsIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.math_equation')}</ListItemText>
      </MenuItem>
    </Menu>
  );
}
