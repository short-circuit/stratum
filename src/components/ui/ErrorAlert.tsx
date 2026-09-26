import Alert from '@mui/material/Alert';
import type { SxProps, Theme } from '@mui/material/styles';

interface Props {
  message: string | null;
  onClose?: () => void;
  /** Optional layout/style overrides merged over the base banner styling. */
  sx?: SxProps<Theme>;
}

/** Local guard so a null/empty message can never render a visible banner. */
function hasMessage(message: string | null): message is string {
  return typeof message === 'string' && message.length > 0;
}

export default function ErrorAlert({ message, onClose, sx }: Props) {
  if (!hasMessage(message)) return null;
  return (
    <Alert severity="error" onClose={onClose} sx={{ borderRadius: 0, ...sx }}>
      {message}
    </Alert>
  );
}
