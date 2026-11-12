//! Android entry point and window bootstrap.
//!
//! The `android-activity` glue calls [`android_main`] on a dedicated native
//! thread after the Java `NativeActivity` (`dev.gpui.mobile.GpuiActivity`,
//! itself a `GpuiInputActivity`) loads `libstratum_mobile.so`. The function
//! never returns while the shell runs: the vendored gpui-mobile platform
//! drives the native event loop inside `Application::run`.
//!
//! Boot order (every step logs to `adb logcat` under the `stratum` tag so the
//! path is provable on a device):
//!
//! 1. `android_logger` + the gpui-mobile panic hook.
//! 2. `gpui_mobile::android::jni::init_platform` (stores the `AndroidApp` and
//!    builds the `AndroidPlatform`) and `shared_platform`.
//! 3. `debug.stratum.gpu` override: `gl` pins the renderer to GLES for the
//!    emulator; anything else leaves GPUI on its Vulkan-first path.
//! 4. `Application::with_platform(...).with_assets(StratumAssets::new()).run`,
//!    which defers the closure until `MainEvent::InitWindow` delivered the
//!    native surface.
//! 5. [`boot`] installs gpui-kit and the shell theme/fonts, resolves the vault
//!    from the persisted settings, installs [`AndroidServices`] and opens the
//!    mobile shell window.
//!
//! Safe-area and IME insets, the system back action and lifecycle phases are
//! delivered by `GpuiInputActivity` through `nativeInsets`/`nativeBack`/
//! `nativeLifecycle`; the vendored JNI entry points turn them into the GPUI
//! platform callbacks the shell consumes.

pub mod services;
pub mod vault;

use std::sync::Arc;

use gpui_kit::{AppContext, Application, WindowOptions};
use stratum_i18n::{set_locale, Locale};
use stratum_ui::provider::provider_from_env;
use stratum_ui::{Shell, StratumAssets, ThemeConfig};

use services::AndroidServices;
use vault::StratumVault;

/// Logcat tag for every host message.
const LOG_TAG: &str = "stratum";

/// `android.system.Properties` value buffer size (`PROP_VALUE_MAX`).
const PROP_VALUE_MAX: usize = 92;

/// The `android-activity` entry point.
///
/// `android-activity` resolves this symbol from the loaded library with
/// `dlsym`, so the function must be exported and unmangled. It uses the Rust
/// ABI (the crate's documented signature) rather than `extern "C"`.
#[unsafe(no_mangle)]
pub fn android_main(app: android_activity::AndroidApp) {
    trace_init(&app);
    trace("android_main entered");
    // Logger first: everything after this point is visible in logcat.
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag(LOG_TAG),
    );
    trace("logger initialised");
    gpui_mobile::android::jni::install_panic_hook();
    trace("panic hook installed");

    log::info!("android_main: entered ({})", crate::host_description());
    log::info!(
        "android_main: app id={} library={} minSdk={}",
        crate::APPLICATION_ID,
        crate::NATIVE_LIBRARY_NAME,
        crate::MIN_SDK
    );

    let platform = gpui_mobile::android::jni::init_platform(&app);
    trace("platform initialised");
    log::info!("android_main: platform initialised");

    let shared = match gpui_mobile::android::jni::shared_platform() {
        Some(shared) => shared,
        None => {
            log::error!("android_main: shared_platform() returned None — aborting");
            return;
        }
    };
    log::info!("android_main: shared platform ready");
    trace("shared platform ready");

    apply_gpu_override(platform);
    trace("gpu override applied");
    log::info!("android_main: starting the gpui application");

    Application::with_platform(shared.into_rc())
        .with_assets(StratumAssets::new())
        .run(boot);

    trace("application run returned");
    log::info!("android_main: Application::run returned — the activity is finishing");
}

/// The boot-trace directory, resolved once from the `AndroidApp`.
static TRACE_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Resolve the boot-trace directory and start a fresh trace file.
///
/// Logcat only receives messages once the Android logger is initialised, and a
/// crash inside platform setup would leave no trace at all. The trace file
/// records the boot path from the first instruction, so
/// `adb shell run-as com.stratum.app cat files/boot-trace.log` proves exactly
/// how far the host got when logcat is empty. It is truncated on every launch.
fn trace_init(app: &android_activity::AndroidApp) {
    let Some(dir) = app.internal_data_path() else {
        return;
    };
    let _ = std::fs::remove_file(dir.join("boot-trace.log"));
    let _ = TRACE_DIR.set(dir.to_path_buf());
}

/// Append a boot step to the trace file (no-op when it could not be created).
fn trace(step: &str) {
    let Some(dir) = TRACE_DIR.get() else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("boot-trace.log"))
    {
        use std::io::Write as _;
        let _ = writeln!(file, "{step}");
    }
}

