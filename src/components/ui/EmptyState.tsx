import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import type { ReactNode } from 'react';

interface Props {
  icon?: ReactNode;
  message: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
}

export default function EmptyState({ icon, message, description, actionLabel, onAction }: Props) {
  return (
    <Box sx={{ textAlign: 'center', py: 6, px: 2 }}>
      {icon && <Box sx={{ mb: 1.5, color: 'text.disabled' }}>{icon}</Box>}
      <Typography variant="body2" color="text.secondary" sx={{ mb: description ? 0.5 : 0 }}>
        {message}
      </Typography>
      {description && (
        <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mb: 1.5 }}>
          {description}
        </Typography>
      )}
      {actionLabel && onAction && (
        <Button variant="outlined" size="small" onClick={onAction}>
          {actionLabel}
        </Button>
      )}
    </Box>
  );
}
