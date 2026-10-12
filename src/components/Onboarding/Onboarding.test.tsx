import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { beforeEach, afterEach, describe, it, expect, vi } from 'vitest';
import Onboarding from './index';
import { useStore } from '../../stores/appStore';
import { useOnboardingStore } from '../../stores/onboardingStore';

// Mock the Tauri command surface so the wizard can run in jsdom without a
// runtime. The Onboarding components and the onboardingStore both consume
// `../../lib/commands` (resolved through the barrel); mocking it here intercepts
// every install/mark call made by the wizard orchestration.
vi.mock('../../lib/commands', () => ({
  installStarterTemplates: vi.fn(),
  installStarterSample: vi.fn(),
  markOnboardingDone: vi.fn(),
  getOnboardingState: vi.fn(),
  setOnboardingPending: vi.fn(),
  pickVaultDirectory: vi.fn(),
}));

import * as api from '../../lib/commands';

const apiMock = api as unknown as Record<
  'installStarterTemplates' | 'installStarterSample' | 'markOnboardingDone' | 'pickVaultDirectory',
  ReturnType<typeof vi.fn>
>;

function LocationDisplay() {
  const loc = useLocation();
  return <div data-testid="location">{loc.pathname}</div>;
}

function renderWizard(onDone: () => void = vi.fn()) {
  return render(
    <MemoryRouter initialEntries={['/']}>
      <Onboarding onDone={onDone} />
      <LocationDisplay />
    </MemoryRouter>,
  );
}

describe('Onboarding wizard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // The wizard gate renders only once a vault is present; seed a healthy
    // vault and a pristine wizard store for each case.
    useStore.setState({
      vault: { path: '/home/user/vault', block_count: 0, page_count: 0 },
      pages: [],
      currentPage: null,
      loading: false,
      error: null,
      persistentError: null,
      themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
    });
    useOnboardingStore.getState().reset();
    apiMock.installStarterTemplates.mockResolvedValue([]);
    apiMock.installStarterSample.mockResolvedValue([]);
    apiMock.markOnboardingDone.mockResolvedValue(undefined);
    apiMock.pickVaultDirectory.mockResolvedValue({ path: '/home/user/vault', block_count: 0, page_count: 0 });
  });

  afterEach(() => {
    useStore.setState({
      vault: null,
      pages: [],
      currentPage: null,
      loading: false,
      error: null,
      persistentError: null,
      themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
    });
  });

  it('renders the welcome step on first load', () => {
    renderWizard();
    expect(screen.getByText('Welcome to Stratum')).toBeInTheDocument();
    expect(screen.getByText('Set up your first vault and start capturing ideas.')).toBeInTheDocument();
  });

  it('advances a step when Next is clicked', () => {
    renderWizard();
    fireEvent.click(screen.getByText('Next'));
    expect(screen.getByText('Step 2 of 4')).toBeInTheDocument();
    // Back recedes to the welcome step.
    fireEvent.click(screen.getByText('Back'));
    expect(screen.getByText('Step 1 of 4')).toBeInTheDocument();
  });

  it('toggles the sample-vault option on the sample step', () => {
    renderWizard();
    fireEvent.click(screen.getByText('Next')); // welcome -> templates
    fireEvent.click(screen.getByText('Next')); // templates -> sample
    const checkbox = screen.getByRole('checkbox') as HTMLInputElement;
    expect(checkbox.checked).toBe(false);
    fireEvent.click(checkbox);
    expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(true);
  });

  it('runs the install orchestration when leaving the sample step', async () => {
    // Enable the sample-vault toggle first so installStarterSample is invoked.
    useOnboardingStore.setState({ sampleVault: true });
    renderWizard();
    fireEvent.click(screen.getByText('Next')); // welcome -> templates
    fireEvent.click(screen.getByText('Next')); // templates -> sample
    fireEvent.click(screen.getByText('Next')); // sample -> finish (triggers install)

    await waitFor(() => {
      expect(apiMock.installStarterTemplates).toHaveBeenCalled();
    });
    expect(apiMock.installStarterSample).toHaveBeenCalled();
    expect(apiMock.markOnboardingDone).toHaveBeenCalled();
    expect(useOnboardingStore.getState().done).toBe(true);
  });

  it('finishes the wizard and navigates to the journal on Get Started', async () => {
    const onDone = vi.fn();
    renderWizard(onDone);
    fireEvent.click(screen.getByText('Next')); // welcome -> templates
    fireEvent.click(screen.getByText('Next')); // templates -> sample
    fireEvent.click(screen.getByText('Next')); // sample -> finish

    await waitFor(() => expect(screen.getByText("You're all set")).toBeInTheDocument());

    await act(async () => {
      fireEvent.click(screen.getByText('Get Started'));
    });

    expect(onDone).toHaveBeenCalled();
    expect(screen.getByTestId('location').textContent).toBe('/journal');
  });
});
