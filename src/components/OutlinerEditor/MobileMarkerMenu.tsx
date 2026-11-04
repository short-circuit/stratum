/**
 * Mobile floating marker menu for the Outliner editor.
 *
 * Extracted verbatim from the marker `<Menu>` block and the MARKER_OPTIONS const
 * in MobileEditorOverlays.tsx during the t_dc58c24c sizing-gate decomposition.
 * Pure presentational move — no behaviour change.
 *
 * @module OutlinerEditor/MobileMarkerMenu
 */

import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import MarkerBadge from '../MarkerBadge';

const MARKER_OPTIONS = ['TODO', 'DOING', 'DONE', 'NOW', 'LATER', 'WAITING', 'CANCELLED'];

export function MobileMarkerMenu({
  anchor,
  onClose,
  onApplyMarker,
}: {
  anchor: HTMLElement | null;
  onClose: () => void;
  onApplyMarker: (marker: string) => void;
}) {
  return (
    <Menu
      open={Boolean(anchor)}
      anchorEl={anchor}
      onClose={onClose}
      anchorOrigin={{ vertical: 'top', horizontal: 'left' }}
      transformOrigin={{ vertical: 'bottom', horizontal: 'left' }}
    >
      {MARKER_OPTIONS.map((m) => (
        <MenuItem key={m} onClick={() => onApplyMarker(m)}>
          <MarkerBadge marker={m} />
        </MenuItem>
      ))}
    </Menu>
  );
}
