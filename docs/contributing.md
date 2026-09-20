# Contributing

We welcome contributions! This guide covers development setup, conventions, and the PR workflow.

## Getting Started

### Nix (recommended)

```bash
# Enter dev shell with all dependencies
nix develop ./nix

# Or use direnv for automatic activation
direnv allow

# Build everything
cargo build --workspace
cargo test --workspace
npm install
npm run build
```

### Manual Setup

Requires:
- Rust 1.75+ (MSRV) — [rustup.rs](https://rustup.rs)
- Node.js 22+
- Tauri v2 system libraries:
  - Linux: `webkitgtk_4_1`, `gtk3`, `glib`, `libsoup_3`, `openssl`, `dbus`, `librsvg`, `libayatana-appindicator`
  - macOS: Xcode Command Line Tools
  - Windows: WebView2 runtime (bundled with Windows 11)

```bash
cargo build --workspace
cargo test --workspace
npm install
npm run tauri:dev
```

## Development Workflow

1. Create a feature branch from `master`
2. Make changes (Rust and/or TypeScript)
3. Run tests
4. Open a pull request

## Testing

```bash
# Run all Rust tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p pkm-index

# Run a specific test
cargo test -p pkm-index -- graph::tests::test_get_backlinks

# Run the MCP server test suite
cargo test -p pkm-mcp --all-targets

# Run the MCP coverage gate (>=80% line coverage of the MCP crates)
bash scripts/mcp-coverage-gate.sh

# Run frontend tests
npm run test

# Frontend linting
npm run lint

# Run the tauri-driver E2E harness (requires: the app built to
# target/debug/stratum-tauri, tauri-driver, WebKitWebDriver, Xvfb)
npm run test:e2e:harness
```

The `test:e2e:harness` suite runs automatically in CI (`.github/workflows/ci.yml`,
`e2e-harness` job) on every push to `master`, on every PR to `master`, and on a
**weekly schedule** (drift check against the latest `master`; manually too via
`workflow_dispatch`) — no manual E2E step is required for normal PRs. The
`e2e-harness` check is **required** for merges to `master` (alongside
`frontend-test`), and the job uploads `test-results/*`, the built binary, and a
last-frame screenshot as `e2e-harness-artifacts` when it fails.

### Running the real E2E locally

The real E2E drives the **compiled app** (`target/debug/stratum-tauri`, which
must be built with the `custom-protocol` feature — see `e2e/harness/README.md`)
over the WebDriver protocol; no mocks.

Prerequisites:

- `tauri-driver` — `cargo install tauri-driver`
- `WebKitWebDriver`:
  - **Arch:** `sudo pacman -S webkit2gtk` (binary ships inside `webkit2gtk-4.1` as
    `/usr/lib/webkit2gtk-4.1/WebKitWebDriver`)
  - **Ubuntu/Debian:** `webkit2gtk-driver` (the `WebKitWebDriver` binary ships in
    this package, not `libwebkit2gtk-4.1-dev`) + `libwebkit2gtk-4.1-dev`
  - If the binary is anywhere else on your machine, set `HARNESS_DRIVER` to its
    absolute path
- `Xvfb` — Arch: `sudo pacman -S xorg-server-xvfb`; Ubuntu: `apt install xvfb`

Then:

```bash
npm run build
cargo build -p stratum-tauri
npm run test:e2e:harness
```

## Project Conventions

### Rust

- Edition 2021
- `cargo fmt` for formatting
- `cargo clippy` for linting
- Unit tests in `#[cfg(test)]` modules alongside implementation
- Use `tempfile` for filesystem tests
- Use `thiserror` for error types
- Use `anyhow` for application-level error handling

### TypeScript / React

- TypeScript 6.0 strict mode
- React 19 with function components and hooks
- **MUI (Material UI)** v6 for UI components — use `@mui/material` and `@mui/icons-material`
  - Use the `sx` prop for styling (not Tailwind)
  - Icons from `@mui/icons-material` — match existing icon style and naming
  - Use MUI's built-in `theme` (light/dark) via `useTheme()` hook
- Zustand for global state
- Tauri `invoke()` wrappers in `src/lib/commands.ts`
- DTO types in `src/lib/types.ts`

## Code Review

All PRs need:
1. Clean CI (build + test + clippy for Rust, build + lint for frontend)
2. At least one reviewer
3. Documentation updates if API changes

## RFC Process

Major features require an RFC. Open an issue with:

- **Motivation**: Why this feature?
- **Design**: How does it work?
- **Alternatives**: What else was considered?
- **Migration**: Impact on existing users

## CI/CD

The project uses GitHub Actions:

| Workflow | Trigger | What it does |
|----------|---------|--------------|
| CI | Push/PR to master, weekly schedule, manual | Build, test, clippy, fmt, lint, frontend unit tests, tauri-driver E2E harness (artifact-uploading on failure) |
| Release | Tag `v*` | Build and publish binaries |

## License

Stratum is licensed under AGPL-3.0-only. By contributing, you agree that your contributions will be licensed under the same license.
