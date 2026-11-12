# Settings

The **Settings** panel is the single place to configure Stratum: vault location,
appearance, AI providers, web research, sync and backup, tray behaviour, and
block reminders. The native shell reproduces the shipped eight-tab page, so the
same fields and validation apply whether you run the React desktop app or the
native GPUI shell.

Settings are persisted in `.pkm/config.toml` inside your vault. The panel writes
through the same service layer as the TOML file, so a change made here and a
hand edit of the config converge on the same values.

> [!tip]
> Every field is validated before it is written. If a value is rejected the
> save is blocked, the offending inputs are marked, and a localized explanation
> appears under each field. Nothing is silently replaced with a default.

## Vault

- **Vault path** — the directory Stratum treats as your knowledge base. Using
  **Browse** switches the active vault to the picked directory immediately and
  reloads the settings for that vault.
- **Pages layout** — flat list or folder tree in the sidebar.
- **Import** — pick an Obsidian, Logseq or generic Markdown source folder,
  preview the detected pages/assets, then import into
  `imported/<source-name>/`. Nothing in the source folder is modified.
- **Export** — HTML, JSON, per-page PDF and portable vault ZIP rows. Each row
  writes to the path shown in the field; the PDF row is disabled when no page
  is open.

## Theme

- **Dark mode**, **primary colour** and **secondary colour** apply live while
  you edit, so you can preview a palette before saving.
- Preset swatches cover the shipped accent pairs; the hex field accepts
  `#rgb` and `#rrggbb` values.
- **Font size** accepts 12–28 px.
- **Interface language** switches the UI catalog immediately and persists the
  choice.

An invalid or half-typed hex value keeps the last valid palette on screen and
blocks the save instead of flashing a fallback theme.

## AI

- **Provider** — Ollama, OpenAI, Anthropic, Google AI, Z.AI or a custom
  OpenAI-compatible endpoint.
- **Endpoint**, **API key**, **model** and **model capabilities** — the model
  list can be refreshed from the configured endpoint, and each model can be
  tagged for chat or embedding use.
- **RAG** — enable retrieval, choose 1–20 context chunks, and set embedding
  dimensions (`0` infers the size from the embedding model).
- **Local endpoint discovery** — probe Ollama, LM Studio, LocalAI and any extra
  host:port entries, then adopt a discovered endpoint.
- **Speech-to-Text** — transcription endpoint, key, model, diarization model,
  language hint, diarization and auto-summarize flags.
- **Text-to-Speech** — endpoint, key, voice, format, playback speed
  (0.25–4.0) and a test-play button.
- **Voice capture** — enable the one-tap button and choose its insertion
  target.

API keys loaded from environment variables are shown as set but not revealed;
the panel explains which variable supplies the key.

## Research

- **SearXNG endpoint** — the instance used by web research.
- **Max results** (1–10) and **research depth** (1–5).
- **Presets** — public SearXNG instances with a per-instance ping, so you can
  test availability before adopting one.

## Developer

- **Reindex all** — re-sync every page from disk into the database.
- **Repair DB from disk** — reconcile the database with the files on disk.
- **Normalize all files** — run every Markdown file through the block parser and
  re-serialize it.
- **Reset onboarding** — show the first-run wizard again on next launch.
- **Install starter templates** — add the bundled template set to this vault.
- **Recent commits** — the expandable commit log lives under Developer; expand it
  to inspect hashes, authors, messages and timestamps.

## Sync

- **Sync mode** — manual, auto-commit, auto-sync or background.
- **Remote, branch and SSH key path** — the SSH key can be supplied here or left
  empty to use the SSH agent.
- **Commit interval** — at least 30 seconds.
- **Pull/push interval** — at least 60 seconds.
- **Commit message template** — must include `{datetime}`; placeholders for
  edited/new/deleted files and the total count are offered inline.
- **Sync now** — manual sync with an inline SSH passphrase prompt when the key
  needs one.
- **Conflicts** — when a merge leaves conflicted files, the tab lists each path
  with **Accept** (stage and commit that file), **Resolve All** (accept every
  listed file) and **Abort Merge** (restore the index to HEAD).
- **Cloud/folder backup** — enable a mirror copy, choose WebDAV, folder or both,
  set the interval (at least 1 minute), exclude list, WebDAV credentials or
  backup folder, test the connection and run a backup on demand. Backup is a
  mirror copy; Git sync is unaffected.

## Tray

- Enable the tray icon and **close to tray**.
- Set the quick-capture shortcut.
- Choose whether quick capture targets today's journal or a specific inbox page.

## Notifications

- Enable block reminders and the notification sound. The reminder scheduler runs
  inside the app while Stratum is open; see
  [Reminders](reminders.md) for the block-level workflow.

## Validation reference

| Field group | Rule |
|-------------|------|
| Colours | `#rgb` or `#rrggbb` |
| Endpoints | full `http://` or `https://` URL |
| Required fields | vault path, branch, shortcut, inbox path (when the target is the inbox), TTS voice/format, voice target |
| Ranges | RAG chunks 1–20, embedding dimensions ≤ 4096, research results 1–10, depth 1–5, TTS speed 0.25–4.0, font size 12–28 |
| Intervals | commit ≥ 30 s, sync ≥ 60 s, backup ≥ 1 min |
| Commit template | must contain `{datetime}` |
| Export paths | non-empty |

The same rules apply to the native shell capture states and are covered by the
settings contract tests.
