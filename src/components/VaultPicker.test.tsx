import { render, screen, fireEvent } from '@testing-library/react';
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { useStore } from '../stores/appStore';
import VaultPicker from './VaultPicker';

// VaultPicker reads the app-level error state directly. Rehydrate the store to
// a clean pre-vault state for each case so tests never leak error state into
// each other.
const cleanState = {
  vault: null,
  pages: [],
  currentPage: null,
  loading: false,
  error: null,
  persistentError: null,
  themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
};

describe('VaultPicker error dismissal (render-surface regression)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useStore.setState(cleanState);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders a transient error with a dismiss control that clears it', () => {
    useStore.getState().showError('bad vault');

    render(<VaultPicker />);
    expect(screen.getByText('bad vault')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Dismiss error'));
    expect(useStore.getState().error).toBeNull();
    expect(screen.queryByText('bad vault')).not.toBeInTheDocument();
  });

  it('renders a persistent error that stays until manually dismissed', () => {
    useStore.getState().showError('sync conflict', { persistent: true });

    render(<VaultPicker />);
    expect(screen.getByText('sync conflict')).toBeInTheDocument();

    // Persistent errors never auto-dismiss.
    vi.advanceTimersByTime(60_000);
    expect(screen.getByText('sync conflict')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Dismiss error'));
    expect(useStore.getState().persistentError).toBeNull();
    expect(screen.queryByText('sync conflict')).not.toBeInTheDocument();
  });

  it('dismissing the shown error leaves unrelated store state intact', () => {
    useStore.getState().showError('transient noise');
    useStore.setState({ loading: false, pages: [{ path: 'a.md', slug: 'a', title: 'A', block_count: 0, modified_at: '2026-01-01' }] });
    const { rerender } = render(<VaultPicker />);

    fireEvent.click(screen.getByLabelText('Dismiss error'));

    rerender(<VaultPicker />);
    expect(useStore.getState().error).toBeNull();
    expect(useStore.getState().persistentError).toBeNull();
    // Dismissing the error must not wipe unrelated store state.
    expect(useStore.getState().pages).toEqual([{ path: 'a.md', slug: 'a', title: 'A', block_count: 0, modified_at: '2026-01-01' }]);
    expect(useStore.getState().loading).toBe(false);
  });
});
