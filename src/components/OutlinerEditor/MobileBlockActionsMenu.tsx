/**
 * Mobile long-press block actions menu for the Outliner editor.
 *
 * Extracted verbatim from the context `<Menu>` block in MobileEditorOverlays.tsx
 * during the t_dc58c24c sizing-gate decomposition. Pure presentational move — no
 * behaviour change.
 *
 * @module OutlinerEditor/MobileBlockActionsMenu
 */

import { useTranslation } from 'react-i18next';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import ContentCutIcon from '@mui/icons-material/ContentCut';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import DeleteIcon from '@mui/icons-material/Delete';
import VolumeUpIcon from '@mui/icons-material/VolumeUp';

export function MobileBlockActionsMenu({
  menu,
  onClose,
  onAction,
}: {
  menu: { x: number; y: number; blockId: string } | null;
  onClose: () => void;
  onAction: (action: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <Menu
      open={Boolean(menu)}
      onClose={onClose}
      anchorReference="anchorPosition"
      anchorPosition={
        menu ? { left: menu.x, top: menu.y } : undefined
      }
    >
      <MenuItem onClick={() => onAction('cut')}>
        <ListItemIcon><ContentCutIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.cut')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={() => onAction('copy')}>
        <ListItemIcon><ContentCopyIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.copy')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={() => onAction('speak')}>
        <ListItemIcon><VolumeUpIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.read_aloud')}</ListItemText>
      </MenuItem>
      <MenuItem onClick={() => onAction('delete')}>
        <ListItemIcon><DeleteIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.delete')}</ListItemText>
      </MenuItem>
    </Menu>
  );
}
