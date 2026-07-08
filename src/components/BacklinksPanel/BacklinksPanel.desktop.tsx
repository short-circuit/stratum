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
import Popover from '@mui/material/Popover';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import { useCtrlHeld } from '../../lib/useCtrlHeld';
import type { BacklinkItem } from '../../lib/types';
import { useBacklinksData, usePreview } from './BacklinksPanel.shared';
import type { BacklinksPanelProps } from './BacklinksPanel.shared';

const BacklinksPanelDesktop = memo(function BacklinksPanelDesktop({ pagePath }: BacklinksPanelProps) {
  const navigate = useNavigate();
  const { backlinks, loading, linked, unlinked } = useBacklinksData(pagePath);
  const { preview, showPreview, dismissPreview } = usePreview();
  const ctrlHeld = useCtrlHeld();
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleMouseEnter = (item: BacklinkItem, e: React.MouseEvent) => {
    const el = e.currentTarget as HTMLElement;
    hoverTimer.current = setTimeout(async () => {
      if (!ctrlHeld.current) return;
      showPreview(item, el);
    }, 200);
  };

  const handleMouseLeave = () => {
    if (hoverTimer.current) { clearTimeout(hoverTimer.current); hoverTimer.current = null; }
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
                    key={i}
                    dense
                    onClick={() => navigate(`/page/${encodeURIComponent(bl.source_page)}`)}
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
                    key={i}
                    dense
                    onClick={() => navigate(`/page/${encodeURIComponent(bl.source_page)}`)}
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

      <Popover
        open={Boolean(preview)}
        anchorEl={preview?.anchorEl}
        onClose={dismissPreview}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
        slotProps={{ paper: { sx: { maxWidth: 280, p: 1.5 } } }}
      >
        {preview?.loading ? (
          <CircularProgress size={14} />
        ) : (
          <>
            <Typography
              variant="subtitle2"
              color="primary"
              sx={{ cursor: 'pointer', '&:hover': { textDecoration: 'underline' }, mb: 0.5 }}
              onClick={() => { navigate(`/page/${encodeURIComponent(preview!.pagePath)}`); dismissPreview(); }}
            >
              {preview?.pageTitle || preview?.pagePath}
            </Typography>
            <Typography variant="caption" color="text.secondary" sx={{ display: '-webkit-box', WebkitLineClamp: 3, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>
              {preview?.content}
            </Typography>
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 0.5 }}>Ctrl+click to navigate</Typography>
          </>
        )}
      </Popover>
    </>
  );
});

export default BacklinksPanelDesktop;
