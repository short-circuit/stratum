# Mobile Development Guide

This document covers everything you need to know about building, running, and testing Stratum on mobile platforms. Stratum uses Tauri v2's mobile support to target both Android and iOS from a shared Rust + TypeScript codebase.

## Build Targets

Stratum currently supports the following mobile targets:

### Android

| Architecture | Rust Target | Status |
|---|---|---|
| ARM64 (most devices) | `aarch64-linux-android` | Active |
| ARMv7 (older devices) | `armv7-linux-androideabi` | CI only |
| x86_64 (emulator, Chromebook) | `x86_64-linux-android` | Active |
| x86 (emulator, older) | `i686-linux-android` | CI only |

The Android build targets API 24 (Android 7.0, Nougat) as the minimum SDK version, configured in `src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "android": {
      "minSdkVersion": 24
    }
  }
}
```

### iOS

| Architecture | Rust Target | Status |
|---|---|---|
| ARM64 (physical device) | `aarch64-apple-ios` | CI only |
| ARM64 Simulator (Apple Silicon Macs) | `aarch64-apple-ios-sim` | Active |
| x86_64 Simulator (Intel Macs) | `x86_64-apple-ios` | CI only |

!!! note "iOS device builds"

    iOS device builds require a paid Apple Developer account, signing certificates, and a provisioning profile. The CI pipeline builds for simulator only. Physical device testing is done locally by developers with Apple Developer accounts.

## Setup

### Android SDK / NDK

Building for Android requires the Android SDK and NDK. The CI pipeline (`.github/workflows/ci.yml`) documents the exact setup.

**Prerequisites:**

- Java 17 (Temurin recommended)
- Android SDK (command line tools)
- NDK 27.0.12077973

**Quick setup:**

```bash
# Install Java
sudo apt install openjdk-17-jdk

# Install Android command-line tools
# Download from https://developer.android.com/studio#command-line-tools-only
ANDROID_HOME="$HOME/Android/Sdk"
mkdir -p "$ANDROID_HOME"

# Install required SDK packages
sdkmanager "platforms;android-34" \
  "build-tools;34.0.0" \
  "ndk;27.0.12077973" \
  "cmake;3.22.1"

# Export environment variables
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.0.12077973"
export PATH="$ANDROID_HOME/ndk/27.0.12077973/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
```

**Set up cross-compilation environment variables:**

The CI configures these for each architecture. For local development you'll need the NDK toolchain on your `PATH` and the following environment variables set:

```bash
# ARM64 (most devices)
export CC_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"
export AR_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"

# x86_64 (emulator)
export CC_x86_64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android21-clang"
export AR_x86_64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
```

**Rust targets:**

The project's `rust-toolchain.toml` includes Android targets. Make sure they are installed:

```bash
rustup target add aarch64-linux-android x86_64-linux-android
```

On first build, `cargo tauri android init` generates the Android project under `src-tauri/gen/android/`. This directory is gitignored but cached in CI.

### iOS / Xcode

Building for iOS requires macOS and Xcode.

**Prerequisites:**

- macOS (latest version recommended)
- Xcode 16+ (from the Mac App Store)
- Xcode Command Line Tools: `xcode-select --install`
- CocoaPods (for iOS dependencies): `sudo gem install cocoapods`

**Rust targets:**

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

On first build, `cargo tauri ios init` generates the Xcode project under `src-tauri/gen/apple/`. You can then open the project in Xcode for signing configuration:

```bash
open src-tauri/gen/apple/stratum-tauri.xcodeproj
```

### Android Patches

Stratum applies Android patches after `tauri android init` by copying files from `src-tauri/android-patches/` into the generated project. The CI pipeline does this explicitly:

```bash
cp -r src-tauri/android-patches/app/src/main/* src-tauri/gen/android/app/src/main/
```

These patches provide:

