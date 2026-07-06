import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import CircularProgress from '@mui/material/CircularProgress';
import Tabs from '@mui/material/Tabs';
import Tab from '@mui/material/Tab';
import CloseIcon from '@mui/icons-material/Close';
import KeyboardArrowUpIcon from '@mui/icons-material/KeyboardArrowUp';
import { useResponsive } from '../../lib/hooks/useResponsive';
import { useLongPress } from '../../lib/hooks/useLongPress';
import { useBacklinksData, usePreview } from './BacklinksPanel.shared';
import type { BacklinksPanelProps } from './BacklinksPanel.shared';
import type { BacklinkItem } from '../../lib/types';

function BacklinkRow({
  item,
  onNavigate,
  onLongPress,
}: {
  item: BacklinkItem;
  onNavigate: (path: string) => void;
  onLongPress: (item: BacklinkItem) => void;
}) {
  const handlers = useLongPress({
    onLongPress: () => onLongPress(item),
    onClick: () => onNavigate(item.source_page),
  });

  return (
    <ListItemButton
      {...(handlers as React.HTMLAttributes<HTMLDivElement>)}
      dense
      sx={{ borderRadius: 1, flexDirection: 'column', alignItems: 'flex-start' }}
    >
      <Typography variant="caption" color="text.secondary">{item.source_page}</Typography>
      <Typography variant="body2" noWrap sx={{ maxWidth: '100%' }}>{item.context}</Typography>
    </ListItemButton>
  );
}

export default function BacklinksPanelMobile({ pagePath }: BacklinksPanelProps) {
  const navigate = useNavigate();
  const { isMobile } = useResponsive();
  const { backlinks, loading, linked, unlinked } = useBacklinksData(pagePath);
  const { preview, showPreview, dismissPreview } = usePreview();
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState(0);

  const handleTabChange = (_: React.SyntheticEvent, v: number) => setTab(v);

  const handleLongPress = useCallback((item: BacklinkItem) => {
    showPreview(item);
  }, [showPreview]);

  const handleNavigate = useCallback((path: string) => {
    navigate(`/page/${encodeURIComponent(path)}`);
    setOpen(false);
  }, [navigate]);

  const items = tab === 0 ? linked : unlinked;

  return (
    <>
      <Box
        onClick={() => setOpen(true)}
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 0.5,
          py: 1,
          borderTop: 1,
          borderColor: 'divider',
          bgcolor: 'background.paper',
          cursor: 'pointer',
          '&:active': { opacity: 0.7 },
        }}
      >
        <KeyboardArrowUpIcon fontSize="small" color="action" />
        <Typography variant="caption" color="text.secondary" sx={{ fontWeight: 600 }}>
          Backlinks ({backlinks.length})
        </Typography>
      </Box>

      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        fullScreen={isMobile}
        slotProps={{
          paper: {
            sx: isMobile
              ? { height: '85vh', borderTopLeftRadius: 16, borderTopRightRadius: 16, marginTop: 'auto' }
              : {},
          },
        }}
      >
        <DialogTitle sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', pb: 0 }}>
          <Typography variant="h6">Backlinks ({backlinks.length})</Typography>
          <IconButton onClick={() => setOpen(false)} size="small" aria-label="Close backlinks">
            <CloseIcon />
          </IconButton>
        </DialogTitle>

        <DialogContent sx={{ pt: 1 }}>
          <Tabs value={tab} onChange={handleTabChange} sx={{ mb: 1 }} variant="fullWidth">
            <Tab label={`Linked (${linked.length})`} />
            <Tab label={`Unlinked (${unlinked.length})`} />
          </Tabs>

          {loading && <CircularProgress size={14} sx={{ display: 'block', mx: 'auto' }} />}

          {!loading && backlinks.length === 0 && (
            <Typography variant="caption" color="text.disabled">No backlinks found.</Typography>
          )}

          <List dense disablePadding>
            {items.map((bl, i) => (
              <BacklinkRow
                key={bl.source_id || i}
                item={bl}
                onNavigate={handleNavigate}
                onLongPress={handleLongPress}
              />
            ))}
          </List>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(preview)} onClose={dismissPreview} fullWidth maxWidth="xs">
        {preview?.loading ? (
          <DialogContent sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
            <CircularProgress size={14} />
          </DialogContent>
        ) : (
          <>
            <DialogTitle sx={{ pb: 0.5 }}>
              <Typography
                variant="subtitle2"
                color="primary"
                sx={{ cursor: 'pointer', '&:hover': { textDecoration: 'underline' } }}
                onClick={() => { navigate(`/page/${encodeURIComponent(preview!.pagePath)}`); dismissPreview(); setOpen(false); }}
              >
                {preview?.pageTitle || preview?.pagePath}
              </Typography>
            </DialogTitle>
            <DialogContent>
              <Typography variant="body2" color="text.secondary">{preview?.content}</Typography>
            </DialogContent>
          </>
        )}
      </Dialog>
    </>
  );
}
