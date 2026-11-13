# Packaging the native shell

This page describes how the desktop and Android artifacts are produced, which
configuration drives them, how the release workflow publishes them, and how to
verify an installer before it ships.

The packaged application is the native gpui shell (`crates/stratum-ui`, binary
`stratum`). The Tauri bundle is built by `ci.yml` until the web frontend is
retired; the native artifacts are published as `stratum-gpui-*` so both can
coexist on one release.

## Formats and tools

| Format | Tool | Configuration |
|--------|------|---------------|
| `.deb` | `cargo-packager` | `[package.metadata.packager]` + `[package.metadata.packager.deb]` in `crates/stratum-ui/Cargo.toml` |
| `.AppImage` | `cargo-packager` (linuxdeploy) | same table, `[package.metadata.packager.appimage]` |
| `.rpm` | `cargo-generate-rpm` | `[package.metadata.generate-rpm]` in `crates/stratum-ui/Cargo.toml` |
| `.msi` | `cargo-packager` (WiX) | `[package.metadata.packager.wix]` |
| `.exe` (NSIS) | `cargo-packager` (NSIS) | `[package.metadata.packager.nsis]` |
| `.app` / `.zip` | `cargo-packager` | `[package.metadata.packager.macos]` |
| `.apk` / `.aab` | `cargo-ndk` + Gradle | `crates/stratum-mobile/android/app/build.gradle.kts` |

The package name is `stratum`, the product name is `Stratum` and the bundle
identifier is `app.stratum`. Name, version, license and repository are inherited
from the crate manifest, so a workspace version bump is the only edit needed —
never hard-code a version in the packaging metadata.

Desktop entries, icons and the license file live in `packaging/`:

```
packaging/
├── stratum.desktop          # static entry shipped by the RPM
├── stratum.desktop.hbs      # template rendered by cargo-packager for deb/AppImage
└── icons/                   # hicolor tree, .ico and .icns
```

`packaging/stratum.desktop` and `packaging/stratum.desktop.hbs` must stay in
sync: the RPM ships the static file, the deb/AppImage render the template.

## Building locally

The scripts are the single source of truth — CI calls exactly the same commands.

```bash
# Enter the dev shell first
nix develop ./nix

# Desktop installers (deb, AppImage, rpm on Linux)
scripts/package-desktop.sh

# One format only, or a cross target
scripts/package-desktop.sh --formats deb
scripts/package-desktop.sh --formats app --target aarch64-apple-darwin

# Android release APK + AAB (needs the upload key in the environment)
scripts/build-android.sh --release
```

Prerequisites:

- `cargo install cargo-packager --locked` and, for RPM,
  `cargo install cargo-generate-rpm --locked`.
- Linux packaging must run on a glibc 2.35 base (Ubuntu 22.04 or the
  `nix develop` shell of an equivalent generation). The `deb` declares
  `libc6 (>= 2.35)`; a binary built against a newer glibc cannot install on
  older distributions. CI builds on `ubuntu-22.04` for this reason.
- Windows needs no extra setup: cargo-packager downloads the WiX and NSIS
  toolchains into its cache on first use.
- macOS needs the Xcode command line tools.

Artifacts and a `sha256sums.txt` manifest are written to `dist/packages`
(override with `OUT_DIR` or `--out-dir`).

### Runtime dependencies

The shell links `libxcb`, `libxkbcommon`, `libasound` and loads the Vulkan
loader through `dlopen` (wgpu). The Wayland client is pure Rust, so no system
Wayland library is required. The deb lists those packages explicitly; the RPM
derives the sonames with the automatic dependency pass and adds
`libvulkan.so.1()(64bit)` by hand because `ldd` cannot see a `dlopen`.

The AppImage is not a full dependency snapshot: linuxdeploy blacklists
`libxcb.so.1` and the Vulkan loader, so those stay on the host (every X11
desktop provides `libxcb1`, and the Vulkan driver has to match the machine's
GPU). `libxkbcommon`, `libxkbcommon-x11`, `libxcb-xkb`, `libXau`, `libXdmcp` and
`libasound` are bundled — `libasound` under `usr/lib/x86_64-linux-gnu` because
linuxdeploy skips it and `[package.metadata.packager.appimage] libs` copies it
in afterwards.

