/**
 * Floating save-status chip for the Outliner editor.
 *
 * Extracted verbatim from OutlinerEditor.desktop.tsx / OutlinerEditor.mobile.tsx
 * during the t_dc58c24c sizing-gate decomposition. The parent keeps computing the
 * `label` (the useMemo) — only the JSX moves here. Pure presentational move — no
 * behaviour change.
 *
 * @module OutlinerEditor/SaveStatusChip
 */

import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';

export function SaveStatusChip({
  label,
  saving,
  right = 12,
}: {
  label: string;
  saving: boolean;
  /** Right offset in px — row mode reserves the top-right action region (56). */
  right?: number;
}) {
  return (
    <Box
      sx={{
        position: 'absolute',
        top: 8,
        right,
        zIndex: 5,
        px: 1,
        py: 0.25,
        borderRadius: 1,
        bgcolor: 'background.paper',
        border: '1px solid',
        borderColor: 'divider',
        boxShadow: 1,
        pointerEvents: 'none',
      }}
    >
      <Typography variant="caption" color={saving ? 'text.secondary' : 'text.disabled'}>
        {label}
      </Typography>
    </Box>
  );
}
