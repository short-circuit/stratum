import { useState, memo } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemText from '@mui/material/ListItemText';
import IconButton from '@mui/material/IconButton';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Tooltip from '@mui/material/Tooltip';
import AddIcon from '@mui/icons-material/Add';
import ArticleIcon from '@mui/icons-material/Article';
import DeleteIcon from '@mui/icons-material/Delete';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import type { PageDto } from '../../lib/types';
import ConfirmDialog from '../ui/ConfirmDialog';
import { useTranslation } from 'react-i18next';
import { groupPagesByFolder, type FolderNode } from '../../lib/pageTree';

interface Props {
  pages: PageDto[];
  collapsed: boolean;
  showNew: boolean;
  newPath: string;
  newTitle: string;
  onShowNewChange: (v: boolean) => void;
  onNewPathChange: (v: string) => void;
  onNewTitleChange: (v: string) => void;
  onCreatePage: () => Promise<void>;
  onDeletePage: (path: string) => void;
  onNavigate: (path: string) => void;
  onNavigateHome?: () => void;
}

export interface PageTreeListProps {
  pages: PageDto[];
  /** Receives the raw page path (e.g. "pages/a.md"); the caller builds the URL. */
  onNavigate: (path: string) => void;
  /** When omitted, leaves render without a delete button. */
  onDelete?: (path: string) => void;
  showBlockCount?: boolean;
}

interface FolderItemProps {
  node: FolderNode;
  pageMap: Map<string, PageDto>;
  onNavigate: (path: string) => void;
  onDelete?: (path: string) => void;
  showBlockCount: boolean;
}

/**
 * Recursively renders one node of the folder tree. Folders default to expanded
 * and remain conditionally rendered (not hidden via MUI Collapse) so the tree
 * works in jsdom and honours `react/no-unstable-nested-components`.
 */
function FolderItem({ node, pageMap, onNavigate, onDelete, showBlockCount }: FolderItemProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(true);

  if (node.isFolder) {
    return (
      <>
        <ListItemButton dense onClick={() => setExpanded(e => !e)} sx={{ borderRadius: 1 }}>
          <IconButton
            size="small"
            aria-label={expanded ? t('sidebar.collapse_folder') : t('sidebar.expand_folder')}
            aria-expanded={expanded}
            onClick={e => {
              e.stopPropagation();
              setExpanded(v => !v);
            }}
            sx={{
              ml: -0.5,
              mr: 0.25,
              p: 0.25,
              color: 'text.secondary',
              transform: expanded ? 'rotate(90deg)' : undefined,
              transition: 'transform 0.15s ease',
            }}
          >
            <ChevronRightIcon fontSize="inherit" />
          </IconButton>
          <ListItemText primary={node.name} slotProps={{ primary: { variant: 'body2', noWrap: true } }} />
        </ListItemButton>
        {expanded && (
          <Box sx={{ pl: 2 }}>
            {node.children.map(child => (
              <FolderItem
                key={child.fullPath ?? child.name}
                node={child}
                pageMap={pageMap}
                onNavigate={onNavigate}
                onDelete={onDelete}
                showBlockCount={showBlockCount}
              />
            ))}
          </Box>
        )}
      </>
    );
  }

  const page = node.fullPath ? pageMap.get(node.fullPath) : undefined;
  return (
    <ListItemButton
      dense
      onClick={() => node.fullPath && onNavigate(node.fullPath)}
      sx={{ borderRadius: 1, '&:hover .delete-btn': { opacity: 1 } }}
    >
      <ListItemText
        primary={page?.title || node.name}
        slotProps={{
          primary: { variant: 'body2', noWrap: true },
          secondary: { variant: 'caption', color: 'text.disabled' as const },
        }}
        secondary={showBlockCount && page?.block_count ? `${page.block_count}b` : undefined}
      />
      {onDelete && (
        <IconButton
          size="small"
          className="delete-btn"
          sx={{
            opacity: 0,
            color: 'error.main',
            ml: 0.5,
            p: 0.25,
            '&:hover': { bgcolor: 'error.light', color: 'error.contrastText' },
          }}
          onClick={e => {
            e.stopPropagation();
            if (node.fullPath) onDelete(node.fullPath);
          }}
        >
          <DeleteIcon fontSize="inherit" />
        </IconButton>
      )}
    </ListItemButton>
  );
}

