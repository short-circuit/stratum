import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import SettingsRow from './SettingsRow';
import SettingsTab from './SettingsTab';
import SettingsAccordion from './SettingsAccordion';
import ThemeColorEditor from './ThemeColorEditor';
import ThemeFontSizeSlider from './ThemeFontSizeSlider';

describe('SettingsRow', () => {
  it('toggle mode renders label and calls onCheckedChange on switch click', () => {
    const onCheckedChange = vi.fn();
    render(<SettingsRow label="Dark mode" checked={true} onCheckedChange={onCheckedChange} />);

    expect(screen.getByText('Dark mode')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('switch'));
    expect(onCheckedChange).toHaveBeenCalledWith(false);
  });

  it('generic mode renders the provided control and label', () => {
    render(<SettingsRow label="Remote" control={<button>Go</button>} />);

    expect(screen.getByText('Remote')).toBeInTheDocument();
    expect(screen.getByText('Go')).toBeInTheDocument();
  });
});

describe('SettingsTab', () => {
  it('renders tab labels and calls onChange with the value on click', () => {
    const onChange = vi.fn();
    render(
      <SettingsTab
        value="vault"
        onChange={onChange}
        tabs={[
          { value: 'vault', label: 'Vault' },
          { value: 'theme', label: 'Theme' },
        ]}
      />,
    );

    expect(screen.getByText('Vault')).toBeInTheDocument();
    expect(screen.getByText('Theme')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Theme'));
    expect(onChange).toHaveBeenCalledWith('theme');
  });
});

describe('SettingsAccordion', () => {
  it('renders summary and toggles onChange when the summary is clicked', () => {
    const onChange = vi.fn();
    render(
      <SettingsAccordion summary={<span>Configure</span>} expanded={false} onChange={onChange}>
        <div>Content</div>
      </SettingsAccordion>,
    );

    expect(screen.getByText('Configure')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Configure'));
    expect(onChange).toHaveBeenCalledWith(true);
  });
});

describe('ThemeColorEditor', () => {
  const presets = ['#f97316', '#ef4444', '#3b82f6'] as const;

  it('renders one swatch button per preset and calls onChange with the preset value', () => {
    const onChange = vi.fn();
    render(<ThemeColorEditor label="Primary" value="#f97316" presets={presets} onChange={onChange} />);

    const buttons = screen.getAllByRole('button');
    expect(buttons.length).toBe(presets.length);
    // Click the second preset (unselected) to assert the value is forwarded.
    fireEvent.click(buttons[1]);
    expect(onChange).toHaveBeenCalledWith('#ef4444');
  });

  it('gives the native color input the localized accessible name', () => {
    render(<ThemeColorEditor label="Primary" value="#f97316" presets={presets} onChange={vi.fn()} />);
    expect(screen.getByLabelText('Pick color')).toBeInTheDocument();
  });
});

describe('ThemeFontSizeSlider', () => {
  it('fires onChange when the value changes', () => {
    const onChange = vi.fn();
    render(<ThemeFontSizeSlider value={16} onChange={onChange} />);

    const slider = screen.getByRole('slider');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    expect(onChange).toHaveBeenCalled();
  });
});
