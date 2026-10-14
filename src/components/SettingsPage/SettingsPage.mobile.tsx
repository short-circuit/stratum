import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import TextField from '@mui/material/TextField';
import UploadFileIcon from '@mui/icons-material/UploadFile';
import { useTranslation } from 'react-i18next';
import { useSettingsPage } from './SettingsPage.shared';
import MobileThemeSection from './MobileThemeSection';
import MobileAiAccordion from './MobileAiAccordion';
import MobileSttTtsSection from './MobileSttTtsSection';
import MobileDeveloperSection from './MobileDeveloperSection';
import MobileSyncSection from './MobileSyncSection';
import MobileNotificationsSection from './MobileNotificationsSection';
import ImportDialog from '../ImportDialog';
import ResearchPresetsSection from './ResearchPresetsSection';

// ---------------------------------------------------------------------------
// Mobile settings page.
// Rendered when the app is in the mobile flow (see SettingsPage/index.tsx).
// Reuses the shared `useSettingsPage` hook (same state/actions as desktop) and
// mirrors the desktop tab surface as a vertical scroll of sections so every
// desktop setting is reachable on a touch device.
// ---------------------------------------------------------------------------

export default function SettingsPageMobile() {
  const { t } = useTranslation();
  const [importOpen, setImportOpen] = useState(false);
  const {
    settings,
    saving,
    fetching,
    msg,
    msgSeverity,
    syncing,
    syncStatus,
    theme,
    updateTheme,
    locale,
    onLocaleChange,
    ai,
    updateAi,
    research,
    setMsg,
    updateVault,
    updateResearch,
    handleSave,
    handleReindex,
    handleRepair,
    handleNormalizeAll,
    reindexProgress,
    handleSyncNow,
    handlePickVaultDirectory,
    // AI model fetch + capability editor
    availableModels,
    handleFetchModels,
    toggleModelCapability,
    // STT / TTS
    stt,
    tts,
    updateStt,
    updateTts,
    voice,
    updateVoice,
    reminders,
    updateReminders,
    // Sync
    syncSettings,
    updateSync,
    backup,
    backupStatus,
    updateBackup,
    runBackupNow,
    testWebdavConnection,
    testing,
    running,
    webdavTestResult,
    commits,
    commitsOpen,
    handleToggleCommits,
    handleStartScheduler,
    handleResetOnboarding,
    handleInstallStarterTemplates,
    installing,
  } = useSettingsPage();

  if (!settings) {
    return (
      <Box sx={{ p: 2 }}>
        <Typography variant="body2" color="text.secondary">
          {t('settings.loading_settings')}
        </Typography>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100%', overflow: 'auto', display: 'flex', flexDirection: 'column' }}>
      {/* Sticky save bar + message */}
      <Box sx={{ position: 'sticky', top: 0, zIndex: 10, bgcolor: 'background.default', px: 2, pt: 2, pb: 1 }}>
        <Button
          variant="contained"
          onClick={handleSave}
          disabled={saving}
          fullWidth
          sx={{ textTransform: 'none', mb: 1 }}
        >
          {saving ? t('settings.saving') : 'Save'}
        </Button>
        {msg && (
          <Alert severity={msgSeverity} onClose={() => setMsg('')} sx={{ mb: 1 }}>
            {msg}
          </Alert>
        )}
      </Box>

      {/* Scrollable content */}
      <Box sx={{ flex: 1 }}>
        {/* ─── Vault Section ─── */}
        <Box sx={{ px: 2, pt: 2 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
            {t('settings.vault')}
          </Typography>
          <Divider sx={{ mb: 1.5 }} />
          <TextField
            label={t('settings.vault_path')}
            value={settings.vault_path || ''}
            onChange={e => updateVault({ vault_path: e.target.value })}
            fullWidth
            size="small"
            sx={{ mb: 1, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
          />
          <Button variant="outlined" size="small" onClick={handlePickVaultDirectory}>
            {t('settings.browse')}
          </Button>
          <Button
            variant="outlined"
            size="small"
            sx={{ ml: 1 }}
            startIcon={<UploadFileIcon />}
            onClick={() => setImportOpen(true)}
          >
            {t('importers.import_notes')}
          </Button>
        </Box>
        <ImportDialog open={importOpen} onClose={() => setImportOpen(false)} />

        {/* ─── Theme Section ─── */}
        <MobileThemeSection theme={theme} updateTheme={updateTheme} locale={locale} onLocaleChange={onLocaleChange} />

        {/* ─── AI Section (collapsible) ─── */}
        <MobileAiAccordion
          ai={ai}
          updateAi={updateAi}
          availableModels={availableModels}
          fetching={fetching}
          onFetchModels={handleFetchModels}
          onToggleModelCapability={toggleModelCapability}
        />

        {/* ─── STT / TTS Section ─── */}
        <MobileSttTtsSection
          stt={stt}
          updateStt={updateStt}
          tts={tts}
          updateTts={updateTts}
          voice={voice}
          updateVoice={updateVoice}
        />

        {/* ─── Research Section ─── */}
        <Box sx={{ px: 2, pt: 3 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
            {t('settings.research')}
          </Typography>
          <Divider sx={{ mb: 1.5 }} />
          <TextField
            label={t('settings.searxng_endpoint')}
            placeholder="http://localhost:8888"
            value={research.searxng_endpoint}
            onChange={e => updateResearch({ searxng_endpoint: e.target.value })}
            fullWidth
            size="small"
            helperText={t('settings.url_of_your_searxng_instance')}
          />
          <Box sx={{ display: 'flex', gap: 1, mt: 1.5 }}>
            <TextField
              label={t('settings.max_results')}
              type="number"
              value={research.max_results}
              onChange={e => updateResearch({ max_results: parseInt(e.target.value) || 3 })}
              size="small"
              slotProps={{ htmlInput: { min: 1, max: 10 } }}
            />
            <TextField
              label={t('settings.research_depth')}
              type="number"
              value={research.max_depth}
              onChange={e => updateResearch({ max_depth: parseInt(e.target.value) || 2 })}
              size="small"
              slotProps={{ htmlInput: { min: 1, max: 5 } }}
            />
          </Box>
          <Box sx={{ mt: 2 }}>
            <ResearchPresetsSection research={research} onResearchChange={updateResearch} />
          </Box>
        </Box>

        {/* ─── Developer Section ─── */}
        <MobileDeveloperSection
          fetching={fetching}
          onReindex={handleReindex}
          onRepair={handleRepair}
          onNormalizeAll={handleNormalizeAll}
          reindexProgress={reindexProgress}
          onResetOnboarding={handleResetOnboarding}
          onInstallTemplates={handleInstallStarterTemplates}
          installing={installing}
        />

        {/* ─── Sync Section ─── */}
        <MobileSyncSection
          syncSettings={syncSettings}
          updateSync={updateSync}
          syncStatus={syncStatus}
          syncing={syncing}
          onSyncNow={handleSyncNow}
          onStartScheduler={handleStartScheduler}
          commits={commits}
          commitsOpen={commitsOpen}
          onToggleCommits={handleToggleCommits}
          backup={backup}
          backupStatus={backupStatus}
          updateBackup={updateBackup}
          runBackupNow={runBackupNow}
          testWebdavConnection={testWebdavConnection}
          testing={testing}
          running={running}
          webdavTestResult={webdavTestResult}
        />

        {/* ─── Notifications Section ─── */}
        <MobileNotificationsSection reminders={reminders} updateReminders={updateReminders} />
      </Box>
    </Box>
  );
}
