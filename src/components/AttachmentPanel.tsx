import { useCallback, useEffect, useRef, useState } from 'react';
import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItem from '@mui/material/ListItem';
import ListItemText from '@mui/material/ListItemText';
import IconButton from '@mui/material/IconButton';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import CircularProgress from '@mui/material/CircularProgress';
import AttachFileIcon from '@mui/icons-material/AttachFile';
import DeleteIcon from '@mui/icons-material/Delete';
import { useTranslation } from 'react-i18next';
import * as api from '../lib/commands';
import { assetFileToBase64, ASSET_MAX_BYTES } from '../lib/assetUpload';
import { useStore } from '../stores/appStore';
import type { AssetRefDto } from '../lib/types';

interface Props {
  pagePath: string;
}

const FILE_INPUT_ID = 'asset-file-input';

function basename(p: string): string {
  return p.split('/').pop() || p;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export default function AttachmentPanel({ pagePath }: Props) {
  const { t } = useTranslation();
  const [refs, setRefs] = useState<AssetRefDto[]>([]);
  const [busy, setBusy] = useState(false);
  const [compressing, setCompressing] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    try {
      const refs = await api.getPageAssetRefs(pagePath);
      setRefs(Array.isArray(refs) ? refs : []);
    } catch (e) {
      console.error(t('asset.upload_failed'), e);
      setRefs([]);
    }
  }, [pagePath, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const importOne = async (file: File) => {
    if (file.size > ASSET_MAX_BYTES && !(file.type || '').startsWith('image/')) {
      useStore.getState().showError(t('asset.file_too_large'));
      return;
    }
    const { b64, mime } = await assetFileToBase64(file);
    const ref = await api.importAssetBytes(b64, file.name, mime || undefined, undefined);
    await api.insertBlock(pagePath, `![[${ref.vault_relative_path}]]`);
  };

  const handleFiles = async (files: File[]) => {
    if (files.length === 0) return;
    setBusy(true);
    const needsCompress = files.some(
      f => f.size > ASSET_MAX_BYTES && (f.type || '').startsWith('image/'),
    );
    if (needsCompress) setCompressing(true);
    try {
      for (const file of files) {
        await importOne(file);
      }
      await refresh();
      void useStore.getState().openPage(pagePath);
    } catch (e) {
      console.error(t('asset.upload_failed'), e);
      useStore.getState().showError(t('asset.upload_failed'));
    } finally {
      setCompressing(false);
      setBusy(false);
    }
  };

  const deleteOne = async (refPath: string) => {
    if (!window.confirm(t('asset.delete_confirm'))) return;
    try {
      await api.deleteAssetRef(refPath);
      await refresh();
      void useStore.getState().openPage(pagePath);
    } catch (e) {
      console.error(t('asset.upload_failed'), e);
      useStore.getState().showError(t('asset.upload_failed'));
    }
  };

  const totalSize = refs.reduce((acc, r) => acc + r.size, 0);

  return (
    <Card variant="outlined" sx={{ m: 1, mt: 1.5 }}>
      <CardContent sx={{ py: 1.5 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
          <Typography
            variant="caption"
            sx={{ fontWeight: 600, textTransform: 'uppercase', color: 'text.secondary' }}
          >
            {t('asset.attachments')} ({refs.length})
          </Typography>
          {refs.length > 0 && (
            <Typography variant="caption" color="text.disabled">
              {formatBytes(totalSize)}
            </Typography>
          )}
          <Box sx={{ flex: 1 }} />
          <input
            ref={inputRef}
            id={FILE_INPUT_ID}
            type="file"
            multiple
            style={{ display: 'none' }}
            onChange={e => {
              if (e.target.files && e.target.files.length > 0) {
                void handleFiles(Array.from(e.target.files));
              }
              e.target.value = '';
            }}
          />
          <Button
            size="small"
            variant="outlined"
            startIcon={<AttachFileIcon />}
            onClick={() => inputRef.current?.click()}
            disabled={busy}
            sx={{ textTransform: 'none', fontSize: '0.7rem' }}
          >
            {compressing ? t('asset.compressing') : t('asset.attach')}
          </Button>
        </Box>

        {busy && !compressing && <CircularProgress size={16} sx={{ mb: 1 }} />}

        {refs.length === 0 && !busy && (
          <Typography variant="caption" color="text.disabled">
            {t('asset.empty')}
          </Typography>
        )}

        {refs.length > 0 && (
          <List dense disablePadding>
            {refs.map(r => (
              <ListItem key={r.vault_relative_path} disablePadding sx={{ py: 0.25 }}>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontSize: '0.8rem' }}>
                      {basename(r.vault_relative_path)}
                    </Typography>
                  }
                  secondary={
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <Chip
                        label={r.mime}
                        size="small"
                        variant="outlined"
                        sx={{ height: 18, fontSize: '0.65rem' }}
                      />
                      <Typography variant="caption" color="text.disabled">
                        {formatBytes(r.size)}
                      </Typography>
                    </Box>
                  }
                />
                <IconButton
                  size="small"
                  onClick={() => void deleteOne(r.vault_relative_path)}
                  title={t('asset.delete')}
                  sx={{ p: 0.5 }}
                >
                  <DeleteIcon fontSize="small" />
                </IconButton>
              </ListItem>
            ))}
          </List>
        )}
      </CardContent>
    </Card>
  );
}

export { FILE_INPUT_ID };
