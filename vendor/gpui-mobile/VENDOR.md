# Vendored: gpui-mobile (Android/iOS platform layer for GPUI)

Upstream: <https://github.com/itsbalamurali/gpui-mobile>
Vendored revision: `5fd54f4` (Merge pull request #63 from longbridge/upstream/16-deregister-ioswindow-from-the-ffi-window)

Upstream license: `GPL-3.0-or-later OR AGPL-3.0-or-later OR Apache-2.0`.
Stratum consumes this fork under **Apache-2.0** (AGPL-3.0 compatible); the
upstream `LICENSE-APACHE`, `LICENSE-GPL` and `LICENSE-AGPL` files are kept
verbatim.

This directory is excluded from the Stratum workspace
(`exclude = ["vendor/gpui-mobile"]` in the root `Cargo.toml`) and carries its own
empty `[workspace]` table so it builds standalone and `cargo clippy
--workspace` does not lint it.

## Why this fork exists

Stratum's native shell targets the `gpui-pre 0.3.12` snapshot (via `gpui-kit
0.9.1`). Upstream `gpui-mobile` tracks an older snapshot (`0.3.6`) and its
Android backend does not implement the mobile platform surface added in 0.3.12
(insets, lifecycle, system back). This fork pins 0.3.12, ports the API changes,
and adds the callbacks the Android host needs.

## Local patches (applied on top of `5fd54f4`)

1. **Dependency pins + standalone workspace.** `gpui` / `gpui_wgpu` pinned to
   `=0.3.12` (renamed packages `gpui-pre` / `gpui-pre-wgpu`), `wgpu` bumped to
   `30.0.1` in both target sections, dev-dependency `gpui` pinned to `=0.3.12`,
   and an empty `[workspace]` table added to `Cargo.toml`.
2. **`PlatformDisplay for AndroidDisplay`.** Added
   `refresh_interval()` using `gpui::refresh_interval_from_hz`; added an
   optional `refresh_hz` field with `with_refresh_hz()` / `refresh_hz()`
   accessors because the NDK has no refresh-rate getter. Unknown rates return
   `None`. Tests cover the conversion and invalid rates.
3. **`Platform for AndroidPlatform`.** Added `platform_info()` (Android /
   Native), `on_displays_changed()` + `notify_displays_changed()`, and
   `on_app_lifecycle()` + `notify_lifecycle(AppLifecyclePhase)`.
   `update_primary_display()` and the `ConfigChanged` path now fire display
   listeners outside the state lock.
4. **`Platform for SharedPlatform`.** Delegates `platform_info`,
   `on_displays_changed` and `on_app_lifecycle` to the wrapped
   `AndroidPlatform`.
5. **`PlatformWindow for AndroidPlatformWindow`.** Implemented
   `on_display_changed()`, `insets()`, `on_insets_changed()` and
   `set_back_handler()`; the callbacks are stored on `AndroidWindow` and fired
   on the main thread.
6. **`AndroidWindow::gpu_specs()`.** Flattened the now-nested
   `Option<Option<GpuSpecs>>` with `and_then`.
7. **`RequestFrameOptions` initializer.** Added the 0.3.12 fields
   `signal_at: None` and `signal_source: FrameRequestSource::NativeCallback`.
8. **Window insets / back / display state.** Added `insets`,
   `insets_callback`, `back_handler` and `display_changed_callback` to
   `WindowState`, plus `set_insets()`, `request_back()` and
   `notify_display_changed()`. `set_insets()` keeps the legacy physical
   `safe_area_insets()` accessor coherent with the Java report and fires the
   registered callback outside the state lock.
9. **JNI entry points.** Added
   `Java_dev_gpui_mobile_GpuiInputActivity_nativeInsets`,
   `..._nativeBack` and `..._nativeLifecycle` (0=Active, 1=Inactive,
   2=Background, 3=Foreground). The Java host reports physical pixels; the
   entry point divides by the window scale factor before building
   `gpui::WindowInsets` (safe area = `systemBars() | displayCutout()`, IME =
   `ime()`, zero corner radii). All three are safe no-ops before the platform
   or primary window exists and are covered by unit tests.
10. **Java host activity.** `GpuiInputActivity` installs an
    `OnApplyWindowInsetsListener` (API 30+ `getInsets(systemBars() |
    displayCutout())` / `getInsets(ime())`; API 26-29 falls back to
    `getSystemWindowInset*()` and reports no IME), overrides `onBackPressed`
    to call `nativeBack()`, forwards `onStart`/`onResume`/`onPause`/`onStop`
    to `nativeLifecycle(...)`, and declares the three native methods.
11. **Android link-search fallback.** `build.rs` adds the NDK API 26 sysroot
    library directory when `cargo-ndk` is invoked without an explicit platform
    (it defaults to API 21, which has no `libnativewindow.so`). This keeps the
    plain `cargo ndk build` invocation linking successfully. `-P 26` /
    `CARGO_NDK_PLATFORM=26` remains preferred because it also stamps the ELF
    with the API 26 tag the example app's `minSdk` expects.

No unrelated upstream code was reformatted.

## Building

The example app's `minSdk` is 26 (Vulkan 1.0 is mandatory from API 26), and
`libnativewindow.so` only exists in the NDK sysroot for API >= 26.
`cargo-ndk` defaults to API 21, which fails to link `-lnativewindow`; the
vendored `build.rs` adds the API 26 link-search directory as a fallback so the
plain invocation links, but pinning the platform is still preferred because it
also stamps the correct ELF API tag:

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/27.1.12297006"
export CARGO_NDK_PLATFORM=26
cargo ndk -t arm64-v8a -t x86_64 -o /tmp/jnilibs \
  build --manifest-path vendor/gpui-mobile/Cargo.toml --lib
```

The crate is consumed by `crates/stratum-mobile`; keep its public API stable
when patching.
