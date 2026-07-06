import Dialog from '@mui/material/Dialog';
import type { DialogProps } from '@mui/material/Dialog';
import { useResponsive } from '../../lib/hooks/useResponsive';

type ResponsiveDialogProps = Omit<DialogProps, 'fullScreen'>;

export function ResponsiveDialog(props: ResponsiveDialogProps) {
  const { isMobile } = useResponsive();
  return <Dialog {...props} fullScreen={isMobile} />;
}
