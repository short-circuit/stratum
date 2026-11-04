/**
 * Shared menu content for the mobile editor's action menus.
 *
 * Both the floating FAB menus (MobileEditorOverlays, full-page editor) and the
 * journal row-actions menu render the SAME item lists through these
 * components, so the insert/marker options never drift between the two
 * surfaces (AGENTS.md: no logic duplicated).
 */

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
import { useTranslation } from 'react-i18next';
import MarkerBadge from '../MarkerBadge';

export interface InsertMenuItemsProps {
  /** Insert a block of the given type (optionally heading level). */
  onInsert: (type: string, level?: number) => void;
  /** Open the full-screen math editor modal. */
  onInsertMath: () => void;
}

/** Insert options: block types + math equation. */
export function InsertMenuItems({ onInsert, onInsertMath }: InsertMenuItemsProps) {
  const { t } = useTranslation();
  return (
    <>
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
    </>
  );
}

export interface MarkerMenuItemsProps {
  /** Marker values to offer, in display order (single source: MARKER_OPTIONS). */
  markers: string[];
  /** Apply a marker to the current selection. */
  onApplyMarker: (marker: string) => void;
}

/** Marker options for the current selection. */
export function MarkerMenuItems({ markers, onApplyMarker }: MarkerMenuItemsProps) {
  return (
    <>
      {markers.map((m) => (
        <MenuItem key={m} onClick={() => onApplyMarker(m)}>
          <MarkerBadge marker={m} />
        </MenuItem>
      ))}
    </>
  );
}
