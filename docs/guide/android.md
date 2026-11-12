# Android app

Stratum ships a native Android build of the GPUI shell. It is the same Rust
frontend as the desktop app — one code base, the same vault format, the same
service layer — hosted by an Android `NativeActivity` that hands GPUI a Vulkan
surface, the soft keyboard and the system window insets.

The frozen React application in `src/` remains the parity reference; the Android
build is the native path that replaces it.

## Requirements

- Android 8.0 (API 26) or newer on a device with Vulkan or OpenGL ES 3.0.
- `arm64-v8a` devices are the shipped target; `x86_64` builds are produced for
  emulators.
- Roughly 60 MB of free storage for a release APK (debug builds with full
  DWARF symbols are considerably larger; `scripts/build-android.sh --debug`
  strips them before packaging).

## Install

Install a release APK:

```bash
adb install -r stratum-android.apk
```

Or build one from source (see [Build from source](#build-from-source)).

The launcher entry is **Stratum**. On first launch the app creates an empty
vault in its private documents directory (`StratumVault`) and opens the journal.

## Navigation

The mobile shell is a bottom navigation layout:

- **Journal**, **Search**, **Graph** — the three primary destinations.
- **More** — every other surface: pages, query, Ask Notes, templates,
  flashcards, kanban, reminders, whiteboards, plugins and settings.

The Android back gesture pops the current sheet or returns to the previous
destination. Panel content is laid out inside the reported safe area, so the
status bar, display cutout, gesture bar and the on-screen keyboard never cover
it.

## Permissions

| Permission | Asked when | Used for |
|---|---|---|
| Microphone | First dictation or voice capture | Voice notes and dictation |
| Notifications | First reminder is scheduled | Reminder alerts |
| Internet | Never asked (normal permission) | AI providers, plugin downloads, sync |

Denying the microphone leaves the rest of the app fully usable; dictation can be
retried later from **Settings → AI → Voice dictation**.

## Attachments and imports

Picking a file (attachment upload, vault import) opens the Android system file
picker through the Storage Access Framework, so no broad storage permission is
required.

## Lifecycle

The shell saves the open editor when the app goes to the background and restores
the last route when it comes back. The GPU surface is released while the app is
not visible and re-created on resume, which is what keeps the process alive
without burning battery.

## Build from source

The Android host lives in `crates/stratum-mobile`, on top of the vendored
`vendor/gpui-mobile` platform layer. One script builds both the native library
and the APK:

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.1.12297006"
./scripts/build-android.sh
```

The script runs `cargo ndk` for `arm64-v8a` and `x86_64`, then Gradle
`assembleDebug`, and prints the APK path. Use `--install` to push it to the
connected device and `--debug` to skip the release profile.

## Verifying a build

```bash
adb shell setprop debug.stratum.gpu gl   # emulator only: SwiftShader GL instead of Vulkan
adb install -r crates/stratum-mobile/android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n com.stratum.app/dev.gpui.mobile.GpuiActivity
adb logcat -s stratum
```

A healthy boot logs the platform initialisation, the vault path and the window
open in that order. `adb exec-out screencap -p > mobile.png` captures the running
shell; the repository's own capture set is produced on the desktop host by
`scripts/capture-gpui-mobile.sh` at the device resolution (1080×2400).

## Known limits

- iOS is not shipped yet; the platform layer is trait-based, so an iOS host can
  be added without touching the shell.
- The kanban board surface is not ported to the native shell yet; the mobile
  build shows the shipped placeholder for that destination.
