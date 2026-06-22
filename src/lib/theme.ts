// Primary + secondary color theming.
// Generates CSS custom properties for both colors with full shade derivation.

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

function setShades(prefix: string, hex: string) {
  const { h, s } = hexToHsl(hex);
  const root = document.documentElement;
  const shades: [number, number][] = [
    [50, 96], [100, 92], [200, 84], [300, 68],
    [400, 55], [500, 50], [600, 42], [700, 34],
    [800, 26], [900, 16],
  ];
  for (const [n, l] of shades) {
    root.style.setProperty(`--${prefix}-${n}`, `hsl(${h}, ${s}%, ${l}%)`);
  }
}

export function applyTheme(primaryHex: string, secondaryHex: string, dark: boolean) {
  setShades('primary', primaryHex);
  setShades('secondary', secondaryHex);

  const root = document.documentElement;

  // Semantic aliases for primary
  root.style.setProperty('--primary-bg', `var(--primary-${dark ? '900' : '50'})`);
  root.style.setProperty('--primary-bg-hover', `var(--primary-${dark ? '800' : '100'})`);
  root.style.setProperty('--primary-text', `var(--primary-${dark ? '300' : '600'})`);
  root.style.setProperty('--primary-border', `var(--primary-${dark ? '800' : '200'})`);

  // Dark mode class
  if (dark) {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }
}
