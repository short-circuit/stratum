import { create } from 'zustand';
import * as api from '../lib/commands';
import i18n from '../i18n';

//! First-run onboarding wizard state.
//!
//! Owns the wizard step, the "start with templates" / "create a sample vault"
//! toggles, the selected template names, and the async install orchestration
//! that runs when the user leaves the sample step. All error copy is rendered
//! through i18n (`t`), never hardcoded, so the i18n gate stays green.

/// File stems of the 12 bundled starter templates (matches `pkm-core`).
export const STARTER_TEMPLATE_NAMES = [
  'daily-note',
  'meeting-notes',
  'reading-list',
  'book-notes',
  'project-tracker',
  'research-notes',
  'flashcards-starter',
  'kanban-starter',
  'whiteboard-starter',
  'habit-tracker',
  'quick-notes',
  'demo-graph',
];

/// Total number of wizard steps (welcome, templates, sample, finish).
export const ONBOARDING_STEPS = 4;

export interface OnboardingState {
  step: number;
  startWithTemplates: boolean;
  selectedTemplates: string[];
  sampleVault: boolean;
  installing: boolean;
  error: string | null;
  done: boolean;
  next: () => void;
  back: () => void;
  toggleTemplates: (v: boolean) => void;
  toggleTemplate: (name: string) => void;
  setSampleVault: (v: boolean) => void;
  runInstall: () => Promise<void>;
  reset: () => void;
}

export const useOnboardingStore = create<OnboardingState>((set, get) => ({
  step: 0,
  startWithTemplates: true,
  selectedTemplates: [...STARTER_TEMPLATE_NAMES],
  sampleVault: false,
  installing: false,
  error: null,
  done: false,

  next: () => set((s) => ({ step: Math.min(s.step + 1, ONBOARDING_STEPS - 1) })),
  back: () => set((s) => ({ step: Math.max(s.step - 1, 0) })),

  toggleTemplates: (v) => set({ startWithTemplates: v }),
  toggleTemplate: (name) =>
    set((s) => ({
      selectedTemplates: s.selectedTemplates.includes(name)
        ? s.selectedTemplates.filter((n) => n !== name)
        : [...s.selectedTemplates, name],
    })),
  setSampleVault: (v) => set({ sampleVault: v }),

  runInstall: async () => {
    set({ installing: true, error: null });
    const { startWithTemplates, selectedTemplates, sampleVault } = get();
    try {
      if (startWithTemplates) {
        const names =
          selectedTemplates.length > 0 ? selectedTemplates : STARTER_TEMPLATE_NAMES;
        await api.installStarterTemplates(names);
      }
      if (sampleVault) {
        await api.installStarterSample();
      }
      await api.markOnboardingDone();
      set({ installing: false, done: true });
    } catch (e) {
      set({ installing: false, error: i18n.t('onboarding.error', { message: String(e) }) });
    }
  },

  reset: () =>
    set({
      step: 0,
      startWithTemplates: true,
      selectedTemplates: [...STARTER_TEMPLATE_NAMES],
      sampleVault: false,
      installing: false,
      error: null,
      done: false,
    }),
}));
