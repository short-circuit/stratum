import { createTheme, type Theme } from '@mui/material/styles';

export function createMuiTheme(
  primaryHex: string,
  secondaryHex: string,
  dark: boolean,
  fontSize?: number,
): Theme {
  const baseSize = (fontSize && fontSize > 0) ? fontSize : 16;

  return createTheme({
    palette: {
      mode: dark ? 'dark' : 'light',
      primary: {
        main: primaryHex,
      },
      secondary: {
        main: secondaryHex,
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
