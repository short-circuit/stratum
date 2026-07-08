import { useMemo, memo } from 'react';
import Popper from '@mui/material/Popper';
import Paper from '@mui/material/Paper';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemText from '@mui/material/ListItemText';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import Box from '@mui/material/Box';
import type { AutocompleteItem } from '../lib/types';

interface AutocompletePopupProps {
  open: boolean;
  anchorPosition: { x: number; y: number } | null;
  items: AutocompleteItem[];
  loading: boolean;
  selectedIndex: number;
  query: string;
  onSelect: (item: AutocompleteItem) => void;
  onClose: () => void;
}

function highlightText(text: string, query: string): React.ReactNode {
  if (!query) return text;

  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const regex = new RegExp(`(${escaped})`, 'gi');
  const parts = text.split(regex);

  if (parts.length === 1) return text;

  return parts.map((part, i) => {
    if (part.toLowerCase() === query.toLowerCase()) {
      return (
        <Typography key={i} component="span" color="primary" sx={{ fontWeight: 600 }}>
          {part}
        </Typography>
      );
    }
    return part;
  });
}

const AutocompletePopup = memo(function AutocompletePopup({
  open,
  anchorPosition,
  items,
  loading,
  selectedIndex,
  query,
  onSelect,
  onClose,
}: AutocompletePopupProps) {
  const virtualElement = useMemo(() => {
    if (!anchorPosition) return null;
    const rect = {
      top: anchorPosition.y,
      left: anchorPosition.x,
      bottom: anchorPosition.y,
      right: anchorPosition.x,
      width: 0,
      height: 0,
      x: anchorPosition.x,
      y: anchorPosition.y,
      toJSON: () => JSON.stringify(rect),
    };
    return {
      getBoundingClientRect: () => rect,
    };
  }, [anchorPosition]);

  return (
    <Popper
      open={open && !!anchorPosition}
      anchorEl={virtualElement}
      placement="bottom-start"
    >
      <Paper
        elevation={8}
        sx={{
          minWidth: 220,
          maxWidth: 350,
          maxHeight: 300,
          overflow: 'auto',
          bgcolor: 'background.paper',
          border: 1,
          borderColor: 'divider',
        }}
        onKeyDown={(e: React.KeyboardEvent) => {
          if (e.key === 'Escape') onClose();
        }}
      >
        {loading ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', p: 2 }}>
            <CircularProgress size={20} />
          </Box>
        ) : items.length === 0 ? (
          <Typography
            variant="body2"
            color="text.secondary"
            sx={{ p: 2, textAlign: 'center' }}
          >
            No results
          </Typography>
        ) : (
          <List dense disablePadding>
            {items.map((item, index) => (
              <AutocompleteItem
                key={`${item.text}-${index}`}
                item={item}
                selected={index === selectedIndex}
                query={query}
                onSelect={onSelect}
              />
            ))}
          </List>
        )}
      </Paper>
    </Popper>
  );
});

interface AutocompleteItemProps {
  item: AutocompleteItem;
  selected: boolean;
  query: string;
  onSelect: (item: AutocompleteItem) => void;
}

const AutocompleteItem = memo(function AutocompleteItem({ item, selected, query, onSelect }: AutocompleteItemProps) {
  return (
    <ListItemButton
      selected={selected}
      onClick={() => onSelect(item)}
    >
      <ListItemText
        primary={highlightText(item.text, query)}
        secondary={
          item.kind === 'backlink' && item.detail
            ? item.detail
            : undefined
        }
      />
    </ListItemButton>
  );
});

export default AutocompletePopup;
