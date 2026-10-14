import Snackbar from '@mui/material/Snackbar';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useStore } from '../../stores/appStore';
import { useReminders } from '../../lib/reminders/useReminders';

/** Global in-app toast shown when a reminder becomes due. Mounted once at app root. */
export default function ReminderToaster() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const setHighlightBlock = useStore((s) => s.setHighlightBlock);
  const { toast, dismissToast } = useReminders();

  if (!toast) return null;

  const open = () => {
    setHighlightBlock({ pagePath: toast.pagePath, blockId: toast.blockId });
    navigate(`/page/${encodeURIComponent(toast.pagePath)}`);
    dismissToast();
  };

  return (
    <Snackbar
      open
      autoHideDuration={8000}
      onClose={dismissToast}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
    >
      <Alert
        severity="info"
        variant="filled"
        onClose={dismissToast}
        action={
          <Button color="inherit" size="small" onClick={open}>
            {t('reminders.toast.open')}
          </Button>
        }
      >
        <strong>{toast.title}</strong> — {toast.message}
      </Alert>
    </Snackbar>
  );
}
