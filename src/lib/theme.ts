// Accent color theming utilities.
// Applies CSS custom properties derived from a hex accent color.

type HSL = { h: number; s: number; l: number };

function hexToHsl(hex: string): HSL {
  let r = 0, g = 0, b = 0;
  const clean = hex.replace('#', '');
  if (clean.length === 3) {
    r = parseInt(clean[0] + clean[0], 16);
    g = parseInt(clean[1] + clean[1], 16);
    b = parseInt(clean[2] + clean[2], 16);
  } else if (clean.length === 6) {
    r = parseInt(clean.substring(0, 2), 16);
    g = parseInt(clean.substring(2, 4), 16);
    b = parseInt(clean.substring(4, 6), 16);
  }
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  let h = 0, s = 0;
  const l = (max + min) / 2;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
      case g: h = ((b - r) / d + 2) / 6; break;
      case b: h = ((r - g) / d + 4) / 6; break;
    }
  }
  return { h: Math.round(h * 360), s: Math.round(s * 100), l: Math.round(l * 100) };
}

export function applyAccentTheme(hex: string, dark: boolean) {
  const { h, s } = hexToHsl(hex);
  const root = document.documentElement;

  // Light mode shades
  root.style.setProperty('--accent', hex);
  root.style.setProperty('--accent-50', `hsl(${h}, ${s}%, 96%)`);
  root.style.setProperty('--accent-100', `hsl(${h}, ${s}%, 92%)`);
  root.style.setProperty('--accent-200', `hsl(${h}, ${s}%, 84%)`);
  root.style.setProperty('--accent-300', `hsl(${h}, ${s}%, 68%)`);
  root.style.setProperty('--accent-400', `hsl(${h}, ${s}%, 55%)`);
  root.style.setProperty('--accent-500', `hsl(${h}, ${s}%, 50%)`);
  root.style.setProperty('--accent-600', `hsl(${h}, ${s}%, 42%)`);
  root.style.setProperty('--accent-700', `hsl(${h}, ${s}%, 34%)`);
  root.style.setProperty('--accent-800', `hsl(${h}, ${s}%, 26%)`);
  root.style.setProperty('--accent-900', `hsl(${h}, ${s}%, 16%)`);

  // Common semantic colors
  root.style.setProperty('--accent-bg', `var(--accent-${dark ? '900' : '50'})`);
  root.style.setProperty('--accent-bg-hover', `var(--accent-${dark ? '800' : '100'})`);
  root.style.setProperty('--accent-text', `var(--accent-${dark ? '300' : '700'})`);
  root.style.setProperty('--accent-border', `var(--accent-${dark ? '800' : '200'})`);

  // Dark mode class
  if (dark) {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }
}
