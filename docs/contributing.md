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

**Manual setup** requires: the Rust toolchain pinned by `rust-toolchain.toml`,
a C toolchain, `pkg-config`, and the native windowing/rendering libraries:

- Linux: `libxkbcommon`, `libxkbcommon-x11`, `libxcb`/`libx11`, a Vulkan loader
  and driver, `alsa-lib`, and GTK 3 with `libayatana-appindicator` for the tray
- macOS: Xcode Command Line Tools
- Windows: MSVC build tools

No Node.js or npm is required — the shell embeds its own assets and catalogs.

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
- Service logic lives in `stratum-app` (`crates/stratum-app`), never in the
  shell. The shell (`crates/stratum-ui`) is a thin renderer over
  `ShellData`/`PanelData`; a panel must not reach into a domain crate directly.
- Every user-facing string goes through `stratum-i18n` with `{{var}}`
  interpolation and must exist in **all eight** catalogs —
  `crates/stratum-i18n/tests/raw_keys.rs` fails when a shell key is missing.

### Native shell (Rust, gpui)

- Rust 2021, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`.
- Panels live under `crates/stratum-ui/src/panels/<name>/`; mobile wrappers
  under `crates/stratum-ui/src/mobile/variants/`.
- Size limits (reviewed, not just convention): file < 500 lines, render body
  < 400 lines, inline function < 20 lines. Same logic in 2+ places → extract to
  `stratum-editor`/`stratum-app` or a shared module.
- A panel that renders text must declare its keys in a `*_I18N_KEYS` gate list;
  `crates/stratum-ui/tests/*_contract.rs` verifies them in every catalog.
- Every shipped surface needs a capture state in `scripts/capture-gpui-panels.sh`
  (or `capture-gpui-mobile.sh`) and a golden under `goldens/gpui/`.

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

# Shell contracts and parity harness
cargo test -p stratum-ui -p stratum-i18n -p stratum-editor
./scripts/verify-gpui-goldens.sh      # golden screenshots (Xvfb + ImageMagick)
./scripts/verify-gpui-perf.sh         # perf budgets
./scripts/verify-gpui-a11y.sh         # accessibility tree probe
```

The `parity` job in CI (`.github/workflows/ci.yml`) runs the golden, perf and
accessibility gates on every push to `master` and every PR; the `test` and
`shell` jobs run the workspace and cross-platform gates. All of them are
required for merges to `master`.

## Documentation rules (MANDATORY)

Every code change must update the docs for the affected area — see
`AGENTS.md` and the table below. CI (planned) and review enforce this.

| Change | Required action |
|--------|-----------------|
| New panel | Add to `Native shell surfaces` table in `AGENTS.md` + `docs/guide/` entry + golden + capture state |
| Renamed panel | Update `AGENTS.md` table, the guide, the router and the goldens |
| Panel module > 500 lines | Split into sub-modules |
| New shell module | Update `docs/development/gpui-ui.md` |
| Rust API changed | Update doc comments; update `docs/development/rust-guide.md` |
| New crate/module | Add to Crate Dependency Graph + Workspace Layout in `AGENTS.md`; add module docs |
| New service method | Keep `stratum-app` authoritative; update `docs/development/service-layer.md` and the inventory gate |
| New feature | Add `docs/guide/` entry; update `contributing.md` if dev workflow changes |
| Behavior change | Update relevant `docs/guide/` entry |
| UI changed | Regenerate the golden (`scripts/verify-gpui-goldens.sh --update`) and review the image diff |
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
Never commit an implementation aid to repo root or `docs/` (except
`docs/development/`).

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

1. Clean CI (fmt, clippy, workspace tests, shell cross-platform build, parity
   harness, Android smoke).
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

Stratum is **AGPL-3.0-only**. Every new dependency (Rust crate)
must be license-compatible with AGPL-3.0 (MIT, Apache-2.0, BSD-2/3, ISC,
Zlib, Unlicense, CC0, MPL-2.0, ... — see `AGENTS.md` for the full list).
Flag any non-obvious license for review before adding the dependency.

## Getting help

- Issues: https://github.com/short-circuit/stratum/issues
- Discussions: https://github.com/short-circuit/stratum/discussions
- Good first issues: https://github.com/short-circuit/stratum/labels/good%20first%20issue
- Docs: https://short-circuit.github.io/stratum/docs/
