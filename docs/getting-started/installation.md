# Installation

Stratum runs on Linux, macOS, and Windows. There are several ways to get it:

## Download a Release

Pre-built binaries are available on the [GitHub Releases page](https://github.com/short-circuit/stratum/releases).

| Platform | Format | File |
|----------|--------|------|
| Linux | AppImage | `stratum_<version>_amd64.AppImage` |
| Linux | Debian | `stratum_<version>_amd64.deb` |
| Windows | MSI | `stratum_<version>_x64.msi` |
| Windows | NSIS | `stratum_<version>_x64-setup.exe` |
| macOS | DMG | `stratum_<version>_x64.dmg` |

!!! note "Linux Dependencies"
    The AppImage bundles most dependencies. For other Linux formats you need WebKitGTK libraries:
    ```bash
    # Debian/Ubuntu
    sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev                      libjavascriptcoregtk-4.1-dev librsvg2-dev

    # Arch Linux
    sudo pacman -S webkitgtk-4.1 gtk3 libsoup3 librsvg
    ```

## Build from Source

### Prerequisites

- **Rust** 1.75+ — [rustup.rs](https://rustup.rs)
- **Node.js** 22+ — [nodejs.org](https://nodejs.org)
- **Tauri v2 system libraries** (see above)

### Nix (recommended)

```bash
# Clone the repository
git clone https://github.com/short-circuit/stratum.git
cd stratum

# Enter the Nix dev shell (all dependencies included)
nix develop ./nix

# Or use direnv for automatic activation
direnv allow

# Build everything
cargo build --workspace
cargo test --workspace
npm install
npm run build
```

### Manual Build

```bash
git clone https://github.com/short-circuit/stratum.git
cd stratum

# Install frontend dependencies
npm install

# Build Rust crates
cargo build --workspace

# Run tests (optional but recommended)
cargo test --workspace
```

## Running the Desktop App

```bash
# Development mode (hot-reload)
npm run tauri:dev

# Production build
npm run tauri:build
```

The built app is in `src-tauri/target/release/stratum`.

### Environment Variables

Stratum uses some environment variables for Wayland compatibility:

```bash
# If you encounter display issues on Linux/ Wayland:
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1

# Or use the provided npm scripts:
npm run tauri:dev:x11  # Force X11 backend
```

## Using the CLI

The CLI binary is built alongside the desktop app:

```bash
cargo run -p pkm-cli -- --help
```

Or build it separately:

```bash
cargo build -p pkm-cli
./target/debug/stratum --help
```

See the [CLI Reference](../cli/command-reference.md) for all commands.
