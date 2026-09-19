import { useRef, memo } from 'react';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Accordion from '@mui/material/Accordion';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import CircularProgress from '@mui/material/CircularProgress';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import { useCtrlHeld } from '../../lib/useCtrlHeld';
import type { BacklinkItem } from '../../lib/types';
import { useBacklinksData, usePreview } from './BacklinksPanel.shared';
import type { BacklinksPanelProps } from './BacklinksPanel.shared';
import BacklinkSnippetPopup from '../BacklinkSnippetPopup';

const BacklinksPanelDesktop = memo(function BacklinksPanelDesktop({ pagePath }: BacklinksPanelProps) {
  const navigate = useNavigate();
  const { backlinks, loading, linked, unlinked } = useBacklinksData(pagePath);
  const { preview, showPreview, dismissPreview } = usePreview();
  const ctrlHeld = useCtrlHeld();
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const popupHovered = useRef(false);

  const handleMouseEnter = (item: BacklinkItem, e: React.MouseEvent) => {
    const el = e.currentTarget as HTMLElement;
    hoverTimer.current = setTimeout(() => {
      if (!ctrlHeld.current) return;
      showPreview(item, el);
    }, 200);
  };

  const handleMouseLeave = () => {
    if (hoverTimer.current) { clearTimeout(hoverTimer.current); hoverTimer.current = null; }
    // Only auto-dismiss on leave once the popup is open, and only if the
    // pointer has not moved into the popup itself (it renders in a portal so
    // leaving the row and entering the popup would otherwise close it before
    // the user can click the title).
    if (preview && !popupHovered.current) dismissPreview();
  };

  const handleClick = (path: string) => (e: React.MouseEvent) => {
    // Ctrl/Meta+click navigates; plain click is unchanged (also navigates, as
    // before). Keeping the same action for both preserves existing behaviour.
    if (e.defaultPrevented) return;
    dismissPreview();
    navigate(`/page/${encodeURIComponent(path)}`);
  };

  return (
    <>
      <Accordion disableGutters square sx={{ boxShadow: 0, '&:before': { display: 'none' } }} slotProps={{ transition: { unmountOnExit: true } }}>
        <AccordionSummary expandIcon={<ExpandMoreIcon />}>
          <Typography variant="caption" sx={{ fontWeight: 600, textTransform: 'uppercase', color: 'text.secondary' }}>
            Backlinks ({backlinks.length})
          </Typography>
        </AccordionSummary>
        <AccordionDetails sx={{ maxHeight: 200, overflow: 'auto', p: 1.5 }}>
          {loading && <CircularProgress size={14} sx={{ display: 'block', mx: 'auto' }} />}

          {linked.length > 0 && (
            <Box sx={{ mb: 1.5 }}>
              <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mb: 0.5 }}>
                Linked References ({linked.length})
              </Typography>
              <List dense disablePadding>
                {linked.map((bl, i) => (
                  <ListItemButton
                    key={bl.source_id || i}
                    dense
                    onClick={handleClick(bl.source_page)}
                    onMouseEnter={(e) => handleMouseEnter(bl, e)}
                    onMouseLeave={handleMouseLeave}
                    sx={{ borderRadius: 1, flexDirection: 'column', alignItems: 'flex-start' }}
                  >
                    <Typography variant="caption" color="text.secondary">{bl.source_page}</Typography>
                    <Typography variant="caption" noWrap sx={{ maxWidth: '100%' }}>{bl.context}</Typography>
                  </ListItemButton>
                ))}
              </List>
            </Box>
          )}

          {unlinked.length > 0 && (
            <Box>
              <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mb: 0.5 }}>
                Unlinked Mentions ({unlinked.length})
              </Typography>
              <List dense disablePadding>
                {unlinked.map((bl, i) => (
                  <ListItemButton
                    key={bl.source_id || i}
                    dense
                    onClick={handleClick(bl.source_page)}
                    onMouseEnter={(e) => handleMouseEnter(bl, e)}
                    onMouseLeave={handleMouseLeave}
                    sx={{ borderRadius: 1, flexDirection: 'column', alignItems: 'flex-start' }}
                  >
                    <Typography variant="caption" color="text.secondary">{bl.source_page}</Typography>
                    <Typography variant="caption" color="text.disabled" noWrap sx={{ maxWidth: '100%' }}>{bl.context}</Typography>
                  </ListItemButton>
                ))}
              </List>
            </Box>
          )}

          {!loading && backlinks.length === 0 && (
            <Typography variant="caption" color="text.disabled">No backlinks found.</Typography>
          )}
        </AccordionDetails>
      </Accordion>

      {preview && (
        <BacklinkSnippetPopup
          noteId={preview.noteId}
          noteTitle={preview.noteTitle}
          context={preview.context}
          anchorContent={preview.anchorContent}
          position={
            preview.anchorEl
              ? (() => {
                  const r = preview.anchorEl.getBoundingClientRect();
                  return { x: r.left, y: r.bottom + 4 };
                })()
              : { x: 0, y: 0 }
          }
          loading={preview.loading}
          error={preview.error}
          onClose={dismissPreview}
          onHoverChange={(h) => { popupHovered.current = h; }}
        />
      )}
    </>
  );
});

export default BacklinksPanelDesktop;
