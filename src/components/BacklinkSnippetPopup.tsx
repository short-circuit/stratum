import { useEffect, useRef } from 'react';
import Popover from '@mui/material/Popover';
import { useNavigate } from 'react-router-dom';
import BacklinkSnippetBody from './BacklinkSnippetBody';

/**
 * Backlink snippet popover (desktop).
 *
 * Shown when the user Ctrl+hovers a backlink (or a backlink with an anchor).
 * Positioned via MUI `anchorPosition` at the link's screen coordinates; MUI's
 * Popover clamps the placement to the viewport (marginThreshold = 16) so the
 * popup never renders off-screen near a viewport edge, and re-clamps on
 * window scroll/resize.
 *
 * Accessibility:
 *  - role="dialog" with an aria-label naming the backlinked note, and
 *    aria-busy while the snippet loads.
 *  - Dismisses on Escape (MUI Popover onClose), on Ctrl/Meta release, and on
 *    mouse leave, so keyboard-only users are not trapped in an open popup.
 *
 * The snippet is fetched by the caller (useEditorData / BacklinksPanel shared
 * logic); this component is a pure view over that state.
 */

export interface BacklinkSnippetPopupProps {
  noteId: string;
  noteTitle: string;
  context: string[];
  anchorContent: string;
  position: { x: number; y: number };
  loading: boolean;
  error: boolean;
  onClose: () => void;
  /** Reports whether the pointer is currently over the popup (hover guard). */
  onHoverChange?: (hovered: boolean) => void;
}

export default function BacklinkSnippetPopup({
  noteId,
  noteTitle,
  context,
  anchorContent,
  position,
  loading,
  error,
  onClose,
  onHoverChange,
}: BacklinkSnippetPopupProps) {
  const navigate = useNavigate();
  const ctrlHeldRef = useRef(false);

  // Track Ctrl/Meta held state so releasing the modifier (without clicking)
  // dismisses the popup.
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') ctrlHeldRef.current = true;
    };
    const up = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') {
        ctrlHeldRef.current = false;
        onClose();
      }
    };
    const blur = () => {
      ctrlHeldRef.current = false;
    };
    window.addEventListener('keydown', down);
    window.addEventListener('keyup', up);
    window.addEventListener('blur', blur);
    return () => {
      window.removeEventListener('keydown', down);
      window.removeEventListener('keyup', up);
      window.removeEventListener('blur', blur);
    };
  }, [onClose]);

  return (
    <Popover
      open
      anchorReference="anchorPosition"
      anchorPosition={{ left: position.x, top: position.y }}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      transformOrigin={{ vertical: 'top', horizontal: 'left' }}
      onClose={onClose}
      slotProps={{
        paper: {
          role: 'dialog',
          'aria-label': `Snippet from: ${noteTitle || noteId || 'note'}`,
          'aria-busy': loading,
          onMouseEnter: () => onHoverChange?.(true),
          onMouseLeave: () => onHoverChange?.(false),
          sx: { maxWidth: 340, p: 0, borderRadius: 1.5 },
        },
      }}
    >
      <BacklinkSnippetBody
        noteTitle={noteTitle}
        noteId={noteId}
        context={context}
        anchorContent={anchorContent}
        loading={loading}
        error={error}
        onNavigate={() => {
          onClose();
          if (noteId) navigate(`/page/${encodeURIComponent(noteId)}`);
        }}
        maxHeight={240}
      />
    </Popover>
  );
}
