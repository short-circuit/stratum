# Settings mobile screenshot capture

Headless before/after capture for the mobile Settings parity work
(`docs` PR requirement: "PR includes before/after screenshots").

## How it works

- Builds a production `dist` (must exist; `npm run build`).
- Serves it via `npm run preview` (default `http://localhost:4173`).
- Drives the app with Playwright + Chromium at realistic mobile viewports
  (360x740 and 428x900) with a mobile user-agent so the app selects the
  mobile flow.
- Injects a self-contained mock of the Tauri IPC surface (see `mockInitScript`)
  so the Settings screen can mount without the Rust runtime.

## Usage

```bash
npm run build
npm run preview &   # keep running
node e2e/capture/capture-settings.mjs .screenshots/before before   # baseline
# ... make changes, rebuild ...
node e2e/capture/capture-settings.mjs .screenshots/after after     # after
```

Captured images per viewport:
- `settings-<phase>-<size>-WxH.png` — full-page settings
- `...-ai-...` — AI accordion expanded + models fetched (capability editor,
  RAG, embedding)
- `...-stt-...` — Speech & Audio / STT-TTS expanded
- `...-sync-...` — scrolled to the Sync section (mode, remote, SSH key,
  commit template, controls, commit log)

## Notes

- The capture relies on the app's `useResponsive` gate, which reads the UA
  (android/ios) / touch support. The mobile UA + `hasTouch`/`isMobile` in the
  context force the mobile path.
- The mock only implements commands the Settings screen calls; anything else
  resolves to `null` with a console warning.
- Requires installed browsers: `npx playwright install chromium`.
