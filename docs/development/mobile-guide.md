# Mobile Development Guide

Stratum ships a native Android application. It is the same Rust shell as the
desktop build — one code base, the same vault format, the same service layer —
hosted by an Android `NativeActivity` that hands GPUI a Vulkan surface, the soft
keyboard and the system window insets. There is **no webview, no JavaScript, no
npm toolchain and no IPC bridge**.

The React + Tauri mobile build was the parity reference until the E12 cutover; it
has been deleted.

---

## Architecture

```
crates/stratum-mobile/          Android host crate (stub on non-Android hosts)
├── src/android/mod.rs          android_main: logger, panic hook, platform init, window bootstrap
├── src/android/vault.rs        vault location under the app documents dir + first-run layout
├── src/android/services.rs     MobileServices impl (mic, file picker, notifications, clipboard, links)
└── android/                    Gradle project (cargo-ndk output → jniLibs → APK/AAB)

crates/stratum-ui/src/mobile/   shared mobile shell
├── insets.rs                   MobileFrame: top/right/bottom/left/ime, frame_from_insets
├── nav.rs                      bottom tabs (Journal/Search/Graph/More), More sheet, back stack
├── services.rs                 MobileServices host seam + NoopServices
└── variants/                   one wrapper per router::Panel

vendor/gpui-mobile/             vendored gpui-pre Android/iOS platform layer
```

The pure models (safe-area frame, nav tables, tray menu, shortcut grammar,
notification text, deep-link parsing, drop classification, panel variants) build
on every target; only the OS halves sit behind `cfg(desktop)` or the Android gate.

---

## Requirements

- Android 8.0 (API 26) or newer on a device with Vulkan or OpenGL ES 3.0.
- `arm64-v8a` devices are the shipped target; `x86_64` builds exist for
  emulators.
- Rust with the `aarch64-linux-android` and `x86_64-linux-android` targets.
- Android SDK (`ANDROID_HOME`, default `$HOME/Android/Sdk`), NDK
  (`ANDROID_NDK_HOME`, default `$ANDROID_HOME/ndk/27.1.12297006`), JDK 17 and
  `cargo-ndk` 4.1.2.
- The Linux host libraries from `.github/actions/install-linux-deps` (the
  vendored platform layer links against them).

```bash
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk --version 4.1.2 --locked
```

---

## Building

```bash
# Release .so + debug APK (developer build)
scripts/build-android.sh

# Debug .so + debug APK
scripts/build-android.sh --debug

# Release .so + signed APK + AAB (release build)
scripts/build-android.sh --release

# Build, then install on the connected device
scripts/build-android.sh --install
```

The script cross-compiles `libstratum_mobile.so` for both ABIs into
`crates/stratum-mobile/android/app/src/main/jniLibs`, runs Gradle, and verifies
that both ABIs are inside the APK. Artifacts land under
`crates/stratum-mobile/android/app/build/outputs/{apk,bundle}/`.

`CARGO_NDK_PLATFORM` is pinned to API 26 because `libnativewindow.so` is absent
from the API 21 sysroot. The Gradle project pins `applicationId
"com.stratum.app"` and `minSdk 26` (AAudio requires API 26; do not lower it).

### Release signing

Gradle reads the signing material from the environment:

| Variable | Purpose |
|---|---|
| `ANDROID_KEYSTORE_PATH` | PKCS#12/JKS keystore with the upload key |
| `ANDROID_KEYSTORE_PASSWORD` | keystore password |
| `ANDROID_KEY_ALIAS` | key alias inside the keystore |
| `ANDROID_KEY_PASSWORD` | key password |
| `STRATUM_ANDROID_VERSION_NAME` | versionName override |
| `STRATUM_ANDROID_VERSION_CODE` | versionCode override |

When `ANDROID_KEYSTORE_PATH` is unset the release build falls back to the
checked-in debug key so it still runs on a developer machine, and the script says
so. CI decodes the base64 keystore secret into a file and exports the same
variables; V1, V2 and V3 signing are enabled in the Gradle config.

---

## Running and testing on a device

```bash
adb install -r crates/stratum-mobile/android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n com.stratum.app/.MainActivity
adb logcat | grep -E '(stratum|libstratum|RustError)'
```

On Android the `eprintln!` calls that appear in the terminal on desktop are
routed through `android_logger`, so `adb logcat` shows them.

`.github/scripts/smoke-android.sh` is the CI smoke test: it installs the APK,
launches it, waits for the platform's own first-frame signal
(`ActivityTaskManager: Displayed …`) and captures a screenshot plus logcat as
evidence. It fails on install errors, process death without a first frame, or a
timeout.

### Vault location

`StratumVault::location` resolves `documents_directory()` from the vendored
platform layer and falls back to the temporary directory when documents is
unavailable; `StratumVault::open` creates the first-run layout under
`…/StratumVault`. The vault must live on a real filesystem path because SQLite
(`rusqlite`) and git cannot operate on SAF `content://` URIs. The onboarding
wizard's vault picker uses the Storage Access Framework to pick a folder, and the
path is resolved before the vault is opened.

