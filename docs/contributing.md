# Contributing

## Getting Started

### Nix (recommended)

```bash
# Enter dev shell with all dependencies
nix develop ./nix

# Or use direnv for auto-activation
direnv allow

# Build everything
cargo build --workspace
cargo test --workspace
npm install
npm run build
```

### Manual

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

1. Create a feature branch
2. Make changes (Rust and/or TypeScript)
3. Run `cargo test --workspace` to pass Rust tests
4. Run `cargo clippy --workspace -- -D warnings`
5. Run `npm run lint` for frontend linting
6. Run `npm run build` to verify frontend builds
7. Open a pull request

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
- Zustand for global state
- Tailwind CSS v4 for styling (CSS custom properties for theming)
- Tauri `invoke()` wrappers in `src/lib/commands.ts`
- DTO types in `src/lib/types.ts`

## Testing

```bash
# Run all Rust tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p pkm-index

# Run a specific test
cargo test -p pkm-index -- graph::tests::test_get_backlinks

# Frontend linting
npm run lint
```

## RFC Process

Major features require an RFC. Open an issue with the RFC template:

- **Motivation**: Why this feature?
- **Design**: How does it work?
- **Alternatives**: What else was considered?
- **Migration**: Impact on existing users

## Code Review

All PRs need:
1. Clean CI (build + test + clippy for Rust, build + lint for frontend)
2. At least one reviewer
3. Documentation updates if API changes