## Signing

No key material is committed. Everything is environment-driven:

| Artifact | Variables | Effect when unset |
|----------|-----------|-------------------|
| Android APK/AAB | `ANDROID_KEYSTORE_PATH`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD` | Gradle signs with the debug key; the build script prints a warning |
| macOS `.app` | `APPLE_SIGNING_IDENTITY` | Bundle stays unsigned |
| macOS notarization | `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` (plus the identity) | Skipped; the ticket is only fetched when all four are set |
| Windows MSI/NSIS | `WINDOWS_CERTIFICATE` (base64 PFX), `WINDOWS_CERTIFICATE_PASSWORD` | Installers stay unsigned |
| RPM | `STRATUM_RPM_SIGNING_KEY`, `STRATUM_RPM_SIGNING_KEY_ID` | Unsigned RPM |

The release workflow reads the same values from repository secrets:
`ANDROID_KEYSTORE` (base64 keystore), `ANDROID_KEYSTORE_PASSWORD`,
`ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, `WINDOWS_CERTIFICATE`,
`WINDOWS_CERTIFICATE_PASSWORD`.

The Android `versionCode` must increase with every store upload. The release
workflow derives it from the tag (`v1.2.3` → `1020300`) and passes
`STRATUM_ANDROID_VERSION_NAME` / `STRATUM_ANDROID_VERSION_CODE` to Gradle.

## Updates

There is no in-app updater, and the native shell keeps that behaviour: the Tauri
build never shipped one, so adding a network client now would be a new feature,
not parity. Updates arrive through the platform channel — `apt`/`dnf` for the
deb and rpm, a replaced AppImage, the MSI/NSIS installer, Google Play for the
Android bundle, or a fresh `.app` from the release page. The shell therefore
never phones home; the release workflow only publishes the artifacts and the
checksum manifest.

## Verifying an installer

`scripts/verify-packages.sh` installs the artifacts into throwaway containers
instead of trusting the build host:

```bash
scripts/verify-packages.sh --dir dist/packages
```

- `.deb` → `ubuntu:22.04`, `apt install ./stratum_*.deb`, then a headless launch
  under Xvfb with the software Vulkan driver.
- `.rpm` → `fedora:42`, `dnf install`, then the same launch probe.
- `.AppImage` → extracted on `ubuntu:22.04`, `libasound` confirmed inside the
  AppDir (the container does not install it), then launched through `AppRun`
  with `APPIMAGE_EXTRACT_AND_RUN=1`. `libxcb1` is installed there because
  linuxdeploy leaves it to the host.
- `.apk`/`.aab` → `apksigner verify` and `aapt2 dump badging` from the local SDK.

The launch probe passes only when the shell stays alive for the whole window
(`timeout` returns 124) and the log contains no panic, so a missing shared
library, a panicking renderer or an early exit fails the check. A probe that
fails a payload assertion prints nothing — read the log file named in the
result, not the console.

The Windows and macOS formats need their own platform, so they are verified by
two scripts that run on the release runner of that platform (and locally on the
same OS):

```powershell
# Windows: installs the MSI, locates the payload, launches it, uninstalls
pwsh scripts/verify-windows-installers.ps1 -Dir dist/packages
```

```bash
# macOS: layout, architecture, signature and launch probe of the .app
scripts/verify-macos-app.sh --dir dist/packages --arch arm64
```

- `.msi` → `msiexec /i /qn /norestart`, then the installed `stratum.exe` is
  located under `%LOCALAPPDATA%`/Program Files, launched with
  `STRATUM_FIXTURE=1`, and removed through the product's uninstall entry. The
  probe passes when the shell stays alive for the probe window or exits cleanly
  (a headless runner may refuse to create a window); a panic or a non-zero exit
  fails it. The uninstall is confirmed by polling for the payload and the
  uninstall registry entry, because an NSIS uninstaller relaunches itself from
  `%TEMP%` and returns before the files are gone.
- `.exe` (NSIS) → the installer runs in silent mode (`/S`), then the same launch
  and uninstall checks apply.