- Custom `MainActivity.kt` that enables edge-to-edge and injects system-bar insets as CSS custom properties (see [Safe Area Handling](#safe-area-handling))
- Themed launcher icons
- Custom `AndroidManifest.xml` with storage permissions
- No-action-bar theme

## Debugging

### Android

**Physical device (recommended):**

```bash
# Enable USB debugging on your device
# Connect via USB and verify
adb devices

# Run in development mode
cargo tauri android dev
```

This builds the Rust code for ARM64, packages the APK, installs it on the connected device, and launches it. Hot-reload of the frontend is supported via the Vite dev server on port 5173. The Rust backend must be rebuilt manually on changes (`cargo build -p src-tauri` and reinstall).

**Emulator:**

```bash
# List available avd images
emulator -list-avds

# Start an emulator
emulator -avd Pixel_6_API_34

# Run Tauri with x86_64 target
cargo tauri android dev --target x86_64
```

**Logs:**

```bash
# Filter Tauri/Stratum logs
adb logcat | grep -E '(stratum|libstratum|tauri|RustError)'

# Full logs with timestamps
adb logcat -v time | grep stratum
```

On Android, the Rust `eprintln!` calls that appear in the terminal on desktop are routed through `android_logger`. You can view them via `adb logcat`.

### iOS

**Simulator:**

```bash
cargo tauri ios dev
```

This builds the Rust code, launches the iOS Simulator, and runs the app with Vite hot-reload for the frontend.

**Physical device:**

```bash
cargo tauri ios build
```

Then open the Xcode project, select your team under Signing & Capabilities, and run on your device.

**Logs:**

Use the Xcode console (View > Debug Area > Activate Console) or the simctl tool:

```bash
xcrun simctl spawn booted log stream --level debug | grep stratum
```

## Responsive Design

Stratum uses a two-tier responsive design approach: a `useResponsive` hook for runtime adaptation and a `*.mobile.tsx` / `*.shared.tsx` file pattern for platform-specific component variants.

### The `useResponsive` Hook

Defined in `src/lib/hooks/useResponsive.ts`:

```typescript
import { useState, useEffect } from 'react';

const MOBILE_BREAKPOINT = 768;

export function useResponsive() {
  const [width, setWidth] = useState(
    typeof window !== 'undefined' ? window.innerWidth : 1200
  );

  useEffect(() => {
    const onResize = () => setWidth(window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  return {
    isMobile: width < MOBILE_BREAKPOINT,
    isDesktop: width >= MOBILE_BREAKPOINT,
    width,
  };
}
```

**Usage:**

```typescript
import { useResponsive } from '../lib/hooks/useResponsive';

function MyPanel() {
  const { isMobile, isDesktop } = useResponsive();

  if (isMobile) {
    return <MobileVariant />;
  }

  return <DesktopVariant />;
}
```

The breakpoint is `768px` (tablet portrait width). Below that, the app renders mobile layouts. Above that, desktop layouts. This is a width-based check so it adapts to both mobile phones and resized desktop windows.

### The `*.mobile.tsx` Component Pattern

For complex panels that need significantly different mobile and desktop implementations, use the three-file pattern:

```
src/components/FeaturePanel/
├── index.tsx                 # Desktop/web implementation (imports .shared)
├── FeaturePanel.mobile.tsx   # Mobile variant (imports .shared)
├── FeaturePanel.shared.tsx   # Shared logic, hooks, types
└── FeaturePanel.test.tsx     # Tests
```

The `index.tsx` uses `useResponsive` to conditionally render the correct variant:

```typescript
import { useResponsive } from '../../lib/hooks/useResponsive';
import { FeaturePanelDesktop } from './index';
import { FeaturePanelMobile } from './FeaturePanel.mobile';

export default function FeaturePanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <FeaturePanelMobile />;
  return <FeaturePanelDesktop />;
}
```

The `.shared.tsx` file holds code that both variants use: types, hooks, utility functions, and pure rendering helpers that don't depend on layout.

### CSS for Mobile

The `src/global.css` file includes mobile-specific touch and safe-area handling. The safe-area values are injected by the Android `MainActivity.kt` and the `index.html` touch-device fallback, then mapped to the `--safe-area-*` custom properties used by components (see [Safe Area Handling](#safe-area-handling)):

```css
:root {
  --safe-area-top: var(--safe-area-inset-top, 0px);
  --safe-area-bottom: var(--safe-area-inset-bottom, 0px);
  --safe-area-left: var(--safe-area-inset-left, 0px);
  --safe-area-right: var(--safe-area-inset-right, 0px);
}

/* Touch devices: fall back to CSS env() for system bar insets */
@media (pointer: coarse) {
  :root {
    --safe-area-top: var(--safe-area-inset-top, env(safe-area-inset-top, 0px));
    --safe-area-bottom: var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px));
    --safe-area-left: var(--safe-area-inset-left, env(safe-area-inset-left, 0px));
    --safe-area-right: var(--safe-area-inset-right, env(safe-area-inset-right, 0px));
  }
  .safe-area-container {
    padding-top: max(var(--safe-area-top, 0px), var(--safe-area-fallback-top, 0px));
    padding-bottom: max(var(--safe-area-bottom, 0px), var(--safe-area-fallback-bottom, 0px));
  }
  .safe-area-main {
    padding-top: var(--safe-area-top, 0px);
    -webkit-overflow-scrolling: touch;
    touch-action: pan-y;
  }
}
```

Use the `.safe-area-container` class on a **desktop** panel's root element to avoid notches and system bars. The mobile shell (`MobileLayout.tsx`) does not use this class — it offsets its absolutely-positioned elements inline (see [Safe Area Handling](#safe-area-handling) for why):

```typescript
function MyPanel() {
  return (
    <div className="safe-area-container">
      {/* content */}
    </div>
  );
}
```

## Platform-Specific Code

### Rust: `#[cfg()]` Attributes

Stratum uses Rust's conditional compilation to handle platform-specific behavior. The main patterns are:

**`#[cfg(target_os = "android")]`**: Code that only runs on Android.

Example from `src-tauri/src/commands/vault.rs` (Android content URI resolution):

```rust
/// Resolve a user-picked path to a real filesystem path.
/// On Android, converts SAF content URI to a real path.
#[cfg(target_os = "android")]
fn resolve_picked_path(picked: &str) -> Result<PathBuf, String> {
    let path_encoded = picked
        .split("/tree/")
        .nth(1)
        .ok_or_else(|| format!("Could not parse Android content URI: {}", picked))?;
    let path_decoded = percent_decode(path_encoded);

    if let Some(subpath) = path_decoded.strip_prefix("primary:") {
        Ok(PathBuf::from("/storage/emulated/0").join(subpath))
    } else if let Some((volume, subpath)) = path_decoded.split_once(':') {
        Ok(PathBuf::from("/storage").join(volume).join(subpath))
    } else {
        Err(format!("Unrecognized content URI format: {}", picked))
    }
}

#[cfg(not(target_os = "android"))]
fn resolve_picked_path(picked: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(picked))
}
```

**`#[cfg(desktop)]`**: Commands or code that should only be registered on desktop.

Example from `src-tauri/src/lib.rs`:

```rust
#[cfg(desktop)]
commands::vault::pick_vault_directory,
```

The `pick_vault_directory` command uses `tauri_plugin_dialog` for a native folder picker, which isn't available on mobile. On mobile, the frontend uses the File System Access API or Android's Storage Access Framework directly.

**`#[cfg(not(target_os = "android"))]`**: The inverse. Code for all platforms except Android.

**`#[cfg_attr(mobile, tauri::mobile_entry_point)]`**: The mobile entry point attribute.

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ...
}
```

This attribute marks the `run()` function as the entry point Tauri calls on mobile platforms.

**Default vault path resolution** differs by platform:

```rust
fn resolve_default_vault_path(_app: &tauri::AppHandle) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        _app.path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("vault"))
            .join("StratumVault")
    }

    #[cfg(not(target_os = "android"))]
    {
        dirs::home_dir()
            .map(|h| h.join("StratumVault"))
            .or_else(|| std::env::current_dir().ok().map(|d| d.join("vault")))
            .unwrap_or_else(|| PathBuf::from("vault"))
    }
}
```

On Android, the vault lives in the app's private data directory. On desktop, it defaults to `~/StratumVault`.

### TypeScript: Platform Detection

For frontend platform detection, use Tauri's platform API rather than user-agent sniffing:

```typescript
import { platform } from '@tauri-apps/plugin-platform';