/**
 * Renders the grouped (folder-tree) page list only — no header, new-page form,
 * or delete confirmation. Shared by `PageTreeGrouped` and the PagesHome views
 * when `pages_view = "tree"`.
 */
export function PageTreeList({ pages, onNavigate, onDelete, showBlockCount = true }: PageTreeListProps) {
  const { t } = useTranslation();
  const pageMap = new Map(pages.map(p => [p.path, p]));
  const tree = groupPagesByFolder(pages.map(p => p.path));

  if (tree.length === 0) {
    return (
      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'center', py: 2 }}>
        {t('pages_home.no_pages_yet')}
      </Typography>
    );
  }

  return (
    <List dense disablePadding>
      {tree.map(node => (
        <FolderItem
          key={node.fullPath ?? node.name}
          node={node}
          pageMap={pageMap}
          onNavigate={onNavigate}
          onDelete={onDelete}
          showBlockCount={showBlockCount}
        />
      ))}
    </List>
  );
}

const PageTreeGrouped = memo(function PageTreeGrouped(props: Props) {
  const { t } = useTranslation();
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const {
    pages,
    collapsed,
    showNew,
    newPath,
    newTitle,
    onShowNewChange,
    onNewPathChange,
    onNewTitleChange,
    onCreatePage,
    onDeletePage,
    onNavigate,
    onNavigateHome,
  } = props;

  if (collapsed) return null;

  const navigateEncoded = (path: string) => onNavigate(`/page/${encodeURIComponent(path)}`);

  return (
    <>
      <Box sx={{ px: 1, pt: 1 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1, py: 0.5 }}>
          <Box
            component="span"
            onClick={onNavigateHome}
            sx={{ display: 'flex', alignItems: 'center', gap: 0.5, cursor: onNavigateHome ? 'pointer' : undefined, '&:hover': onNavigateHome ? { color: 'primary.main' } : undefined }}
          >
            <ArticleIcon fontSize="inherit" sx={{ fontSize: '0.8rem', color: 'text.secondary' }} />
            <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', textTransform: 'uppercase' }}>
              {t('sidebar.recent')}
            </Typography>
          </Box>
          <Tooltip title={t('sidebar.new_page')} arrow>
            <IconButton size="small" onClick={() => onShowNewChange(!showNew)} sx={{ color: 'text.secondary' }}>
              <AddIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </Box>

        {showNew && (
          <Box sx={{ p: 1, mb: 1, bgcolor: 'action.hover', borderRadius: 1 }}>
            <TextField
              size="small"
              placeholder={t('sidebar.path_e_g_pages_my_note_md')}
              value={newPath}
              onChange={e => onNewPathChange(e.target.value)}
              fullWidth
              sx={{ mb: 0.5, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
            />
            <TextField
              size="small"
              placeholder={t('sidebar.title_optional')}
              value={newTitle}
              onChange={e => onNewTitleChange(e.target.value)}
              fullWidth
              sx={{ mb: 0.75, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
            />
            <Box sx={{ display: 'flex', gap: 0.5 }}>
              <Button size="small" variant="contained" onClick={onCreatePage}>
                {t('sidebar.create')}
              </Button>
              <Button size="small" onClick={() => onShowNewChange(false)}>
                {t('sidebar.cancel')}
              </Button>
            </Box>
          </Box>
        )}

        <PageTreeList
          pages={pages}
          onNavigate={navigateEncoded}
          onDelete={setDeleteTarget}
          showBlockCount
        />
      </Box>

      <ConfirmDialog
        open={deleteTarget !== null}
        title={t('sidebar.delete_page')}
        message={`${t('editor.delete')} ${deleteTarget ?? ''}? ${t('common.this_cannot_be_undone')}`}
        confirmLabel={t('editor.delete')}
        confirmColor="error"
        onConfirm={() => {
          if (deleteTarget) onDeletePage(deleteTarget);
          setDeleteTarget(null);
        }}
        onCancel={() => setDeleteTarget(null)}
      />
    </>
  );
});

export default PageTreeGrouped;
