import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Tooltip from '@mui/material/Tooltip';
import RefreshIcon from '@mui/icons-material/Refresh';
import FileUploadIcon from '@mui/icons-material/FileUpload';

interface Props {
  collapsed: boolean;
  exporting: boolean;
  onRefresh: () => void;
  onExport: () => void;
}

export default function SidebarFooter({ collapsed, exporting, onRefresh, onExport }: Props) {
  return (
    <Box
      sx={{
        borderTop: 1,
        borderColor: 'divider',
        display: 'flex',
        alignItems: 'center',
        justifyContent: collapsed ? 'center' : 'space-between',
        flexDirection: collapsed ? 'column' : 'row',
        gap: collapsed ? 0.25 : 0,
        px: collapsed ? 0.5 : 1.5,
        py: collapsed ? 0.75 : 1,
      }}
    >
      {collapsed ? (
        <>
          <Tooltip title="Refresh" arrow>
            <IconButton size="small" onClick={onRefresh} sx={{ color: 'text.secondary' }}>
              <RefreshIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          <Tooltip title="Export HTML" arrow>
            <span>
              <IconButton size="small" onClick={onExport} disabled={exporting} sx={{ color: 'text.secondary' }}>
                <FileUploadIcon fontSize="small" />
              </IconButton>
            </span>
          </Tooltip>
        </>
      ) : (
        <>
          <Button
            size="small"
            onClick={onRefresh}
            startIcon={<RefreshIcon />}
            sx={{ color: 'text.secondary', textTransform: 'none', fontSize: '0.75rem' }}
          >
            Refresh
          </Button>
          <Button
            size="small"
            onClick={onExport}
            disabled={exporting}
            startIcon={<FileUploadIcon />}
            sx={{ color: 'text.secondary', textTransform: 'none', fontSize: '0.75rem' }}
          >
            {exporting ? '...' : 'Export'}
          </Button>
          <Typography variant="caption" color="text.disabled">
            v{__APP_VERSION__}
          </Typography>
        </>
      )}
    </Box>
  );
}
