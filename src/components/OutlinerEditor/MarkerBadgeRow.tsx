/**
 * Page marker badge strip for the Outliner editor.
 *
 * Extracted verbatim from OutlinerEditor.desktop.tsx / OutlinerEditor.mobile.tsx
 * during the t_dc58c24c sizing-gate decomposition. Renders the list of task
 * markers present on a page. `horizontalPadding` defaults to 2 (desktop); the
 * mobile variant passes 1 to keep the rendered DOM/sx identical to its original
 * call site. Pure presentational move — no behaviour change.
 *
 * @module OutlinerEditor/MarkerBadgeRow
 */

import Box from '@mui/material/Box';
import MarkerBadge from '../MarkerBadge';

export function MarkerBadgeRow({
  markers,
  horizontalPadding = 2,
}: {
  markers: string[];
  horizontalPadding?: number;
}) {
  return (
    <Box
      sx={{
        px: horizontalPadding,
        py: 0.5,
        display: 'flex',
        gap: 0.5,
        flexWrap: 'wrap',
        borderBottom: 1,
        borderColor: 'divider',
        bgcolor: 'action.hover',
      }}
    >
      {markers.map((m) => (
        <MarkerBadge key={m} marker={m} />
      ))}
    </Box>
  );
}