### Capabilities

| Capability | Implementation |
|---|---|
| Microphone (dictation, voice capture) | runtime `RECORD_AUDIO` request through the platform layer, then `AudioRecord` |
| Reminder notifications | `NotificationManager` channel created on first use |
| File picker (attachments, import) | Storage Access Framework picker |
| `stratum://` deep links | intent filter on the host activity; routes through the shared parser |
| Clipboard | Android clipboard service |
| URL opener | `Intent.ACTION_VIEW` |

---

## Layout and safe areas

`crates/stratum-ui/src/mobile/insets.rs` owns the geometry:

- `MobileFrame` carries top/right/bottom/left **and** the IME height. The IME
  height is deliberately separate from the bottom safe-area edge, because a
  keyboard overlay must not shift the home-indicator padding.
- `frame_from_insets` parses the `STRATUM_INSETS` capture override
  (`top,right,bottom,left,ime`).
- `frame_for_window` derives the frame from the real window and the platform
  insets the Android host reports.

The bottom navigation exposes four tabs (Journal, Search, Graph, More) and a
ten-row More sheet. Every touch target is at least 44 px. `mobile/nav.rs` and the
`MOBILE_I18N_KEYS` catalog gate pin the chrome, and the mobile contract tests
cover the nav, the sheet, the back stack and the frame math without opening a
window.

`Shell::mobile_back` handles the back gesture. `MobileServices` is the host seam
`AndroidServices` implements (microphone, picker, notifications, clipboard,
opener); tests use `NoopServices`.

### Verifying a safe-area change

The geometry is testable without a device:

```bash
STRATUM_MOBILE=1 STRATUM_INSETS=48,0,72,0,0 ./scripts/capture-gpui-mobile.sh
```

The capture pins the status-bar, home-indicator and gesture-area frame and
produces exactly 1080×2400 PNGs. `STRATUM_INSETS` takes
`top,right,bottom,left,ime`; the IME value must move content without touching the
bottom edge.

---

## Capturing the mobile surface

```bash
./scripts/capture-gpui-mobile.sh
```

The script owns a private 1080×2400 Xvfb screen and captures fifteen states
(journal, calendar, page, search, graph, More sheet, Ask Notes, Query, Templates,
Flashcards, Kanban, Reminders, Whiteboards, Plugins, Settings) into
`dist/gpui-mobile-*.png`, and fails unless every PNG is exactly 1080×2400. It
sets `STRATUM_MOBILE=1` and `STRATUM_INSETS=48,0,72,0,0` so the geometry matches
the Android host without a device. `scripts/verify-gpui-goldens.sh` diffs the
same captures against `goldens/gpui/gpui-mobile-*.png`.

---

## CI

`.github/workflows/ci.yml`:

- `android` — cross-compiles both ABIs with `scripts/build-android.sh` and
  uploads the debug APK.
- `android-smoke` — boots a headless x86_64 emulator (API 34), installs that APK,
  launches the app and waits for the first rendered frame, archiving a
  screenshot and logcat as `android-smoke-evidence`. GitHub-hosted Linux runners
  have no `/dev/kvm`, so the emulator runs unaccelerated; the smoke script
  tolerates the platform's input-dispatch watchdog and retries the launch when
  the system force-finishes a healthy activity.

Release artifacts (signed APK + AAB) are produced by
`.github/workflows/gpui-release.yml` on `v*` tags.

---

## Limitations

| Area | Limit | Notes |
|---|---|---|
| Window count | single window | The mobile shell renders one window; desktop-only surfaces (tray, Quick Capture shortcut) are not shown. |
| iOS | not shipped | The vendored `gpui-mobile` layer carries iOS support, but `stratum-mobile` currently packages Android only. |
| File watcher | unavailable | `pkm-watcher` (inotify) does not work on Android's filesystem; rely on manual reindex or periodic polling. |
| Emulator performance | software Vulkan | Runners have no KVM, so the smoke test runs on a software GPU. |
| Text input | IME only | Hardware keyboards work through the same IME path; there is no desktop keymap on mobile. |

---

## Troubleshooting

| Problem | Likely Cause | Fix |
|---------|-------------|-----|
| `Operation not permitted` writing to the vault | The vault must live in the app documents directory | Let `StratumVault` create `…/StratumVault`; do not point the vault at a SAF URI. |
| `INSTALL_FAILED_INVALID_APK` | Architecture mismatch | Build for the device ABI (`arm64-v8a` for most phones, `x86_64` for emulators). |
| App content behind system bars | A surface ignores the frame | Render inside `MobileFrame`; the shell and the panel variants already offset by the insets. |
| App never reaches a first frame in CI | Software emulation stalled | Re-run; the smoke script already retries once after a platform ANR. |
