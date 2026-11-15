# Changelog

All notable changes to Stratum are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Release notes for
published versions are also available on the
[GitHub Releases page](https://github.com/short-circuit/stratum/releases).

## [2.0.0] — unreleased

The native cutover. The shipped application is now a single Rust/gpui shell;
the React + Tauri frontend was deleted once the native surfaces reached parity.

### Added

- **Native gpui desktop shell** — sidebar, router, shell chrome and all twelve
  navigation surfaces (`crates/stratum-ui`).
- **`stratum-editor`** — headless block-editor engine plus the native editor
  view: WYSIWYM staging, slash actions, table blocks, block drag-and-drop,
  wiki-link autocomplete, attachments (thumbnails, drag-and-drop, paste upload)
  and read-aloud.
- **`stratum-graph`** — deterministic force simulation, link curvature, camera
  fit and a headless 3D renderer behind the native graph panel.
- **`stratum-canvas`** — Excalidraw-compatible board engine with inline text
  editing, groups, locking, z-order and nudge, behind the native whiteboards.
- **`stratum-platform`** — native OS glue (tray, quick capture, notifications,
  global shortcut) replacing the seven Tauri plugins.
- **`stratum-mobile`** — gpui Android host: activity, safe areas, system back
  gesture and the Gradle APK/AAB project.
- **`stratum-i18n`** — eight locale catalogs (de, el, en, es, fr, it, ja, zh)
  with a catalog-to-code coverage gate.
- Journal calendar, stacking, read mode and voice capture in the native shell.
- Native settings surface with live theme application.
- Packaging for every shipped format — deb, rpm, AppImage, MSI, NSIS, macOS
  bundles and Android APK/AAB — with the `gpui-release.yml` workflow.
- Parity harness: golden screenshots, performance budgets and an accessibility
  probe, replacing the Playwright e2e suite.

### Changed

- One CI pipeline (`ci.yml`) guards the native shell: repo hygiene, workspace
  fmt/clippy/test, Windows and macOS shell builds, parity, Android build and
  smoke, MCP coverage and the security audit.
- Project version is now 2.0.0 (workspace `version`, Android `versionName`).

### Removed

- The React/TypeScript frontend (`src/`), the Tauri shell (`src-tauri/`), the
  Playwright harness (`e2e/`, `verification/cdp_probe.mjs`) and the npm
  toolchain (`package.json`, lockfile, `tsconfig*`, Vite/Vitest/ESLint/Knip
  configs). The frozen web build remains available as the `v1.0.0` tag.

## [1.0.0] — 2026-11-08

- First stable release: React/Tauri desktop application, Android build, deb /
  rpm / AppImage / MSI / macOS packaging, and the published documentation site.
