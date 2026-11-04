/**
 * "Create page" popover for unresolved [[wiki-links]] in the Outliner editor.
 *
 * Extracted verbatim from OutlinerEditor.desktop.tsx / OutlinerEditor.mobile.tsx
 * during the t_dc58c24c sizing-gate decomposition. The original `setDeadLinkPopup(null)`
 * calls inside the click handler become `onClose()` (the parent passes
 * `() => setDeadLinkPopup(null)`) — identical semantics. Pure presentational move —
 * no behaviour change.
 *
 * @module OutlinerEditor/DeadLinkCreatePopup
 */

import { useTranslation } from 'react-i18next';
import type { RefObject } from 'react';
import Popover from '@mui/material/Popover';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';
import AddCircleIcon from '@mui/icons-material/AddCircle';
import * as api from '../../lib/commands';
import { useRecentsStore } from '../../stores/recentsStore';
import type { DeadLinkPopupState } from './useEditorData';

export function DeadLinkCreatePopup({
  popup,
  onClose,
  navigateRef,
}: {
  popup: DeadLinkPopupState;
  onClose: () => void;
  navigateRef: RefObject<(path: string) => void>;
}) {
  const { t } = useTranslation();
  return (
    <Popover
      open={Boolean(popup)}
      anchorReference="anchorPosition"
      anchorPosition={
        popup
          ? {
              left: popup.position.x,
              top: popup.position.y,
            }
          : undefined
      }
      onClose={onClose}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      transformOrigin={{ vertical: 'top', horizontal: 'left' }}
      disableScrollLock
      slotProps={{ paper: { sx: { p: 0.5 } } }}
    >
      <Tooltip title={t('editor.create_page')}>
        <IconButton
          size="small"
          color="primary"
          onClick={async () => {
            if (!popup) return;
            const slug = popup.target;
            try {
              await api.createPage(slug);
              useRecentsStore.getState().refresh();
            } catch (e) {
              if (!String(e).includes(t('editor.already_exists'))) {
                console.error(t('editor.failed_to_create_page'), e);
                onClose();
                return;
              }
            }
            navigateRef.current('/page/' + encodeURIComponent(slug));
            onClose();
          }}
        >
          <AddCircleIcon fontSize="small" />
        </IconButton>
      </Tooltip>
    </Popover>
  );
}
