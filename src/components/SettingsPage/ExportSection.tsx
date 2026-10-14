import { useState } from 'react';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import IconButton from '@mui/material/IconButton';
import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import { useTranslation } from 'react-i18next';
import { useStore } from '../../stores/appStore';
import { exportHtml, exportJson, exportPdf, exportZip } from '../../lib/commands/features';

interface ExportSectionProps {
  vaultPath: string;
}

type ExportKey = 'html' | 'json' | 'pdf' | 'zip';

interface ExportFinish {
  output_path: string;
  pages_exported?: number;
  files_packed?: number;
  page_count?: number;
}

interface ExportMessage {
  severity: 'success' | 'error';
  text: string;
  path?: string;
}

interface ExportRow {
  key: ExportKey;
  labelKey: string;
  filters: { name: string; extensions: string[] }[];
  browse: 'folder' | 'file';
  run: (path: string) => Promise<ExportFinish>;
  success: (r: ExportFinish) => string;
}

export default function ExportSection({ vaultPath }: ExportSectionProps) {
  const { t } = useTranslation();
  const currentPage = useStore(s => s.currentPage);
  const slug = currentPage?.slug || 'page';

  const [paths, setPaths] = useState<Record<ExportKey, string>>(() => ({
    html: `${vaultPath}/export-html`,
    json: `${vaultPath}/export-json`,
    pdf: `${vaultPath}/export-${slug}.pdf`,
    zip: `${vaultPath}/export.zip`,
  }));
  const [busyKey, setBusyKey] = useState<ExportKey | null>(null);
  const [msg, setMsg] = useState<ExportMessage | null>(null);

  const rows: ExportRow[] = [
    {
      key: 'html',
      labelKey: 'export.html',
      filters: [{ name: 'HTML', extensions: ['html'] }],
      browse: 'folder',
      run: async path => {
        const r = await exportHtml(path);
        return { output_path: r.output_dir, pages_exported: r.pages_exported };
      },
      success: r => t('export.success_html', { count: r.pages_exported, path: r.output_path }),
    },
    {
      key: 'json',
      labelKey: 'export.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
      browse: 'folder',
      run: async path => {
        const r = await exportJson(path);
        return { output_path: r.output_dir, pages_exported: r.pages_exported };
      },
      success: r => t('export.success_json', { count: r.pages_exported, path: r.output_path }),
    },
    {
      key: 'pdf',
      labelKey: 'export.pdf',
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
      browse: 'file',
      run: path => exportPdf(currentPage?.path || '', path),
      success: r => t('export.success_pdf', { path: r.output_path }),
    },
    {
      key: 'zip',
      labelKey: 'export.zip',
      filters: [{ name: 'ZIP', extensions: ['zip'] }],
      browse: 'file',
      run: path => exportZip(path),
      success: r => t('export.success_zip', { count: r.files_packed, pages: r.page_count, path: r.output_path }),
    },
  ];

  const handleBrowse = async (row: ExportRow) => {
    const dlg = await import('@tauri-apps/plugin-dialog');
    if (row.browse === 'folder') {
      const selected = await dlg.open({ directory: true, multiple: false });
      if (selected) setPaths(p => ({ ...p, [row.key]: selected }));
    } else {
      const selected = await dlg.save({ defaultPath: paths[row.key], filters: row.filters });
      if (selected) setPaths(p => ({ ...p, [row.key]: selected }));
    }
  };

  const handleExport = async (row: ExportRow) => {
    setBusyKey(row.key);
    setMsg(null);
    try {
      const r = await row.run(paths[row.key]);
      setMsg({ severity: 'success', text: row.success(r), path: r.output_path });
    } catch (e) {
      setMsg({ severity: 'error', text: `${t('export.failed')}${e}` });
    } finally {
      setBusyKey(null);
    }
  };

  const handleOpenFolder = async (path: string) => {
    try {
      const opener = await import('@tauri-apps/plugin-opener');
      await opener.revealItemInDir(path);
    } catch {
      // Opening the folder is best-effort (e.g. not supported on Android).
    }
  };

  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 0.5, color: 'text.secondary' }}>
        {t('export.section_label')}
      </Typography>
      <Typography variant="caption" sx={{ display: 'block', mb: 1.5, color: 'text.disabled' }}>
        {vaultPath}
      </Typography>

      {msg && (
        <Alert
          severity={msg.severity}
          sx={{ mb: 1.5 }}
          action={
            msg.severity === 'success' && msg.path ? (
              <IconButton size="small" color="inherit" onClick={() => handleOpenFolder(msg.path!)}>
                <FolderOpenIcon fontSize="small" />
              </IconButton>
            ) : undefined
          }
        >
          {msg.text}
        </Alert>
      )}

      {rows.map(row => {
        const disabled = row.key === 'pdf' && !currentPage;
        return (
          <Box key={row.key} sx={{ display: 'flex', alignItems: 'flex-start', gap: 1, mb: 1.5 }}>
            <TextField
              label={t('export.output_path')}
              value={paths[row.key]}
              onChange={e => setPaths(p => ({ ...p, [row.key]: e.target.value }))}
              size="small"
              sx={{ flex: 1, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
              helperText={row.key === 'pdf' && !currentPage ? t('export.no_current_page') : undefined}
            />
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
              <Button variant="outlined" size="small" onClick={() => handleBrowse(row)}>
                {t('export.browse')}
              </Button>
              <Button
                variant="contained"
                size="small"
                disabled={disabled || busyKey !== null}
                onClick={() => handleExport(row)}
              >
                {busyKey === row.key ? t('export.exporting') : t(row.labelKey)}
              </Button>
            </Box>
          </Box>
        );
      })}
    </Box>
  );
}
