//! Android host for the Stratum gpui shell.
//!
//! This crate is the packaging target for the Android APK. `cargo ndk` builds
//! it as `libstratum_mobile.so`, the Gradle project in `android/` bundles it
//! behind the vendored `NativeActivity` subclass
//! (`dev.gpui.mobile.GpuiActivity`), and the exported [`android_main`] entry
//! point boots the same gpui shell the desktop binary renders.
//!
//! The host owns only platform glue:
//!
//! - [`android::android_main`] is the `android-activity` entry point: it
//!   initialises logcat, the vendored gpui-mobile platform, and runs the gpui
//!   application with the shell asset source.
//! - `android::vault` resolves the on-device vault location and creates the
//!   first-run layout (an empty vault is valid, no pages are seeded).
//! - `android::services` implements [`stratum_ui::mobile::MobileServices`] on
//!   top of the gpui-mobile packages (microphone, file picker, notifications,
//!   clipboard, URL launcher).
//!
//! Android builds compile the `android` module and pull in the vendored
//! `gpui-mobile` platform layer. Host builds (Linux CI, docs) compile an empty
//! stub so `cargo build`/`cargo clippy --workspace` stay green without any
//! Android dependency.
//!
//! Build with `scripts/build-android.sh` (arm64-v8a + x86_64, minSdk 26); the
//! APK lands in `crates/stratum-mobile/android/app/build/outputs/apk/debug/`.

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::android_main;

/// Whether this build hosts the Android shell.
pub const IS_ANDROID_HOST: bool = cfg!(target_os = "android");

/// The application id the Gradle project packages.
pub const APPLICATION_ID: &str = "com.stratum.app";

/// The native library name the manifest's `android.app.lib_name` points at.
pub const NATIVE_LIBRARY_NAME: &str = "stratum_mobile";

/// The directory name created under the app documents directory for the vault.
pub const VAULT_DIR: &str = "StratumVault";

/// The minimum Android API level the host supports (Vulkan 1.0 is mandatory
/// from API 26 and `libnativewindow.so` only exists in the API 26 sysroot).
pub const MIN_SDK: u32 = 26;

/// A short description of the host build, used by the boot log.
pub fn host_description() -> &'static str {
    if IS_ANDROID_HOST {
        "Android host (NativeActivity + gpui-mobile)"
    } else {
        "host stub (Android deps are not compiled on this target)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_metadata_matches_the_gradle_project() {
        assert_eq!(APPLICATION_ID, "com.stratum.app");
        assert_eq!(NATIVE_LIBRARY_NAME, "stratum_mobile");
        assert_eq!(VAULT_DIR, "StratumVault");
        assert_eq!(MIN_SDK, 26);
    }

    #[test]
    fn host_builds_are_described() {
        let description = host_description();
        if IS_ANDROID_HOST {
            assert!(description.starts_with("Android host"));
        } else {
            assert!(description.starts_with("host stub"));
        }
    }
}