// In an async context
const os = await platform.os(); // 'android', 'ios', 'linux', 'macos', 'windows'
```

For simple responsive layout decisions, prefer `useResponsive` over platform checks. Platform checks are only needed when the behavior difference is fundamental (e.g., file picker API differences), not layout.

### Android Content URI Handling

Android's Storage Access Framework (SAF) returns content URIs like `content://com.android.externalstorage.documents/tree/primary%3ADocuments`. These need special handling:

1. The frontend invokes `init_vault` with the path string from the SAF picker
2. The Rust command calls `resolve_picked_path()` which parses the URI
3. On Android, it decodes percent-encoding and converts `primary:` paths to `/storage/emulated/0/`
4. On desktop, it passes the path through unchanged

## Lifecycle

### Save on Suspend

Mobile operating systems can kill your app at any time when it is in the background. Stratum handles this with a save-on-suspend pattern:

- The editor auto-saves content to the SQLite block store on every change (debounced at 500ms)
- On Android, the `MainActivity` receives the system's `onPause`/`onStop` lifecycle events
- Before the app goes to the background, the frontend must flush any pending saves

The Tauri v2 mobile runtime emits the `tauri://close-requested` event when the app is being suspended. The frontend listens for this:

```typescript
import { getCurrentWindow } from '@tauri-apps/api/window';

// In App.tsx or editor container
const appWindow = getCurrentWindow();
appWindow.onCloseRequested(async () => {
  await flushPendingSaves();
});
```

