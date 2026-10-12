import Box from '@mui/material/Box';
import Tabs from '@mui/material/Tabs';
import Tab from '@mui/material/Tab';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import VaultTab from './VaultTab';
import ThemeTab from './ThemeTab';
import AITab from './AITab';
import ResearchTab from './ResearchTab';
import DeveloperTab from './DeveloperTab';
import SyncTab from './SyncTab';
import TrayTab from './TrayTab';
import { useTranslation } from 'react-i18next';
import { useSettingsPage, type SettingsTab } from './SettingsPage.shared';

export default function SettingsPageDesktop() {
  const { t } = useTranslation();
  const {
    settings,
    saving,
    fetching,
    msg,
    msgSeverity,
    availableModels,
    tab,
    setTab,
    syncStatus,
    commits,
    commitsOpen,
    syncing,
    ai,
    research,
    theme,
    syncSettings,
    stt,
    tts,
    locale,
    onLocaleChange,
    setMsg,
    updateAi,
    updateVault,
    updateResearch,
    updateTheme,
    updateSync,
    updateStt,
    updateTts,
    tray,
    updateTray,
    handleSave,
    handleFetchModels,
    handleReindex,
    handleRepair,
    handleNormalizeAll,
    reindexProgress,
    handleSyncNow,
    toggleModelCapability,
    handleToggleCommits,
    handleStartScheduler,
    handlePickVaultDirectory,
    handleResetOnboarding,
    handleInstallStarterTemplates,
    installing,
  } = useSettingsPage();

  if (!settings) {
    return (
      <Box sx={{ p: 3 }}>
        {/* Intentionally minimal — no loading verbosity needed */}
        <Box sx={{ color: 'text.secondary', typography: 'body2' }}>
          {t('settings.loading_settings')}
        </Box>
      </Box>
    );
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Tab bar */}
      <Box
        sx={{
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          flexShrink: 0,
        }}
      >
        <Tabs value={tab} onChange={(_, v: SettingsTab) => setTab(v)} sx={{ minHeight: 40 }}>
          <Tab label={t('settings.vault')} value="vault" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label={t('settings.theme')} value="theme" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label={t('settings.ai')} value="ai" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label={t('settings.research')} value="research" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label={t('settings.developer')} value="developer" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label={t('settings.sync')} value="sync" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label={t('settings.tray')} value="tray" sx={{ minHeight: 40, textTransform: 'none' }} />
        </Tabs>
        <Box sx={{ flex: 1 }} />
        <Button
          variant="contained"
          onClick={handleSave}
          disabled={saving}
          sx={{ mr: 2, textTransform: 'none' }}
        >
          {saving ? t('settings.saving') : t('settings.save')}
        </Button>
      </Box>

      {/* Message */}
      {msg && (
        <Alert severity={msgSeverity} sx={{ mx: 3, mt: 1.5 }} onClose={() => setMsg('')}>
          {msg}
        </Alert>
      )}

      {/* Tab content */}
      <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
        {tab === 'vault' && (
          <VaultTab
            vaultPath={settings.vault_path || ''}
            onVaultPathChange={path => updateVault({ vault_path: path })}
            onBrowse={handlePickVaultDirectory}
          />
        )}
        {tab === 'theme' && (
          <ThemeTab theme={theme} onThemeChange={updateTheme} locale={locale} onLocaleChange={onLocaleChange} />
        )}
        {tab === 'ai' && (
          <AITab
            ai={ai}
            onAiChange={updateAi}
            availableModels={availableModels}
            fetching={fetching}
            onFetchModels={handleFetchModels}
            onToggleModelCapability={toggleModelCapability}
            stt={stt}
            onSttChange={updateStt}
            tts={tts}
            onTtsChange={updateTts}
          />
        )}
        {tab === 'research' && (
          <ResearchTab research={research} onResearchChange={updateResearch} />
        )}
        {tab === 'developer' && (
          <DeveloperTab
            fetching={fetching}
            onReindex={handleReindex}
            onRepair={handleRepair}
            onNormalizeAll={handleNormalizeAll}
            reindexProgress={reindexProgress}
            onResetOnboarding={handleResetOnboarding}
            onInstallTemplates={handleInstallStarterTemplates}
            installing={installing}
          />
        )}
        {tab === 'sync' && (
          <SyncTab
            syncSettings={syncSettings}
            onSyncChange={updateSync}
            syncStatus={syncStatus}
            commits={commits}
            commitsOpen={commitsOpen}
            onToggleCommits={handleToggleCommits}
            syncing={syncing}
            onSyncNow={handleSyncNow}
            onStartScheduler={handleStartScheduler}
          />
        )}
        {tab === 'tray' && (
          <TrayTab tray={tray} onTrayChange={updateTray} />
        )}
      </Box>
    </Box>
  );
}
