import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import App from './App'
import { applyTheme } from './lib/theme'
import * as api from './lib/commands'
import './global.css'

// Load settings and apply theme before render
async function init() {
  try {
    const settings = await api.getSettings();
    if (settings.theme) {
      applyTheme(
        settings.theme.primary_color || '#f97316',
        settings.theme.secondary_color || '#6b7280',
        settings.theme.dark_mode,
      );
    }
  } catch {
    applyTheme('#f97316', '#6b7280', true);
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
