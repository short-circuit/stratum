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

- Custom `MainActivity.kt` with edge-to-edge display and safe area injection
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

The `src/global.css` file includes mobile-specific touch handling:

```css
/* Safe area integration (injected by Android MainActivity / iOS WebKit) */
:root {
  --safe-area-top: var(--safe-area-inset-top, env(safe-area-inset-top, 0px));
  --safe-area-bottom: var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px));
}

/* Coarse pointer = touch device — prevent scroll conflicts with editor */
@media (pointer: coarse) {
  .blocknote-editor-container {
    touch-action: pan-y;
  }
  .bn-editor {
    touch-action: pan-y;
  }
}
```

Use the `.safe-area-container` class on your panel's root element to avoid notches and system bars:

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

The `MainActivity.kt` in `src-tauri/android-patches/` handles Android-specific lifecycle needs:

```kotlin
class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        WindowCompat.setDecorFitsSystemWindows(window, false)
        super.onCreate(savedInstanceState)
        scheduleSafeAreaInjection()
    }

    private fun scheduleSafeAreaInjection() {
        // Injects safe area insets as CSS custom properties
        // Retries at 100ms, 500ms, and 1500ms to handle race conditions
    }

    private fun injectSafeArea() {
        // Reads system bar insets and sets CSS variables:
        // --safe-area-inset-top, --safe-area-inset-bottom
    }
}
```

The safe area injection sets CSS custom properties on the webview document so the frontend can account for status bars, notches, and navigation bars.

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

### On the Simulator (iOS)

```bash
# Apple Silicon Macs (default)
cargo tauri ios dev

# Intel Macs (specify x86_64 simulator)
cargo tauri ios dev --target x86_64-apple-ios
```

### Automated Testing

The CI pipeline runs Android and iOS builds on every tagged release (see `.github/workflows/ci.yml`). The `android` job:

1. Sets up Android SDK and NDK
2. Adds all Android Rust targets
3. Runs `tauri android init`
4. Applies Android patches
5. Runs `tauri android build --target aarch64 --apk`
6. Signs the APK (if keystore is configured)
7. Uploads the APK and AAB as build artifacts

The `ios` job:

1. Installs iOS Rust targets
2. Runs `tauri ios init` (if not already initialized)
3. Patches the Xcode project for Tauri compatibility
4. Runs `tauri ios build --target aarch64-sim --ci`
5. Zips the built app
6. Uploads as a build artifact

Both jobs run on tag pushes (`v*`) and require the `test` job to pass first.

### Manual Test Checklist

Before shipping a mobile change, verify:

- [ ] App launches on Android (physical device or emulator)
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
