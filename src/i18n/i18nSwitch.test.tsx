import { describe, it, expect } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import { act } from 'react';
import { useTranslation } from 'react-i18next';
import { initI18n, setLocale } from './index';

//! Runtime locale-switch rendering test (E9.F1 acceptance: "switch to de → UI
//! translated, no English fallback in de/it/fr/es").
//!
//! Uses the REAL i18next instance and REAL locale JSON files through a minimal
//! component — this exercises the exact code path a component uses
//! (`useTranslation().t`) and proves the translated value renders, not the key
//! and not the English fallback.

function Probe() {
  const { t } = useTranslation();
  return (
    <div>
      <span data-testid="cancel">{t('common.cancel')}</span>
      <span data-testid="save">{t('common.save')}</span>
      <span data-testid="language">{t('settings.language')}</span>
    </div>
  );
}

describe('runtime locale switch renders translated UI', () => {
  it('en renders English values', async () => {
    await initI18n('en');
    render(<Probe />);
    expect(screen.getByTestId('cancel').textContent).toBe('Cancel');
    expect(screen.getByTestId('save').textContent).toBe('Save');
    expect(screen.getByTestId('language').textContent).toBe('Language');
    cleanup();
  });

  it('de renders native German (no English fallback, no raw key)', async () => {
    await initI18n('de');
    render(<Probe />);
    expect(screen.getByTestId('cancel').textContent).toBe('Abbrechen');
    expect(screen.getByTestId('save').textContent).toBe('Speichern');
    expect(screen.getByTestId('language').textContent).toBe('Sprache');
    cleanup();
  });

  it('it renders native Italian', async () => {
    await initI18n('it');
    render(<Probe />);
    expect(screen.getByTestId('cancel').textContent).toBe('Annulla');
    expect(screen.getByTestId('save').textContent).toBe('Salva');
    cleanup();
  });

  it('fr renders native French', async () => {
    await initI18n('fr');
    render(<Probe />);
    expect(screen.getByTestId('cancel').textContent).toBe('Annuler');
    expect(screen.getByTestId('save').textContent).toBe('Enregistrer');
    cleanup();
  });

  it('es renders native Spanish', async () => {
    await initI18n('es');
    render(<Probe />);
    expect(screen.getByTestId('cancel').textContent).toBe('Cancelar');
    expect(screen.getByTestId('save').textContent).toBe('Guardar');
    cleanup();
  });

  it('setLocale switches an already-initialized instance at runtime', async () => {
    await initI18n('en');
    render(<Probe />);
    expect(screen.getByTestId('cancel').textContent).toBe('Cancel');
    await act(async () => {
      await setLocale('de');
    });
    expect(screen.getByTestId('cancel').textContent).toBe('Abbrechen');
    expect(screen.getByTestId('save').textContent).toBe('Speichern');
    cleanup();
  });

  it('switching back to en restores English without re-init', async () => {
    await initI18n('de');
    render(<Probe />);
    expect(screen.getByTestId('cancel').textContent).toBe('Abbrechen');
    await act(async () => {
      await setLocale('en');
    });
    expect(screen.getByTestId('cancel').textContent).toBe('Cancel');
    cleanup();
  });
});
