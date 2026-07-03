import { createTheme, type Theme } from '@mui/material/styles';

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

function hslStr(h: number, s: number, l: number): string {
  return `hsl(${h}, ${s}%, ${l}%)`;
}

export function createMuiTheme(
  primaryHex: string,
  secondaryHex: string,
  dark: boolean,
  fontSize?: number,
): Theme {
  const baseSize = (fontSize && fontSize > 0) ? fontSize : 16;
  const { h: sh, s: ss } = hexToHsl(secondaryHex);

  // Generate background/surface/divider colors from the secondary hue
  let backgroundDefault: string;
  let backgroundPaper: string;
  let dividerColor: string;

  if (dark) {
    backgroundDefault = hslStr(sh, Math.min(ss, 25), 7);
    backgroundPaper = hslStr(sh, Math.min(ss, 20), 12);
    dividerColor = hslStr(sh, Math.min(ss, 18), 22);
  } else {
    backgroundDefault = hslStr(sh, Math.min(ss, 20), 94);
    backgroundPaper = '#ffffff';
    dividerColor = hslStr(sh, Math.min(ss, 18), 82);
  }

  return createTheme({
    palette: {
      mode: dark ? 'dark' : 'light',
      primary: {
        main: primaryHex,
      },
      secondary: {
        main: secondaryHex,
      },
      background: {
        default: backgroundDefault,
        paper: backgroundPaper,
      },
      divider: dividerColor,
      text: {
        primary: dark ? hslStr(sh, 10, 92) : hslStr(sh, 12, 18),
        secondary: dark ? hslStr(sh, 8, 68) : hslStr(sh, 10, 48),
        disabled: dark ? hslStr(sh, 6, 38) : hslStr(sh, 8, 68),
      },
    },
    typography: {
      htmlFontSize: baseSize,
      fontFamily: '"Inter", "Roboto", "Helvetica", "Arial", sans-serif',
    },
    shape: {
      borderRadius: 8,
    },
    components: {
      MuiButton: {
        styleOverrides: {
          root: {
            textTransform: 'none',
            fontWeight: 500,
          },
        },
      },
      MuiDrawer: {
        styleOverrides: {
          paper: {
            borderRight: '1px solid',
            borderColor: 'divider',
          },
        },
      },
      MuiPopover: {
        styleOverrides: {
          paper: {
            borderRadius: 8,
          },
        },
      },
      MuiDialog: {
        styleOverrides: {
          paper: {
            borderRadius: 12,
          },
        },
      },
      MuiCssBaseline: {
        styleOverrides: {
          body: {
            overflow: 'hidden',
          },
          '#root': {
            width: '100vw',
            height: '100vh',
          },
        },
      },
    },
  });
}
