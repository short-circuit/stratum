# Contributing to Stratum

Stratum is AGPL-3.0-only and community-driven. Contributions of any size are
welcome — bug reports, documentation fixes, translations, tests, and new
features.

By contributing, you agree that your contributions are licensed under the
same [AGPL-3.0-only](https://github.com/short-circuit/stratum/blob/master/LICENSE)
license as the project.

## Before you start

- **Check the kanban board** — most work is tracked as task-scoped cards
  (`t_<task-id>`). Look for open cards before starting something big.
- **Check existing issues** — a bug may already be reported or in progress.
- **Run the tests** — see [Testing](#testing) below. They must pass before
  you open a PR.

## Getting started

### Fork + clone

```bash
# 1. Fork https://github.com/short-circuit/stratum (GitHub → Fork)
# 2. Clone your fork and add upstream:
git clone git@github.com:<your-username>/stratum.git
cd stratum
git remote add upstream https://github.com/short-circuit/stratum.git

# 3. Always branch off the latest master:
git fetch upstream
git checkout -b feat/my-feature upstream/master
```

Note: this repository works from **worktrees**. If you are an existing
contributor, task-scoped branches are usually created as Git worktrees
(`.worktrees/t_<task-id>`); follow the current convention of the board.

### Development environment

**Nix (recommended)**

```bash
nix develop ./nix          # enter dev shell with all dependencies
direnv allow               # or auto-activate via direnv
```

**Manual setup** requires: Rust 1.75+ (MSRV), Node.js 22+, and Tauri v2
system libraries:

- Linux: `webkitgtk_4_1`, `gtk3`, `glib`, `libsoup_3`, `openssl`, `dbus`,
  `librsvg`, `libayatana-appindicator`
- macOS: Xcode Command Line Tools
- Windows: WebView2 runtime (bundled with Windows 11)

## Branch naming + commit conventions

### Branch names

Follow one of these patterns (see
[`docs/development/branch-policy.md`](development/branch-policy.md)):

```
feature/<slug>
fix/<slug>-t_<task-id>
stratum/t_<task-id>-<slug>      # task-scoped (prepared by the board)
```

### Conventional commits

This repository uses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
Commit messages follow:

```
<type>(<scope>): <short summary>

<body — optional>

<footer — optional>
```

Allowed `type`s: `feat`, `fix`, `docs`, `test`, `chore`, `refactor`,
`perf`, `style`, `build`, `ci`, `revert`.

Scope is the crate/component it affects (`pkm-index`, `graph`, `sync`,
`ui`, `docs`, ...).

Examples (from recent history):

```
feat(E10.F7): Plugin Directory + Theme Hub — install from stratum-plugins
fix(clippy): resolve -D warnings in pkm-block assets
docs: fix stale endpoint-validation ref and LICENSE 404 on gh-pages
chore(repo-hygiene): e2e probe paths portable via import.meta.url
```

**Rules:**
- Imperative, lowercase, no trailing period.
- Keep the subject under ~72 characters.
- Reference the related task id in the footer if applicable: `(t_<task-id>)`.

## Code style

### Rust

- Edition 2021. `cargo fmt` for formatting, `cargo clippy` for linting.
- Tests as `#[cfg(test)]` modules alongside implementation.
- Use `tempfile` for filesystem tests.
- The single error type is `PkmError` from `pkm-core` — do **not** create
  new error types in individual crates; extend `PkmError` variants instead
  (use `#[from]` derives).
- Every source file must have a module-level doc comment (`//!`), and every
  public function a doc comment (`///`), per the project standards in
  `AGENTS.md`.
- Command handlers in `src-tauri/src/commands/` are **thin glue layers** —
  business logic belongs in the corresponding crate
  (`pkm-block`, `pkm-index`, `pkm-sync`, `pkm-core`, ...).

### TypeScript / React

- TypeScript strict mode, React 19, function components + hooks.
- **MUI v6** for UI — use `@mui/material` + `@mui/icons-material`, `sx`
  prop for styling (no Tailwind).
- Zustand for state; `src/lib/commands.ts` wrappers for `tauri.invoke()`;
  DTO types in `src/lib/types.ts` (must match the Rust `Serialize` structs).
- Component size limits (enforced by convention):
  - File < 500 lines, component JSX < 50 lines, inline function < 20 lines.
  - Same logic in 2+ files → extract to `src/lib/` or `src/lib/hooks/`.
  - Every component defines `interface Props`.
- **i18n:** all user-facing strings go through `useTranslation()` —
  never hardcode UI strings (the `i18nGate` test fails otherwise). Add a new
  key to **all** locale files under `src/i18n/locales/` (not only `en.json`)
  or the `localeParity` test fails.

## Testing

Run these **before** opening a pull request:

```bash
# Rust (workspace)
cargo build --workspace
cargo test --workspace

# Specific crate / test
cargo test -p pkm-index
cargo test -p pkm-index -- graph::tests::test_get_backlinks

# MCP server suite + coverage gate (>=80%)
cargo test -p pkm-mcp --all-targets
bash scripts/mcp-coverage-gate.sh

# Frontend
npm install
npm run test          # unit + component tests (vitest + jsdom)
npm run lint          # ESLint (no Prettier — formatting is Rust-only)
npm run build         # tsc -b && vite build

# E2E harness (requires: app built to target/debug/stratum-tauri,
# tauri-driver, WebKitWebDriver, Xvfb)
npm run test:e2e:harness
```

The `e2e-harness` job in CI (`.github/workflows/ci.yml`) runs on every push
to `master`, on every PR, and on a weekly schedule — you do not need to run
the harness locally for normal PRs, but it is **required** (with
`frontend-test`) for merges to `master`.

### Running the real E2E locally

```bash
npm run build
cargo build -p stratum-tauri      # must include the custom-protocol feature
npm run test:e2e:harness
```

Prerequisites: `cargo install tauri-driver`, WebKitWebDriver
(Arch: `webkit2gtk-4.1` → `/usr/lib/webkit2gtk-4.1/WebKitWebDriver`;
Ubuntu/Debian: `webkit2gtk-driver`), and Xvfb
(Arch: `xorg-server-xvfb`; Ubuntu: `xvfb`). Set `HARNESS_DRIVER` to the
WebKitWebDriver absolute path if it's elsewhere.

## Documentation rules (MANDATORY)

Every code change must update the docs for the affected area — see
`AGENTS.md` and the table below. CI (planned) and review enforce this.

| Change | Required action |
|--------|-----------------|
| New component | Add to `Frontend Components` table in `AGENTS.md` + `docs/guide/` entry if user-facing |
| Renamed component | Update `AGENTS.md` table; update all user docs |
| Component > 400 lines | Split into sub-components |
| New hook/utility | Add to `Custom Hooks` table in `AGENTS.md`; update `docs/development/frontend-guide.md` |
| Rust API changed | Update doc comments; update `docs/development/rust-guide.md` |
| New crate/module | Add to Crate Dependency Graph + Workspace Layout in `AGENTS.md`; add module docs |
| New feature | Add `docs/guide/` entry; update `contributing.md` if dev workflow changes |
| Behavior change | Update relevant `docs/guide/` entry |
| UI changed | Verify screenshots in docs still match; update if needed |
| Mobile change | Update `docs/development/mobile-guide.md` |
| New dependency | Verify AGPL-3.0 compatibility (see policy below) |
| `Cargo.toml` changed | Verify the dependency graph in `AGENTS.md` matches |

The docs are built with **MkDocs Material** (`docs/mkdocs.yml`). Validate
with:

```bash
mkdocs build -f docs/mkdocs.yml --strict
```

### Repo hygiene — what gets committed

See `AGENTS.md` → "Repo Hygiene". In short: shipped code and user docs only.
Plans, QA reports, findings, evidence, screenshots → `.hermes/` (gitignored).
Never commit an implementation aid to repo root, `docs/` (except
`docs/development/`), or `verification/` (except `verification/cdp_probe.mjs`).

## Plugin submission

WASM plugins for Stratum live in the
[stratum-plugins repository](https://github.com/short-circuit/stratum-plugins).
To submit a plugin:

1. Follow
   [PLUGIN_TEMPLATE.md](https://github.com/short-circuit/stratum-plugins/blob/master/PLUGIN_TEMPLATE.md)
   to scaffold your plugin.
2. Open a PR against `stratum-plugins` (the catalog index is generated
   automatically).
3. Mention the plugin in the in-app Plugin Directory / Theme Hub.

## Pull requests

Open a PR against `master` from your branch. All PRs need:

1. Clean CI (build + test + clippy Rust; build + lint frontend; E2E harness).
2. At least one reviewer.
3. Documentation updates if APIs change.

**PR template** (`.github/PULL_REQUEST_TEMPLATE.md`):

- What does this change do and why?
- Does it fix a bug or add a feature? Reference issue/task id.
- Which tests were run, and what was the result?
- Docs updated? (if needed)

## RFC process

Major features require an RFC. Open an issue with:

- **Motivation** — why this feature?
- **Design** — how does it work?
- **Alternatives** — what else was considered?
- **Migration** — impact on existing users.

## License & dependency policy

Stratum is **AGPL-3.0-only**. Every new dependency (Rust crate, npm package)
must be license-compatible with AGPL-3.0 (MIT, Apache-2.0, BSD-2/3, ISC,
Zlib, Unlicense, CC0, MPL-2.0, ... — see `AGENTS.md` for the full list).
Flag any non-obvious license for review before adding the dependency.

## Getting help

- Issues: https://github.com/short-circuit/stratum/issues
- Discussions: https://github.com/short-circuit/stratum/discussions
- Good first issues: https://github.com/short-circuit/stratum/labels/good%20first%20issue
- Docs: https://short-circuit.github.io/stratum/docs/
