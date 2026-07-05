import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import type { ReactNode } from 'react';

interface Action {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  variant?: 'text' | 'outlined' | 'contained';
  color?: 'primary' | 'secondary' | 'error' | 'inherit';
}

interface Props {
  title: string;
  subtitle?: string;
  actions?: Action[];
  onBack?: () => void;
  children?: ReactNode;
}

export default function PageHeader({ title, subtitle, actions, onBack, children }: Props) {
  return (
    <Box sx={{
      px: 2, py: 1.5, borderBottom: 1, borderColor: 'divider',
      display: 'flex', alignItems: 'center', gap: 1.5, flexShrink: 0,
    }}>
      {onBack && (
        <IconButton size="small" onClick={onBack} sx={{ color: 'text.secondary' }}>
          <ArrowBackIcon fontSize="small" />
        </IconButton>
      )}
      <Box sx={{ minWidth: 0 }}>
        <Typography variant="h6" sx={{ fontWeight: 700, fontSize: '1.1rem', lineHeight: 1.2 }}>
          {title}
        </Typography>
        {subtitle && (
          <Typography variant="caption" color="text.secondary" noWrap sx={{ display: 'block' }}>
            {subtitle}
          </Typography>
        )}
      </Box>
      <Box sx={{ flex: 1 }} />
      {actions?.map((a, i) => (
        <Button key={i} size="small" variant={a.variant || 'text'} onClick={a.onClick} disabled={a.disabled} sx={{ textTransform: 'none', fontSize: '0.75rem' }}>
          {a.label}
        </Button>
      ))}
      {children}
    </Box>
  );
}
