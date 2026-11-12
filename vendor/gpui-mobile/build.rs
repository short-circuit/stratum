//! Build script for the vendored `gpui-mobile` fork.
//!
//! Android builds link `libnativewindow.so` through the `ndk` crate's
//! `nativewindow` feature. That library only exists in the NDK sysroot for
//! API 26+ (Vulkan 1.0 is mandatory from API 26), while `cargo-ndk` defaults
//! to API 21. When the selected platform is older, add the API 26 sysroot
//! directory as a link search path so `-lnativewindow` resolves.
//!
//! Passing `CARGO_NDK_PLATFORM=26` (or `cargo ndk -P 26`) is still preferred;
//! this is a fallback for the default invocation.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_ROOT");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK");
    println!("cargo:rerun-if-env-changed=CARGO_NDK_PLATFORM");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }

    // Treat an unset platform as cargo-ndk's default (21).
    let platform = env::var("CARGO_NDK_PLATFORM")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(21);
    if platform >= 26 {
        return;
    }

    let Some(ndk) = env::var_os("ANDROID_NDK_HOME")
        .or_else(|| env::var_os("ANDROID_NDK_ROOT"))
        .or_else(|| env::var_os("ANDROID_NDK"))
    else {
        // No NDK hint: leave the link paths untouched.
        return;
    };

    let host_tag = match env::var("HOST").unwrap_or_default() {
        host if host.contains("linux") => "linux-x86_64",
        host if host.contains("darwin") => "darwin-x86_64",
        host if host.contains("windows") => "windows-x86_64",
        _ => return,
    };

    let target_abi = match env::var("TARGET").unwrap_or_default().as_str() {
        "aarch64-linux-android" => "aarch64-linux-android",
        "armv7-linux-androideabi" => "arm-linux-androideabi",
        "i686-linux-android" => "i686-linux-android",
        "x86_64-linux-android" => "x86_64-linux-android",
        _ => return,
    };

    let api_26_dir = PathBuf::from(ndk)
        .join("toolchains/llvm/prebuilt")
        .join(host_tag)
        .join("sysroot/usr/lib")
        .join(target_abi)
        .join("26");

    if api_26_dir.is_dir() {
        println!("cargo:rustc-link-search=native={}", api_26_dir.display());
    }
}
