import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

interface VaultTabProps {
  vaultPath: string;
  onVaultPathChange: (path: string) => void;
  onBrowse: () => void;
}

export default function VaultTab({ vaultPath, onVaultPathChange, onBrowse }: VaultTabProps) {
  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>Vault</Typography>
      <Box sx={{ maxWidth: 480 }}>
        <TextField
          label="Vault Path"
          value={vaultPath}
          onChange={e => onVaultPathChange(e.target.value)}
          fullWidth
          size="small"
          sx={{ mb: 1, '& .MuiInputBase-input': { fontFamily: 'monospace' } }}
        />
        <Button variant="outlined" size="small" onClick={onBrowse}>
          Browse
        </Button>
      </Box>
    </Box>
  );
}
