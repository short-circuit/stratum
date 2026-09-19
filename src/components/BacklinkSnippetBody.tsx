import Typography from '@mui/material/Typography';
import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';

/**
 * Shared body for the backlink snippet popup.
 *
 * Renders the note title (clickable to navigate) and the snippet context
 * window with the exact backlinked block highlighted. Handles the loading and
 * error states so neither the Popover (desktop) nor the Dialog (mobile) ever
 * renders an empty or stuck frame.
 *
 * The highlighted portion is the anchor block — the exact content the backlink
 * points at. It is matched by string equality against `anchorContent`, so the
 * surrounding context blocks render as plain paragraphs while the backlinked
 * block carries the emphasis background.
 */

export interface BacklinkSnippetBodyProps {
  noteTitle: string;
  noteId: string;
  context: string[];
  anchorContent: string;
  loading: boolean;
  error: boolean;
  onNavigate: () => void;
  /** Height cap for the scrollable snippet window (desktop popover). */
  maxHeight?: number | string;
}

export default function BacklinkSnippetBody({
  noteTitle,
  noteId,
  context,
  anchorContent,
  loading,
  error,
  onNavigate,
  maxHeight,
}: BacklinkSnippetBodyProps) {
  if (loading) {
    return (
      <Box sx={{ p: 1.5, display: 'flex', justifyContent: 'center' }}>
        <CircularProgress size={16} />
      </Box>
    );
  }

  if (error) {
    return (
      <Box sx={{ maxWidth: 280, p: 1.5 }}>
        <Typography variant="subtitle2" sx={{ mb: 0.5 }}>
          {noteTitle || 'Backlinked note'}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          Could not load the backlinked snippet.
        </Typography>
      </Box>
    );
  }

  const blocks = (context ?? []).filter((b) => b && b.trim().length > 0);

  return (
    <Box sx={{ maxWidth: 320, p: 1.5 }}>
      <Typography
        variant="subtitle2"
        color="primary"
        component="button"
        type="button"
        onClick={onNavigate}
        sx={{
          cursor: 'pointer',
          bgcolor: 'transparent',
          border: 'none',
          p: 0,
          textAlign: 'left',
          font: 'inherit',
          '&:hover': { textDecoration: 'underline' },
          display: 'block',
        }}
      >
        {noteTitle || noteId}
      </Typography>
      <Box
        sx={{
          mt: 0.5,
          maxHeight,
          overflowY: blocks.length > 1 ? 'auto' : 'hidden',
          pr: blocks.length > 1 ? 0.5 : 0,
        }}
      >
        {blocks.length === 0 ? (
          <Typography variant="caption" color="text.disabled">
            (no content)
          </Typography>
        ) : (
          blocks.map((block, i) => {
            // The anchor block is the backlinked portion — highlight it.
            const isAnchor =
              anchorContent.length > 0 && block === anchorContent;
            return (
              <Typography
                key={i}
                variant="caption"
                component="div"
                sx={{
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                  color: isAnchor ? 'text.primary' : 'text.secondary',
                  fontWeight: isAnchor ? 600 : 400,
                  bgcolor: isAnchor ? 'action.selected' : 'transparent',
                  borderRadius: 0.5,
                  px: 0.5,
                  py: 0.25,
                  mb: 0.25,
                }}
              >
                {block}
              </Typography>
            );
          })
        )}
      </Box>
      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 0.5 }}>
        Ctrl+click to navigate
      </Typography>
    </Box>
  );
}
