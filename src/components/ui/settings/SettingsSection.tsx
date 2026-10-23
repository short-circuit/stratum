//! Shared settings section wrapper.
//!
//! Renders a consistent section header (title, optional description, optional
//! divider) with a variant-aware layout so the desktop SettingsPage tabs and the
//! mobile settings sections produce structurally identical headers. The
//! `variant` controls the layout: `mobile` adds horizontal padding and uses a
//! bolder subtitle; `desktop` renders with no padding and a muted subtitle.

import type { ReactNode } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import type { SxProps, Theme } from '@mui/material';

interface SettingsSectionProps {
  title?: string;
  description?: string;
  children?: ReactNode;
  divider?: boolean;
  variant?: 'desktop' | 'mobile';
  sx?: SxProps<Theme>;
}

export default function SettingsSection({
  title,
  description,
  children,
  divider = false,
  variant = 'desktop',
  sx,
}: SettingsSectionProps) {
  if (variant === 'mobile') {
    return (
      <Box sx={{ px: 2, pt: 3, ...sx }}>
        {title && (
          <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
            {title}
          </Typography>
        )}
        {description && (
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
            {description}
          </Typography>
        )}
        {divider && <Divider sx={{ mb: 1.5 }} />}
        {children}
      </Box>
    );
  }
  return (
    <Box sx={sx}>
      {title && (
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
          {title}
        </Typography>
      )}
      {description && (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
          {description}
        </Typography>
      )}
      {divider && <Divider sx={{ mb: 1.5 }} />}
      {children}
    </Box>
  );
}
