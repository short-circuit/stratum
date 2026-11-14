# Installation

Stratum runs on Linux, macOS, and Windows as a single native application
(gpui shell over the Rust core), plus an Android host of the same shell.

## Download a Release

Pre-built binaries are available on the [GitHub Releases page](https://github.com/short-circuit/stratum/releases).

| Platform | Format | File |
|----------|--------|------|
| Linux | AppImage | `stratum-gpui-linux.AppImage` |
| Linux | Debian | `stratum-gpui-linux.deb` |
| Linux | RPM | `stratum-gpui-linux.rpm` |
| Windows | MSI installer | `stratum-gpui-windows.msi` |
| Windows | NSIS installer | `stratum-gpui-windows-setup.exe` |
| macOS | App bundle (arm64) | `stratum-gpui-macos-arm64.zip` |
| macOS | App bundle (x86_64) | `stratum-gpui-macos-x86_64.zip` |
| Android | APK / AAB | `stratum-gpui-android.apk` / `stratum-gpui-android.aab` |

Every release ships a `sha256sums.txt` with checksums for all artifacts.

!!! note "Linux runtime dependencies"
    The AppImage bundles the xkbcommon stack and `libasound` and needs the
    system `libxcb1` plus a Vulkan driver. The deb declares `libc6 (>= 2.35)`,
    `libasound2`, `libxcb1`, `libxcb-xkb1`, `libxkbcommon-x11`, `libvulkan1`;
    the RPM derives the same sonames. No WebKitGTK or GTK web stack is needed —
    the shell renders natively.

## Build from Source

### Prerequisites

- **Rust** (pinned by `rust-toolchain.toml`) — [rustup.rs](https://rustup.rs)
- A C toolchain, `pkg-config` and the native windowing libraries:
  `libxkbcommon`, `libxcb`/`libx11`, a Vulkan loader and driver, ALSA, and GTK 3
  for the tray indicator (Linux). The Nix shell below provides all of them.
- No Node.js, no npm, no web toolchain — the shell embeds its own assets.

### Nix (recommended)

```bash
# Clone the repository
git clone https://github.com/short-circuit/stratum.git
cd stratum

# Enter the Nix dev shell (all dependencies included)
nix develop ./nix

# Or use direnv for automatic activation
direnv allow

# Build and test
cargo build --workspace
cargo test --workspace
```

### Manual Build

```bash
git clone https://github.com/short-circuit/stratum.git
cd stratum

# Build Rust crates
cargo build --workspace

# Run tests (optional but recommended)
cargo test --workspace
```

## Running the Desktop App

```bash
# Development build
cargo build -p stratum-ui

# Run against the deterministic fixture vault (no vault required)
STRATUM_FIXTURE=1 cargo run -p stratum-ui

# Run against a real vault
STRATUM_VAULT=/path/to/vault cargo run -p stratum-ui
```

The built binary is `target/debug/stratum` (or `target/release/stratum` for a
release build). See [Configuration](configuration.md) for the environment
variables that select a vault, a locale or a start state.

### Installer builds

```bash
scripts/package-desktop.sh                 # deb, AppImage and rpm for this host
scripts/verify-packages.sh --dir dist/packages
```

`scripts/build-android.sh` builds the Android APK/AAB (`--release` for the
signed release artifacts).

## Using the CLI

The CLI binary is built alongside the rest of the workspace:

```bash
cargo run -p pkm-cli -- --help
```

Or build it separately:

```bash
cargo build -p pkm-cli
./target/debug/pkm-cli --help
```

See the [CLI Reference](../cli/command-reference.md) for all commands.

## Using the MCP Server

To expose your vault to external AI clients over the Model Context Protocol,
build and run the `pkm-mcp` binary:

```bash
cargo build -p pkm-mcp
./target/debug/pkm-mcp --vault /path/to/your/vault
```

For transports, configuration, authentication, the full tool list, and
troubleshooting, see the [MCP Server guide](../guide/mcp-server.md).
