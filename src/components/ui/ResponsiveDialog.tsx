import Dialog from '@mui/material/Dialog';
import type { DialogProps } from '@mui/material/Dialog';
import { useResponsive } from '../../lib/hooks/useResponsive';

interface ResponsiveDialogProps extends Omit<DialogProps, 'fullScreen'> {
  // All DialogProps except fullScreen (which is auto-managed based on isMobile)
}

export function ResponsiveDialog(props: ResponsiveDialogProps) {
  const { isMobile } = useResponsive();
  return <Dialog {...props} fullScreen={isMobile} />;
}