- `.zip` (macOS) → the archive is expanded with `ditto`, the bundle must carry
  `app.stratum`, `LSMinimumSystemVersion` 11.0 and exactly the requested
  architecture, and the shell is launched the same way. A Developer ID signature
  must pass `codesign --verify --deep --strict` and `spctl`; without the Apple
  secrets the bundle is unsigned (or ad-hoc signed) and Gatekeeper rejects it by
  design, so the script reports that instead of failing — see the first-launch
  note in `docs/getting-started/installation.md`.

## CI release job

`.github/workflows/gpui-release.yml` builds every format and publishes the
artifacts with checksums:

- **Triggers** — `v*` tags (publish), pull requests that touch packaging inputs
  (Linux gate), and manual dispatch (build only).
- **Jobs** — `linux` (ubuntu-22.04, deb/AppImage/rpm + container verification),
  `windows` (MSI/NSIS + install/launch/uninstall verification), `macos` (arm64
  and x86_64 app bundles + bundle/signature/launch verification), `android`
  (signed APK/AAB) and `release`, which requires all four to succeed. A
  packaging job therefore fails on a broken installer, not only on a broken
  build.
- **Publication** — assets are renamed to `stratum-gpui-<platform>.<ext>`, a
  single `sha256sums.txt` is generated over all of them, verified with
  `sha256sum -c` on the runner, and the draft release for the tag is updated.
  A final step downloads the published assets back through the API and checks
  them against the manifest, so a corrupted upload fails the run instead of
  shipping. The release stays a draft (a human publishes it), which also means
  anonymous downloads of the assets only work after it is published.
- **Failure diagnostics** — a packaging job that fails keeps its output as a
  `log-<job>` artifact, and the `diagnostics` job attaches those logs to the
  draft release as `<job>-packaging.log`, because Actions artifacts need a token
  to download while release assets do not. The verification scripts also emit
  `::error::` annotations, which show up on the check run without opening the
  job log.

The Tauri workflow (`ci.yml`) also publishes on a `v*` tag. It ships its own
manifest, `sha256sums-pkm.txt`, because both workflows update the same draft
release: a shared `sha256sums.txt` name would let whichever release job finished
last overwrite the other channel's checksums.

Verify a download against the published manifest:

```bash
sha256sum -c sha256sums.txt
```

## Pitfalls

- **glibc floor.** Building Linux packages on a newer distribution silently
  raises the required `libc6` version. Always package on Ubuntu 22.04 (CI) or
  inside the equivalent container.
- **linuxdeploy and AppImages.** linuxdeploy runs itself and its output plugin
  as AppImages; `scripts/package-desktop.sh` exports
  `APPIMAGE_EXTRACT_AND_RUN=1` so no FUSE mount is needed in containers or CI.
- **linuxdeploy strip.** The bundled `strip` in linuxdeploy 1-alpha cannot read
  `.relr.dyn` sections from newer distributions, which aborts the AppImage
  build. Another reason to build on the Ubuntu 22.04 base.
- **AppImage system libraries.** linuxdeploy blacklists `libxcb.so.1` and the
  Vulkan loader, so the AppImage needs both on the host; bundling `libxcb1`
  yourself is possible by adding it to
  `[package.metadata.packager.appimage] libs`, but the Vulkan driver must always
  match the GPU. The verification container installs `libxcb1` for this reason.
- **Stripping is platform-specific.** `strip --strip-unneeded` is GNU binutils
  only; macOS cctools `strip` takes `-x`, and Windows runners have no guaranteed
  `strip`. `scripts/package-desktop.sh` picks the flag per platform and skips
  the step instead of failing when the tool is absent.
- **`cargo-generate-rpm -p`.** The flag takes the crate *directory*
  (`crates/stratum-ui`), not the package name; a package name fails with
  `stratum-ui/Cargo.toml: No such file or directory`.
- **Path bases.** cargo-packager changes into the crate directory before
  packaging, so its configured paths are written relative to
  `crates/stratum-ui`; cargo-generate-rpm runs from the repository root, so its
  asset paths are root-relative.
- **`require-sh`.** The RPM ships no scriptlets, so `/bin/sh` is not a
  dependency (`require-sh = false`); re-enable it if a scriptlet is ever added.
