import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ResearchTab from './ResearchTab';
import * as api from '../../lib/commands';

vi.mock('../../lib/commands', () => ({
  pingSearchInstance: vi.fn(),
}));

const PRESET_URLS = [
  'https://searx.be',
  'https://search.bus-hit.me',
  'https://priv.au',
  'https://searx.tiekoetter.com',
];

function makeProps(overrides: any = {}) {
  return {
    research: { searxng_endpoint: 'http://localhost:8888', max_results: 3, max_depth: 2 },
    onResearchChange: vi.fn(),
    ...overrides,
  };
}

describe('ResearchTab — public SearXNG presets', () => {
  beforeEach(() => {
    (api.pingSearchInstance as ReturnType<typeof vi.fn>).mockReset();
  });

  it('renders all four preset URLs and reports a reachable instance', async () => {
    (api.pingSearchInstance as ReturnType<typeof vi.fn>).mockResolvedValue({
      ok: true,
      latency_ms: 120,
      error: null,
    });
    render(<ResearchTab {...makeProps()} />);

    for (const url of PRESET_URLS) {
      expect(screen.getByText(url)).toBeInTheDocument();
    }

    fireEvent.click(screen.getAllByText('Test instance')[0]);
    await expect(screen.findByText('Reachable — 120ms')).resolves.toBeInTheDocument();
  });

  it('reports an instance as unreachable when the ping fails', async () => {
    (api.pingSearchInstance as ReturnType<typeof vi.fn>).mockResolvedValue({
      ok: false,
      latency_ms: 0,
      error: null,
    });
    render(<ResearchTab {...makeProps()} />);

    fireEvent.click(screen.getAllByText('Test instance')[0]);
    await expect(screen.findByText('Unreachable')).resolves.toBeInTheDocument();
  });

  it('use this instance updates the searxng endpoint', () => {
    const props = makeProps();
    render(<ResearchTab {...props} />);

    fireEvent.click(screen.getAllByText('Use this instance')[0]);
    expect(props.onResearchChange).toHaveBeenCalledWith({ searxng_endpoint: 'https://searx.be' });
  });
});
