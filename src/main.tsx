import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import App from './App'
import { applyAccentTheme } from './lib/theme'
import * as api from './lib/commands'
import './global.css'

// Load settings and apply theme before render
async function init() {
  try {
    const settings = await api.getSettings();
    if (settings.theme) {
      applyAccentTheme(settings.theme.accent_color || '#f97316', settings.theme.dark_mode);
    }
  } catch {
    // Use defaults
    applyAccentTheme('#f97316', true);
  }

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </React.StrictMode>,
  )
}

init()