### Restore on Resume

When the app returns to the foreground:

- The SQLite database is reopened (it persists across background/foreground cycles on most devices)
- The index engine reinitializes from the existing database. No rebuild needed.
- The last-opened page is restored from `appStore` (which persists state in `localStorage`)
- Sync state is rechecked

### Android-Specific Lifecycle

The `MainActivity.kt` in `src-tauri/android-patches/` handles Android-specific lifecycle needs. On startup it enables edge-to-edge, registers that runtime edge-to-edge listeners report the system-bar insets, and schedules safe-area injection retries (100/500/1500/5000 ms) to cover the Activity-start / WebView-init race:

```kotlin
class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        ViewCompat.setOnApplyWindowInsetsListener(window.decorView) { _, insets ->
            injectSafeArea(insets)
            insets
        }
        scheduleSafeAreaInjection()
    }
}
```

The safe area injection reads the system-bar + display-cutout insets and sets `--safe-area-inset-top` / `--safe-area-inset-bottom` (in CSS pixels) on the webview document so the frontend can account for status bars, notches, and navigation bars. See [Safe Area Handling](#safe-area-handling) for the full implementation and consumption.

## Known Issues

### Android

| Issue | Description | Workaround |
|---|---|---|
| **Content URI parsing** | SAF content URI format varies by manufacturer (Samsung, Xiaomi, etc. may differ from stock Android) | The `resolve_picked_path` function handles standard formats. File bugs for manufacturer-specific URI patterns. |
| **File watcher disabled** | `pkm-watcher` (inotify-based) does not work on Android's filesystem due to permission restrictions | Rely on manual reindex (`reindex_vault` command) or periodic polling. |
| **Git sync limitations** | SSH key storage on Android is not fully supported. HTTPS sync with credential helpers works but is untested. | Use manual sync mode. Auto-sync may not be reliable. |
| **Large vaults on low-RAM devices** | Vaults with 10k+ blocks may cause memory pressure on devices with less than 4GB RAM | The Tantivy index is memory-mapped. SQLite works well under constraints. Most issues come from the frontend rendering large graphs. |
| **Soft keyboard overlap** | The editor may not always adjust correctly when the soft keyboard appears | `android:windowSoftInputMode="adjustResize"` is set in the manifest. Use `touch-action: pan-y` to allow scroll while editing. |

### iOS

| Issue | Description | Workaround |
|---|---|---|
| **Simulator only** | CI builds only produce simulator binaries | Local builds with a developer account can produce device binaries. |
| **WebKit limits** | iOS WebKit may impose stricter memory limits on web content than desktop | Keep DOM size reasonable. Lazy-render large lists. |
| **Keyboard handling** | Hardware keyboard support on iPad is limited | Software keyboard mode is the primary input method. |
| **Background execution** | iOS aggressively suspends background apps | Save-on-suspend handling is critical. Test thoroughly. |

### Cross-Platform

| Issue | Description | Workaround |
|---|---|---|
| **Window count** | Tauri mobile only supports single-window mode | The app uses one window. Desktop features that rely on multiple windows are unavailable on mobile. |
| **Plugin availability** | Not all Tauri plugins support mobile targets | Check plugin documentation before adding new plugins. `tauri-plugin-dialog` is desktop-only. Use platform-detection to conditionally register commands. |
| **File system access** | SAF on Android vs POSIX paths on desktop are fundamentally different | Always use `resolve_picked_path()` or abstract file access behind a command wrapper. |

## Testing

### On a Physical Device (Android)

**Development build:**

```bash
# With one device connected via USB
cargo tauri android dev
```

This builds, installs, and launches the app. Use `adb logcat` to view Rust logs.

**Release APK:**

```bash
cargo tauri android build --target aarch64 --apk
```

The APK is output to `src-tauri/gen/android/app/build/outputs/apk/`. Install it with:

```bash
adb install src-tauri/gen/android/app/build/outputs/apk/release/stratum.apk
```

### On an Emulator (Android)

```bash
# Start emulator first
emulator -avd Pixel_6_API_34 -no-snapshot

# Build and deploy for x86_64 (much faster for emulator)
cargo tauri android dev --target x86_64
```

Emulators with x86_64 targets are significantly faster for Rust compilation because they avoid ARM cross-compilation. Use this for rapid iteration on the Rust backend.

### Verifying Safe Area Handling (Android)

The status-bar offset is not exercised by `npm run test` (which runs in a plain browser layout). To verify a safe-area change against the real Android WebView:

1. Build and deploy a debug APK to an emulator booted with a status bar / gesture-nav (e.g. `Pixel_6_API_34`):

   ```bash
   cargo tauri android dev --target x86_64
   ```

2. Confirm the app draws edge-to-edge and the mobile top bar is *not* under the status bar. The easiest live check is a CDP probe over the adb-forwarded WebView debugging socket (a working example lives in `verification/cdp_probe.mjs` from the status-bar fix). It asserts that `getBoundingClientRect().top` of the top bar equals the injected `--safe-area-inset-top` and that the bottom nav fits within the viewport.

3. The expected values on the reference emulator are:

   ```json
   {
     "safeAreaTop": "48.761906px",
     "safeAreaBottom": "24.0px",
     "topBarTopPx": 48.761905670166016,
     "bottomNavFitsViewport": true
   }
   ```

   The exact pixel values vary by device/emulator; the invariant is that `topBarTop == safeAreaTop` (top bar offset by the status-bar inset, nothing drawn behind it) and `bottom == safeAreaBottom` (bottom nav sits above the gesture/navigation bar).

4. Record a `screencap` and the probe result as review evidence, matching the pattern in `verification/` (see the `c5f3d7c` commit for the recorded evidence).

### On the Simulator (iOS)

```bash
# Apple Silicon Macs (default)
cargo tauri ios dev

# Intel Macs (specify x86_64 simulator)
cargo tauri ios dev --target x86_64-apple-ios
```

### Automated Testing

The CI pipeline builds Android on every pull request and push to `master` (a debug APK, uploaded as the `android-debug-apk` artifact for the smoke-test gate) and on every tagged release (`v*`, see `.github/workflows/ci.yml`). The `android` job:

1. Sets up Android SDK and NDK
2. Adds all Android Rust targets
3. Runs `tauri android init`
4. Applies Android patches
5. On tags: runs `tauri android build --target aarch64 --apk --aab` to produce a **release** APK and AAB **signed with the release keystore from CI secrets**
6. On PRs/pushes: runs `tauri android build --debug --target aarch64 x86_64 --apk` to produce a **debug** APK for the CI gate and on-device smoke test (the emulator is x86_64, so the debug APK must ship the x86_64 ABI)
7. Uploads the APK/AAB (tags) or debug APK (PRs/pushes) as build artifacts

A separate `android-smoke` job (PRs / pushes to master) then boots a headless x86_64 emulator (API 34), installs the debug APK, launches the app and waits for the first rendered frame. GitHub-hosted Linux runners do not expose a working `/dev/kvm` to the runner user, so the emulator runs in software-emulation mode (`-accel off`); the smoke script is deliberately robust to that (see `smoke-android.sh` header) and treats the Android platform's own `Displayed` first-frame signal as the pass criterion, retrying the launch when the unaccelerated system force-finishes a healthy activity. It archives a screenshot (`screencap`) and a logcat dump as `android-smoke-evidence` so reviewers have proof the app starts, and it **fails the run** if the app crashes (process death) or never reaches a first frame within the timeout. No physical device is required — the emulator runs on the same GitHub-hosted runner.

The `ios` job:

1. Installs iOS Rust targets
2. Runs `tauri ios init` (generates `src-tauri/gen/apple` on first run)
3. Patches the Xcode project for Tauri compatibility
4. Runs `tauri ios build --debug --target aarch64-sim --ci`
5. Zips the built app
6. Uploads as a build artifact

iOS is built **for the simulator only** (`aarch64-sim`) — this requires **no Apple
Developer signing certificate or provisioning profile**, so it runs on every
pull request and push to `master` as a CI gate (debug build, artifact
`ios-sim-app`) as well as on tagged releases (`v*`, release build, artifact
`ios`). The `test` job must pass first. Physical-device builds still require a
paid Apple Developer account and are done locally by developers.

The Android release-signing keystore secrets (`ANDROID_KEYSTORE`,
`ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`) must
be set on the repository to sign release artifacts; the debug APK does not need
them.

### Manual Test Checklist

Before shipping a mobile change, verify:

- [x] App loads and draws its first frame on Android — covered automatically by the `android-smoke` CI job (headless emulator)
- [ ] App launches on iOS simulator
- [ ] Vault creation and opening works
- [ ] Block editor loads and saves content
- [ ] Wiki-link autocomplete works
- [ ] Search returns results
- [ ] Graph view renders (may be slow on low-end devices)
- [ ] Back button / gesture navigation works correctly
- [ ] Keyboard does not obscure the editor
- [ ] App recovers from backgrounding (save + restore)
- [ ] Orientation changes don't break layout
- [ ] Safe area insets are respected on notched devices

---

## Vault Storage on Android

On Android, the vault is stored in the app's **private internal storage** at `/data/user/0/app.stratum/StratumVault/`. This is necessary because:

- **SQLite** (`rusqlite`) requires a real filesystem path — it cannot work with Android SAF `content://` URIs
- **git** (`gix`/gitoxide) also requires a real filesystem path for the `.git/` directory
- Android's scoped storage (API 30+) blocks raw filesystem writes to `/storage/emulated/0/`

The vault directory contains:

```
/data/user/0/app.stratum/StratumVault/
├── .pkm/
│   ├── blocks.db           # SQLite block store
│   ├── config.toml         # App configuration
│   └── search.idx          # Tantivy search index
├── journals/
│   └── 2025-01-01.md       # Daily notes
├── pages/
│   └── My Note.md          # User pages
└── .git/                   # Git repository (via gix)
```

### Why not external/SD storage?

Android's scoped storage (API 30+, ~95% of active devices) prevents apps from writing to arbitrary external storage paths using `std::fs`. The Storage Access Framework (SAF) provides `content://` URIs, but:
- `rusqlite` cannot open databases from SAF URIs
- `gix` cannot initialize git repositories on SAF URIs

**Export/import** commands use the `tauri-plugin-android-fs` SAF APIs to let users back up or restore their vault to/from external storage.

### Backup

- **Android Auto Backup**: The vault is automatically backed up to Google Drive (when enabled on the device). This is controlled by `android:allowBackup="true"` in `AndroidManifest.xml`.
- **Git remote**: Configure a git remote in Settings → Sync to push your vault to GitHub/GitLab. Clone on desktop for full `.md` file access.
- **Manual export**: Use the export command (planned) to copy vault files to a user-selected SAF location.

---

## Building APKs

### Debug APK (signed, for testing)

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.1.12297006"
export NDK="$ANDROID_NDK_HOME"
export PATH="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"

# Set cross-compilation environment variables for all targets
export CC_aarch64_linux_android="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"
export AR_aarch64_linux_android="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
export CC_x86_64_linux_android="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android21-clang"
export AR_x86_64_linux_android="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
export CC_armv7_linux_androideabi="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi21-clang"
export AR_armv7_linux_androideabi="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
export CC_i686_linux_android="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/i686-linux-android21-clang"
export AR_i686_linux_android="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
export JAVA_HOME="/usr/lib/jvm/java-21-openjdk"

# Copy frontend assets and Android patches
cp -r dist/. src-tauri/gen/android/app/src/main/assets/
cp -r src-tauri/android-patches/app/src/main/* src-tauri/gen/android/app/src/main/

# Build signed debug APK
npx tauri android build --debug --target aarch64 --apk
```

The debug APK is output at:
```
src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

**Important**: The debug APK uses `applicationIdSuffix = ".debug"` so it installs as `app.stratum.debug` alongside any release build. It is auto-signed with the debug keystore at `~/.android/debug.keystore`.

### Release APK (signed, for distribution)

```bash
npx tauri android build --target aarch64 --apk
```

Release APKs and AABs are signed at build time by Gradle using the release signing config in `src-tauri/android-patches/app/build.gradle.kts`, which reads the keystore credentials from the environment (set by CI from GitHub secrets). For local release builds, set these before building:

```bash
export ANDROID_KEYSTORE=/path/to/release.keystore
export ANDROID_KEYSTORE_PASSWORD=...
export ANDROID_KEY_ALIAS=...
export ANDROID_KEY_PASSWORD=...
```

If these are not set, the signing config falls back to the checked-in debug keystore (`src-tauri/android-patches/debug.keystore`) so local/CI builds still succeed.

### APK Signing

Release signing happens inside the Gradle build (see `signingConfigs.release` in `src-tauri/android-patches/app/build.gradle.kts`). The CI workflow decodes the base64 `ANDROID_KEYSTORE` secret to a keystore file and exports the signing env vars, so release artifacts are signed with the production keystore. The signing config enables V1, V2, and V3 signing:

- V2 signing is **required** for Android 11+ (API 30+) when `targetSdkVersion >= 30`.
- V1 provides backward compatibility for older Android versions.
- V3 supports key rotation.

---

## Safe Area Handling

Android's edge-to-edge display mode renders the WebView behind the system bars (status bar and gesture/navigation bar). Stratum handles the resulting screen insets entirely in the Android patch layer and the frontend. There is **no** edge-to-edge or safe-area switch in `tauri.conf.json`; edge-to-edge is enabled programmatically in the patched `MainActivity.kt`.

### 1. Required configuration (`src-tauri/tauri.conf.json`)

The `bundle.android` section only pins the SDK floor and the debug applicationId suffix:

```json
"android": {
  "minSdkVersion": 26,
  "debugApplicationIdSuffix": ".debug"
}
```

- `minSdkVersion: 26` is required because the audio backend uses AAudio (API 26+). Do not lower it.
- `debugApplicationIdSuffix: ".debug"` makes debug builds install as `app.stratum.debug`, alongside any release build.
- The release `applicationId` is derived from the `identifier` (`app.stratum`).
- **Edge-to-edge is not a config key.** It is enabled in the patched `MainActivity.kt` (below). The Android patches under `src-tauri/android-patches/` are copied over the generated `src-tauri/gen/android` project before building — both locally and by CI.

### 2. `MainActivity.kt` — system-inset injection

The patched activity at `src-tauri/android-patches/app/src/main/java/app/stratum/MainActivity.kt`:

- Calls `enableEdgeToEdge()` so the WebView draws behind the system bars.
- Registers an `OnApplyWindowInsetsListener` on the decor view that captures `systemBars() + displayCutout()` insets and injects them.
- Re-injects on a timer via `scheduleSafeAreaInjection()` (delays of 100, 500, 1500, and 5000 ms) to survive the race between Activity start and WebView initialization.
- `injectSafeArea()` walks the view tree to find the `WebView`, converts the raw pixel insets to CSS pixels using the display density, and sets the CSS custom properties `--safe-area-inset-top` and `--safe-area-inset-bottom` on `document.documentElement` via `evaluateJavascript()`.

```kotlin
override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    ViewCompat.setOnApplyWindowInsetsListener(window.decorView) { _, insets ->
        injectSafeArea(insets)
        insets
    }
    scheduleSafeAreaInjection()
}

private fun injectSafeArea(insets: WindowInsetsCompat) {
    val wv = findWebView() ?: return
    val sb = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or
        WindowInsetsCompat.Type.displayCutout()
    )
    val topDp = sb.top / resources.displayMetrics.density
    val bottomDp = sb.bottom / resources.displayMetrics.density
    val js = "(function(){" +
        "var s=document.documentElement.style;" +
        "s.setProperty('--safe-area-inset-top','${topDp}px');" +
        "s.setProperty('--safe-area-inset-bottom','${bottomDp}px');" +
        "})()"
    wv.evaluateJavascript(js, null)
}
```

### 3. `index.html` — viewport meta + touch-device fallback

The viewport meta is set to `viewport-fit=cover` so the WebView extends into the safe areas:

```html
<meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover" />
```

An inline script in `index.html` provides a **fallback** for when the Kotlin injection has not run. It only acts on touch-capable clients (`ontouchstart` in window or `navigator.maxTouchPoints`), does nothing if `--safe-area-inset-*` is already set (so it never clobbers the Kotlin values), and otherwise derives the insets from `visualViewport` (capped to 60 CSS px, ignoring browser-chrome offsets on desktop).

### 4. CSS variables (`src/global.css`)

`src/global.css` maps the injected/derived insets to the `--safe-area-*` custom properties the components consume:

```css
:root {
  --safe-area-top: var(--safe-area-inset-top, 0px);
  --safe-area-bottom: var(--safe-area-inset-bottom, 0px);
  --safe-area-left: var(--safe-area-inset-left, 0px);
  --safe-area-right: var(--safe-area-inset-right, 0px);
}

/* Touch devices: fall back to env() when the injected variable is absent */
@media (pointer: coarse) {
  :root {
    --safe-area-top: var(--safe-area-inset-top, env(safe-area-inset-top, 0px));
    --safe-area-bottom: var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px));
    --safe-area-left: var(--safe-area-inset-left, env(safe-area-inset-left, 0px));
    --safe-area-right: var(--safe-area-inset-right, env(safe-area-inset-right, 0px));
  }
}
```

- On touch devices, `.safe-area-container` uses `padding-top/bottom: max(var(--safe-area-top), var(--safe-area-fallback-top))` and `.safe-area-main` uses `padding-top: var(--safe-area-top, 0px)`. The `--safe-area-fallback-*` and `env()` fallback only apply inside the `pointer: coarse` media query — desktop components never see them.
- **Desktop (pointer: fine) deliberately avoids `env()`** because WebKitGTK / some compositors report non-zero safe-area values on desktop, which would break layouts.

### 5. Component offsets

- **Desktop** (`App.tsx`): the root Box uses `className="safe-area-container"` and the `<main>` element uses `className="safe-area-main"`.
- **Mobile shell** (`MobileLayout.tsx`): does **not** use the padding class. Its absolutely-positioned top bar, error banner, and content area are offset with inline styles using `var(--safe-area-top, 0px)` (via the `SAFE_AREA_TOP` constant), because a padding-based container class cannot offset absolutely-positioned children. On desktop this variable resolves to `0`, so it is a no-op outside devices with system bars.
- **Landing / empty state** (`VaultPicker.tsx`): reserves the insets with `paddingTop: 'var(--safe-area-top, 0px)'` and `paddingBottom: 'var(--safe-area-bottom, 0px)'`.
- **Bottom navigation** (`MobileNav.tsx`): the fixed bottom `Paper` sets `paddingBottom: 'var(--safe-area-bottom)'` so the gesture/navigation bar is not overlapped.

---

## Troubleshooting

| Problem | Likely Cause | Fix |
|---------|-------------|-----|
| `Operation not permitted` writing to vault | Android scoped storage — vault must be in private data dir | Vault auto-creates in `/data/user/0/app.stratum/`. Do NOT use folder picker for vault location. |
| `package invalid` on APK install | APK is unsigned or uses wrong signature scheme | Build with `--debug` flag, or sign with `apksigner --v2-signing-enabled true` |
| `INSTALL_FAILED_INVALID_APK` | Architecture mismatch | Build with correct `--target` for your device (`aarch64` for most phones) |
| App content behind system bars | A component draws over the status bar / gesture bar on Android | Components must consume the safe-area insets. The mobile shell, landing view, and bottom nav already offset via inline styles using `--safe-area-top` / `--safe-area-bottom` (see [Safe Area Handling](#safe-area-handling)). New full-screen mobile components must do the same. |
| WebView safe area values are 0px | The activity's inset injection or the `index.html` fallback did not run before the frame captured | Rely on the Kotlin `MainActivity` injection (retries at 100/500/1500/5000 ms) plus the `index.html` touch-device `visualViewport` fallback. Desktop should report `0px` by design — the `env()` fallback is gated to `pointer: coarse` so compositor-reported inset values do not break desktop layouts. |
