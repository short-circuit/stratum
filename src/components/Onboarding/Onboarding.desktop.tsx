import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Checkbox from '@mui/material/Checkbox';
import FormControlLabel from '@mui/material/FormControlLabel';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Link from '@mui/material/Link';
import { useStore } from '../../stores/appStore';
import { useOnboarding, STEPS } from './Onboarding.shared';
import type { OnboardingProps } from './index';

export default function OnboardingDesktop({ onDone }: OnboardingProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const ob = useOnboarding();
  const vault = useStore((s) => s.vault);
  const pickVaultDirectory = useStore((s) => s.pickVaultDirectory);
  const [picking, setPicking] = useState(false);

  const step = ob.step;
  const total = STEPS.length;

  const handlePick = async () => {
    setPicking(true);
    try {
      await pickVaultDirectory();
    } finally {
      setPicking(false);
    }
  };

  const handleFinish = () => {
    onDone();
    navigate('/journal');
  };

  const nextDisabled = (step === 0 && !vault) || ob.installing;
  const renderStep = () => {
    switch (step) {
      case 0:
        return (
          <Box>
            <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>
              {t('onboarding.welcome_title')}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t('onboarding.welcome_subtitle')}
            </Typography>
            <Button
              variant="contained"
              disabled={picking}
              onClick={handlePick}
              startIcon={picking ? <CircularProgress size={16} /> : undefined}
              sx={{ textTransform: 'none' }}
            >
              {t('onboarding.choose_vault_folder')}
            </Button>
            {vault && (
              <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
                {vault.path}
              </Typography>
            )}
          </Box>
        );
      case 1:
        return (
          <Box>
            <Typography variant="h6" sx={{ fontWeight: 600, mb: 1 }}>
              {t('onboarding.start_with_templates')}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t('onboarding.templates_hint')}
            </Typography>
            <FormControlLabel
              control={
                <Checkbox
                  checked={ob.startWithTemplates}
                  onChange={(e) => ob.toggleTemplates(e.target.checked)}
                />
              }
              label={t('onboarding.start_with_templates')}
            />
            {ob.startWithTemplates && (
              <Box sx={{ pl: 2, display: 'flex', flexDirection: 'column' }}>
                {ob.selectedTemplates.map((name) => (
                  <FormControlLabel
                    key={name}
                    control={
                      <Checkbox
                        size="small"
                        checked={ob.selectedTemplates.includes(name)}
                        onChange={() => ob.toggleTemplate(name)}
                      />
                    }
                    label={name}
                  />
                ))}
              </Box>
            )}
          </Box>
        );
      case 2:
        return (
          <Box>
            <Typography variant="h6" sx={{ fontWeight: 600, mb: 1 }}>
              {t('onboarding.sample_vault')}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t('onboarding.sample_vault_hint')}
            </Typography>
            <FormControlLabel
              control={
                <Checkbox
                  checked={ob.sampleVault}
                  onChange={(e) => ob.setSampleVault(e.target.checked)}
                />
              }
              label={t('onboarding.sample_vault')}
            />
          </Box>
        );
      case 3:
        return (
          <Box>
            <Typography variant="h6" sx={{ fontWeight: 600, mb: 1 }}>
              {t('onboarding.finish_title')}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t('onboarding.finish_hint')}
            </Typography>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
              <Button
                variant="contained"
                onClick={handleFinish}
                disabled={ob.installing}
                sx={{ textTransform: 'none' }}
              >
                {ob.installing ? t('onboarding.installing') : t('onboarding.finish')}
              </Button>
              <Link
                href="docs/getting-started/quickstart.md"
                target="_blank"
                rel="noopener noreferrer"
              >
                {t('onboarding.open_quick_start')}
              </Link>
            </Box>
          </Box>
        );
      default:
        return null;
    }
  };

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100vh',
        p: 2,
        bgcolor: 'background.default',
      }}
    >
      <Card sx={{ width: '100%', maxWidth: 540 }}>
        <CardContent sx={{ p: { xs: 2, sm: 4 } }}>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
            {t('onboarding.step', { current: step + 1, total })}
          </Typography>

          {renderStep()}

          {ob.error && (
            <Alert severity="error" sx={{ mt: 2 }}>
              {ob.error}
            </Alert>
          )}

          <Box sx={{ display: 'flex', justifyContent: 'space-between', mt: 3 }}>
            <Button
              onClick={ob.back}
              disabled={step === 0}
              sx={{ textTransform: 'none' }}
            >
              {t('onboarding.back')}
            </Button>
            {step < total - 1 && (
              <Button
                variant="contained"
                onClick={ob.handleNext}
                disabled={nextDisabled}
                sx={{ textTransform: 'none' }}
              >
                {ob.installing ? t('onboarding.installing') : t('onboarding.next')}
              </Button>
            )}
          </Box>
        </CardContent>
      </Card>
    </Box>
  );
}
