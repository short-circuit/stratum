import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { ThemeProvider } from '@mui/material/styles'
import CssBaseline from '@mui/material/CssBaseline'
import '@fontsource/inter/300.css'
import '@fontsource/inter/400.css'
import '@fontsource/inter/500.css'
import '@fontsource/inter/600.css'
import '@fontsource/inter/700.css'
import App from './App'
import { applyTheme } from './lib/theme'
import { createMuiTheme } from './lib/muiTheme'
import * as api from './lib/commands'
import './global.css'

async function init() {
  let primary = '#f97316'
  let secondary = '#6b7280'
  let dark = true
  let fontSize = 16

  try {
    const settings = await api.getSettings();
    if (settings.theme) {
      primary = settings.theme.primary_color || primary
      secondary = settings.theme.secondary_color || secondary
      dark = settings.theme.dark_mode ?? dark
      fontSize = settings.theme.font_size || fontSize
      applyTheme(primary, secondary, dark, fontSize)
    }
  } catch {
    applyTheme(primary, secondary, dark)
  }

  const muiTheme = createMuiTheme(primary, secondary, dark, fontSize)

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <ThemeProvider theme={muiTheme}>
        <CssBaseline />
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </ThemeProvider>
    </React.StrictMode>,
  )
}

init()
