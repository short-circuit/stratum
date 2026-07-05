import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Chip from '@mui/material/Chip';
import Typography from '@mui/material/Typography';
import OpenInNewIcon from '@mui/icons-material/OpenInNew';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useNavigate } from 'react-router-dom';
import type { KanbanBlockDto } from '../../lib/types';

interface KanbanCardProps {
  block: KanbanBlockDto;
  onContextMenu?: (block: KanbanBlockDto, e: React.MouseEvent) => void;
  onClick?: (block: KanbanBlockDto) => void;
}

export default function KanbanCard({ block, onContextMenu, onClick }: KanbanCardProps) {
  const navigate = useNavigate();
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: block.id,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <Card
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      onContextMenu={(e) => onContextMenu?.(block, e)}
      onClick={() => onClick?.(block)}
      sx={{
        cursor: 'grab',
        '&:hover': { boxShadow: 3 },
        mb: 1,
        userSelect: 'none',
      }}
    >
      <CardContent sx={{ p: 1.5, '&:last-child': { pb: 1.5 } }}>
        <Typography variant="body2" sx={{ fontWeight: 500, mb: 0.5, wordBreak: 'break-word' }}>
          {block.content}
        </Typography>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, flexWrap: 'wrap' }}>
          {block.marker && (
            <Chip
              label={block.marker}
              size="small"
              variant="outlined"
              sx={{ height: 20, fontSize: '0.65rem' }}
            />
          )}
          <Button
            size="small"
            variant="text"
            sx={{
              fontSize: '0.65rem',
              p: 0,
              minWidth: 0,
              textTransform: 'none',
              color: 'text.secondary',
              '&:hover': { color: 'primary.main' },
            }}
            onClick={(e) => {
              e.stopPropagation();
              navigate(`/page/${encodeURIComponent(block.page_path)}`);
            }}
          >
            {block.page_title || block.page_path.replace('journals/', '').replace('.md', '')}
          </Button>
          {block.page_path.startsWith('journals/') && (
            <OpenInNewIcon sx={{ fontSize: '0.6rem', color: 'text.disabled' }} />
          )}
        </Box>
      </CardContent>
    </Card>
  );
}