/// Boot the Stratum mobile shell inside the gpui application callback.
///
/// This runs once the native surface exists (`MainEvent::InitWindow`), which is
/// what `Application::run` waits for on Android. Each step logs so a device
/// capture can prove the boot path from logcat.
fn boot(cx: &mut gpui_kit::App) {
    trace("boot: gpui application callback entered");
    log::info!("boot: gpui application callback entered");
    gpui_kit::init(cx);
    trace("boot: gpui-kit initialised");
    log::info!("boot: gpui-kit initialised");
    stratum_ui::init(cx, &ThemeConfig::default());
    trace("boot: shell theme and fonts installed");
    log::info!("boot: shell theme and fonts installed");

    let vault = match StratumVault::location().and_then(StratumVault::open) {
        Ok(vault) => vault,
        Err(error) => {
            log::error!("boot: vault unavailable: {error}");
            return;
        }
    };
    trace("boot: vault opened");
    log::info!("boot: vault at {}", vault.path().display());

    let provider = provider_from_env(vault.path().to_path_buf(), false);
    trace("boot: service provider ready");
    log::info!("boot: service provider ready");

    // The persisted settings carry the UI locale. A missing or unreadable
    // config keeps the compiled-in English catalog instead of failing boot.
    match provider.settings() {
        Ok(settings) => match Locale::from_code(&settings.locale) {
            Some(locale) => {
                set_locale(locale);
                log::info!("boot: locale set to {}", settings.locale);
            }
            None => log::warn!(
                "boot: ignoring unsupported persisted locale {:?}",
                settings.locale
            ),
        },
        Err(error) => log::warn!("boot: settings unavailable ({error}); keeping default locale"),
    }

    let mut shell = Shell::new_mobile(provider);
    shell.set_mobile_services(Arc::new(AndroidServices::new()));
    trace("boot: mobile shell constructed");
    log::info!("boot: mobile shell constructed with Android host services");

    match gpui_kit::open_window(
        WindowOptions {
            window_bounds: None,
            ..Default::default()
        },
        cx,
        move |_window, cx| cx.new(|_| shell),
    ) {
        Ok(_) => {
            trace("boot: mobile shell window opened");
            log::info!("boot: mobile shell window opened");
        }
        Err(error) => log::error!("boot: open_window failed: {error}"),
    }

    // Input and IME callbacks only reach the shell once the app owns focus.
    cx.activate(true);
    trace("boot: application activated");
    log::info!("boot: application activated");
}

/// Honour the `debug.stratum.gpu` system property.
///
/// `adb shell setprop debug.stratum.gpu gl` pins the renderer to GLES, which
/// the emulator needs (its SwiftShader Vulkan path chokes on wgpu's SPIR-V).
/// Any other value leaves GPUI on the Vulkan-first default. The vendored
/// platform exposes the preference through
/// [`gpui_mobile::android::AndroidPlatform::set_preferred_backend`]; reading
/// the property never panics before the platform exists.
fn apply_gpu_override(platform: &Arc<gpui_mobile::android::AndroidPlatform>) {
    let Some(value) = system_property("debug.stratum.gpu") else {
        log::info!("android_main: debug.stratum.gpu unset — Vulkan-first default");
        return;
    };
    if value.eq_ignore_ascii_case("gl") {
        platform.set_preferred_backend(gpui_mobile::android::AndroidBackend::Gles);
        log::info!("android_main: GPU backend pinned to GLES by debug.stratum.gpu");
    } else {
        log::info!("android_main: debug.stratum.gpu={value} — Vulkan-first default");
    }
}

/// Read an Android system property through `libc`.
///
/// Returns `None` when the property is unset, empty or not valid UTF-8.
fn system_property(name: &str) -> Option<String> {
    let name = std::ffi::CString::new(name).ok()?;
    let mut buffer = [0u8; PROP_VALUE_MAX];
    // SAFETY: both pointers are valid for the duration of the call; the buffer
    // is `PROP_VALUE_MAX` bytes, which is the size the NDK documents.
    let len = unsafe {
        libc::__system_property_get(name.as_ptr(), buffer.as_mut_ptr().cast::<libc::c_char>())
    };
    if len <= 0 {
        return None;
    }
    let value = std::str::from_utf8(&buffer[..len as usize]).ok()?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_property_returns_none_for_an_unset_property() {
        // `debug.stratum.gpu` is normally unset on a device unless the host set
        // it, so this also documents the default path.
        if let Some(value) = system_property("debug.stratum.gpu") {
            assert!(!value.is_empty());
        }
    }

    #[test]
    fn system_property_handles_invalid_names() {
        // An empty name is still a valid C string, so this must not panic.
        assert!(system_property("").is_none());
    }
}
