import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import Popover from '@mui/material/Popover';
import { useNavigate } from 'react-router-dom';

interface Props {
  content: string;
  pageTitle: string | null;
  pagePath: string;
  position: { x: number; y: number };
  loading: boolean;
  onClose: () => void;
}

export default function LinkPreviewPopup({ content, pageTitle, pagePath, position, loading, onClose }: Props) {
  const navigate = useNavigate();

  return (
    <Popover
      open
      onClose={onClose}
      anchorReference="anchorPosition"
      anchorPosition={{ left: position.x, top: position.y }}
      anchorOrigin={{ vertical: 'top', horizontal: 'left' }}
      transformOrigin={{ vertical: 'top', horizontal: 'left' }}
      slotProps={{ paper: { sx: { maxWidth: 280, p: 1.5 } } }}
    >
      {loading ? (
        <CircularProgress size={14} />
      ) : (
        <>
          <Typography
            variant="subtitle2"
            color="primary"
            sx={{ cursor: 'pointer', '&:hover': { textDecoration: 'underline' }, mb: 0.5 }}
            onClick={() => { navigate(`/page/${encodeURIComponent(pagePath)}`); onClose(); }}
          >
            {pageTitle || pagePath}
          </Typography>
          <Typography variant="caption" color="text.secondary" sx={{ display: '-webkit-box', WebkitLineClamp: 3, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>
            {content}
          </Typography>
          <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 0.5 }}>Ctrl+click to navigate</Typography>
        </>
      )}
    </Popover>
  );
}
