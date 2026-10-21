//! Shared settings row.
//!
//! Renders a labeled row with an optional control. When `checked` and
//! `onCheckedChange` are provided it renders a toggle switch (FormControlLabel);
//! otherwise it renders a generic row with an arbitrary `control` on the right.
//! Used for desktop/mobile parity across settings sections.

import type { ReactNode } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import type { SxProps, Theme } from '@mui/material';

interface SettingsRowProps {
  label: string;
  description?: string;
  control?: ReactNode;
  checked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  stacked?: boolean;
  sx?: SxProps<Theme>;
}

export default function SettingsRow({
  label,
  description,
  control,
  checked,
  onCheckedChange,
  stacked = false,
  sx,
}: SettingsRowProps) {
  if (checked !== undefined && onCheckedChange) {
    return (
      <FormControlLabel
        control={<Switch checked={checked} onChange={e => onCheckedChange(e.target.checked)} />}
        label={label}
        sx={sx}
      />
    );
  }
  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: stacked ? 'column' : 'row',
        justifyContent: 'space-between',
        alignItems: 'center',
        gap: 1,
        ...sx,
      }}
    >
      <Box>
        <Typography variant="body2">{label}</Typography>
        {description && (
          <Typography variant="caption" color="text.secondary">
            {description}
          </Typography>
        )}
      </Box>
      <Box sx={{ flexShrink: 0, ...(stacked ? { width: '100%' } : {}) }}>{control}</Box>
    </Box>
  );
}
