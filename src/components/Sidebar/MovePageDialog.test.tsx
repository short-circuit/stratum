import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import MovePageDialog from './MovePageDialog';
import type { PageDto } from '../../lib/types';

const PAGE_A: PageDto = {
  path: 'pages/a.md',
  slug: 'a',
  title: 'Alpha',
  block_count: 2,
  modified_at: '2026-09-22T10:00:00Z',
};
const PAGE_B: PageDto = {
  path: 'pages/b.md',
  slug: 'b',
  title: null,
  block_count: 0,
  modified_at: '2026-09-22T11:00:00Z',
};

function makeDeferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => { resolve = res; });
  return { promise, resolve };
}

const pathInput = () => screen.getByLabelText('New path') as HTMLInputElement;

function renderDialog(
  overrides: Partial<Parameters<typeof MovePageDialog>[0]> = {},
) {
  const props = {
    open: true,
    page: PAGE_A,
    pages: [PAGE_A, PAGE_B],
    onClose: vi.fn(),
    onSubmit: vi.fn(() => Promise.resolve(true)),
    ...overrides,
  };
  render(<MovePageDialog {...props} />);
  return props;
}

describe('MovePageDialog', () => {
  it('opens prefilled with the current path in the New path field', async () => {
    renderDialog();
    await waitFor(() => expect(pathInput().value).toBe('pages/a.md'));
  });

  it('submits onSubmit with the old path and a normalized new path', async () => {
    const props = renderDialog();
    fireEvent.change(pathInput(), { target: { value: 'pages/c' } });
    fireEvent.click(screen.getByText('Move'));
    await waitFor(() =>
      expect(props.onSubmit).toHaveBeenCalledWith('pages/a.md', 'pages/c.md'),
    );
  });

  it('does not double the .md extension when the user types it explicitly', async () => {
    const props = renderDialog();
    fireEvent.change(pathInput(), { target: { value: 'pages/c.md' } });
    fireEvent.click(screen.getByText('Move'));
    await waitFor(() =>
      expect(props.onSubmit).toHaveBeenCalledWith('pages/a.md', 'pages/c.md'),
    );
  });

  it('disables the Move button while the submit is pending', async () => {
    const deferred = makeDeferred<boolean>();
    const props = renderDialog({ onSubmit: vi.fn(() => deferred.promise) });
    fireEvent.change(pathInput(), { target: { value: 'pages/c.md' } });
    fireEvent.click(screen.getByText('Move'));
    const moveBtn = screen.getByText('Move').closest('button')!;
    await waitFor(() => expect(moveBtn.disabled).toBe(true));
    expect(props.onSubmit).toHaveBeenCalledTimes(1);
    deferred.resolve(true);
    await waitFor(() => expect(moveBtn.disabled).toBe(false));
  });

  it('calls onClose when the submit resolves to true', async () => {
    const props = renderDialog();
    fireEvent.change(pathInput(), { target: { value: 'pages/c.md' } });
    fireEvent.click(screen.getByText('Move'));
    await waitFor(() => expect(props.onClose).toHaveBeenCalled());
  });

  it('does not call onClose when the submit resolves to false', async () => {
    const props = renderDialog({ onSubmit: vi.fn(() => Promise.resolve(false)) });
    fireEvent.change(pathInput(), { target: { value: 'pages/c.md' } });
    fireEvent.click(screen.getByText('Move'));
    await waitFor(() => expect(props.onSubmit).toHaveBeenCalled());
    expect(props.onClose).not.toHaveBeenCalled();
  });

  it('cancel calls onClose and does not call onSubmit', () => {
    const props = renderDialog();
    fireEvent.click(screen.getByText('Cancel'));
    expect(props.onClose).toHaveBeenCalled();
    expect(props.onSubmit).not.toHaveBeenCalled();
  });

  it('shows an error for an empty path and does not submit', () => {
    const props = renderDialog();
    fireEvent.change(pathInput(), { target: { value: '' } });
    fireEvent.click(screen.getByText('Move'));
    expect(screen.getByText('Path is required.')).toBeInTheDocument();
    expect(props.onSubmit).not.toHaveBeenCalled();
  });

  it('shows an error for a path not under pages/ and does not submit', () => {
    const props = renderDialog();
    fireEvent.change(pathInput(), { target: { value: 'foo.md' } });
    fireEvent.click(screen.getByText('Move'));
    expect(
      screen.getByText('Path must start with "pages/" (e.g. pages/my-note.md).'),
    ).toBeInTheDocument();
    expect(props.onSubmit).not.toHaveBeenCalled();
  });

  it('shows an error when the target already exists and does not submit', () => {
    const props = renderDialog();
    fireEvent.change(pathInput(), { target: { value: 'pages/b.md' } });
    fireEvent.click(screen.getByText('Move'));
    expect(screen.getByText('A page already exists at "pages/b.md".')).toBeInTheDocument();
    expect(props.onSubmit).not.toHaveBeenCalled();
  });
});
