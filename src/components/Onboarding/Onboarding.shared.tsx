import { useCallback } from 'react';
import { useShallow } from 'zustand/react/shallow';
import {
  useOnboardingStore,
  STARTER_TEMPLATE_NAMES,
  type OnboardingState,
} from '../../stores/onboardingStore';

//! Shared onboarding logic, types, and step definitions.
//!
//! Both the desktop and mobile variants consume the `useOnboarding()` hook so
//! the wizard behaves identically regardless of platform. All UI copy is
//! localized via `t()` in the render components; nothing here hardcodes
//! user-facing text except the template file stems (which are literal file
//! names, not translatable content).

/// The ordered list of wizard steps.
export const STEPS = ['welcome', 'templates', 'sample', 'finish'] as const;

export type OnboardingStep = (typeof STEPS)[number];

/// Names of the bundled starter templates, rendered verbatim (file stems).
export const STARTER_TEMPLATES = STARTER_TEMPLATE_NAMES.map((name) => ({ name }));

export interface UseOnboardingResult extends OnboardingState {
  /** Advance the wizard, running the install orchestration before the finish step. */
  handleNext: () => Promise<void>;
}

export function useOnboarding(): UseOnboardingResult {
  const state = useOnboardingStore(
    useShallow((s) => ({
      step: s.step,
      startWithTemplates: s.startWithTemplates,
      selectedTemplates: s.selectedTemplates,
      sampleVault: s.sampleVault,
      installing: s.installing,
      error: s.error,
      done: s.done,
      next: s.next,
      back: s.back,
      toggleTemplates: s.toggleTemplates,
      toggleTemplate: s.toggleTemplate,
      setSampleVault: s.setSampleVault,
      runInstall: s.runInstall,
      reset: s.reset,
    })),
  );

  // Leaving the sample step (moving into the finish step) triggers the install
  // orchestration. On success it marks the wizard done; on failure it stores the
  // error and keeps us on the current step so the user can retry Next.
  const { step, runInstall, next } = state;
  const handleNext = useCallback(async () => {
    if (step === STEPS.length - 2) {
      await runInstall();
      if (useOnboardingStore.getState().error) return;
    }
    next();
  }, [step, runInstall, next]);

  return { ...state, handleNext };
}
