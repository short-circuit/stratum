import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import ThemeTab from './ThemeTab';

const theme = {
  dark_mode: true,
  primary_color: '#f97316',
  secondary_color: '#6b7280',
  font_size: 16,
};

describe('ThemeTab language selector', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the language selector with 8 native-language options', () => {
    render(
      <ThemeTab
        theme={theme}
        onThemeChange={vi.fn()}
        locale="en"
        onLocaleChange={vi.fn()}
      />,
    );

    // The Interface-language label must render as translated text, not a raw key.
    expect(screen.getAllByText('Interface language').length).toBeGreaterThan(0);

    fireEvent.mouseDown(screen.getByRole('combobox'));
    const names = ['English', 'Deutsch', 'Italiano', 'Français', 'Español', '中文', '日本語', 'Ελληνικά'];
    names.forEach(name => {
      expect(screen.getAllByText(name).length).toBeGreaterThan(0);
    });
  });

  it('calls onLocaleChange with the selected language code', () => {
    const onLocaleChange = vi.fn();
    render(
      <ThemeTab
        theme={theme}
        onThemeChange={vi.fn()}
        locale="en"
        onLocaleChange={onLocaleChange}
      />,
    );

    fireEvent.mouseDown(screen.getByRole('combobox'));
    fireEvent.click(screen.getByText('Deutsch'));

    expect(onLocaleChange).toHaveBeenCalledWith('de');
  });
});
