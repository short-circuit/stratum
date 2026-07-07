import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import '@fontsource/inter/300.css'
import '@fontsource/inter/400.css'
import '@fontsource/inter/500.css'
import '@fontsource/inter/600.css'
import '@fontsource/inter/700.css'
import App from './App'
import { applyTheme } from './lib/theme'
import { useStore } from './stores/appStore'
import { useSyncStore } from './stores/syncStore'
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

  useStore.getState().setThemeConfig({ primaryHex: primary, secondaryHex: secondary, dark, fontSize })

  // Initialize sync status on startup so it's available globally
  useSyncStore.getState().fetchSyncStatus();

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </React.StrictMode>,
  )
}

init()
