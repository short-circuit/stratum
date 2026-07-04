import Chip from '@mui/material/Chip';

const MARKER_COLORS: Record<string, { bg: string; text: string }> = {
  TODO:       { bg: '#f59e0b', text: '#fff' },
  DOING:      { bg: '#3b82f6', text: '#fff' },
  DONE:       { bg: '#10b981', text: '#fff' },
  NOW:        { bg: '#8b5cf6', text: '#fff' },
  LATER:      { bg: '#f97316', text: '#fff' },
  WAITING:    { bg: '#ec4899', text: '#fff' },
  CANCELLED:  { bg: '#6b7280', text: '#fff' },
};

interface Props {
  marker: string | null;
}

export default function MarkerBadge({ marker }: Props) {
  if (!marker) return null;
  const normalized = marker.toUpperCase();
  const colors = MARKER_COLORS[normalized] ?? { bg: '#6b7280', text: '#fff' };
  return (
    <Chip
      label={marker}
      size="small"
      sx={{
        height: 20,
        fontSize: '0.6rem',
        fontWeight: 700,
        bgcolor: colors.bg,
        color: colors.text,
        letterSpacing: '0.02em',
      }}
    />
  );
}
