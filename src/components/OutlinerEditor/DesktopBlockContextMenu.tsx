/**
 * Desktop right-click block context menu for the Outliner editor.
 *
 * Extracted verbatim from the `<Menu>` block in OutlinerEditor.desktop.tsx during
 * the t_dc58c24c sizing-gate decomposition. The read-aloud / set-change-clear
 * reminder branching stays INSIDE this component, driven by `menu.due`. Handlers
 * call `onReadAloud` / `onOpenReminder` / `onClearReminder` after `onClose()`.
 * Pure presentational move — no behaviour change.
 *
 * @module OutlinerEditor/DesktopBlockContextMenu
 */

import { useTranslation } from 'react-i18next';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import AlarmIcon from '@mui/icons-material/Alarm';
import AlarmOffIcon from '@mui/icons-material/AlarmOff';

export interface DesktopBlockMenuState {
  mouseX: number;
  mouseY: number;
  blockId: string;
  due: string | null;
}

export function DesktopBlockContextMenu({
  menu,
  onClose,
  onReadAloud,
  onOpenReminder,
  onClearReminder,
}: {
  menu: DesktopBlockMenuState | null;
  onClose: () => void;
  onReadAloud: (blockId: string) => void;
  onOpenReminder: (blockId: string) => void;
  onClearReminder: (blockId: string) => void;
}) {
  const { t } = useTranslation();
  const blockMenuDue = menu?.due ?? null;
  return (
    <Menu
      open={Boolean(menu)}
      onClose={onClose}
      anchorReference="anchorPosition"
      anchorPosition={
        menu ? { left: menu.mouseX, top: menu.mouseY } : undefined
      }
    >
      <MenuItem
        onClick={() => {
          onClose();
          if (menu) onReadAloud(menu.blockId);
        }}
      >
        <ListItemIcon><VolumeUpIcon fontSize="small" /></ListItemIcon>
        <ListItemText>{t('editor.read_aloud')}</ListItemText>
      </MenuItem>
      {blockMenuDue === null ? (
        <MenuItem
          onClick={() => {
            onClose();
            if (menu) onOpenReminder(menu.blockId);
          }}
        >
          <ListItemIcon><AlarmIcon fontSize="small" /></ListItemIcon>
          <ListItemText>{t('reminders.dialog.set_reminder')}</ListItemText>
        </MenuItem>
      ) : (
        <>
          <MenuItem
            onClick={() => {
              onClose();
              if (menu) onOpenReminder(menu.blockId);
            }}
          >
            <ListItemIcon><AlarmIcon fontSize="small" /></ListItemIcon>
            <ListItemText>{t('reminders.dialog.change_reminder')}</ListItemText>
          </MenuItem>
          <MenuItem
            onClick={() => {
              onClose();
              if (menu) onClearReminder(menu.blockId);
            }}
          >
            <ListItemIcon><AlarmOffIcon fontSize="small" /></ListItemIcon>
            <ListItemText>{t('reminders.dialog.clear_reminder')}</ListItemText>
          </MenuItem>
        </>
      )}
    </Menu>
  );
}
