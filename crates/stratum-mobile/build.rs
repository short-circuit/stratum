//! Link configuration for the Android host.
//!
//! `cpal`'s Android backend links the Oboe C++ audio library statically, and
//! that object file references the C++ runtime (`__cxa_pure_virtual`,
//! `__cxa_begin_catch`, `operator new`, ...). `cargo-ndk` does not add the C++
//! runtime to the final link, so without these directives the device fails to
//! load the library:
//!
//! ```text
//! dlopen failed: cannot locate symbol "__cxa_pure_virtual" referenced by
//! .../libstratum_mobile.so
//! ```
//!
//! Linking the NDK's static libc++ (from the NDK sysroot for the target ABI)
//! makes the `.so` self-contained, so the APK does not need to ship or preload
//! `libc++_shared.so`. The sysroot is located from the same environment
//! variables `cargo-ndk` reads, so a plain `cargo ndk` invocation works.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_ROOT");
    println!("cargo:rerun-if-env-changed=ANDROID_HOME");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }

    let abi = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "aarch64-linux-android",
        Ok("x86_64") => "x86_64-linux-android",
        Ok("arm") => "arm-linux-androideabi",
        Ok("x86") => "i686-linux-android",
        other => panic!("unsupported Android target arch: {other:?}"),
    };

    let Some(ndk) = ndk_home() else {
        // Without an NDK the link would fail anyway; let the linker report it.
        println!("cargo:warning=ANDROID_NDK_HOME is unset; skipping the libc++ search path");
        return;
    };

    let sysroot_lib = ndk
        .join("toolchains")
        .join("llvm")
        .join("prebuilt")
        .join(host_tag())
        .join("sysroot")
        .join("usr")
        .join("lib")
        .join(abi);
    if !sysroot_lib.is_dir() {
        panic!(
            "Android NDK sysroot library directory is missing: {}",
            sysroot_lib.display()
        );
    }

    // Pass the archives by absolute path instead of `-lc++_static`: adding the
    // NDK sysroot as a plain `-L` search path also makes the linker resolve
    // `-lc` / `-lm` / `-ldl` to their *static* archives, which pulls a private
    // copy of bionic into the `.so`. That copy never runs `__libc_init`, so
    // `__libc_auxv` stays null and the first `std::thread::spawn` segfaults in
    // `getauxval`.
    println!(
        "cargo:rustc-link-arg={}",
        sysroot_lib.join("libc++_static.a").display()
    );
    println!(
        "cargo:rustc-link-arg={}",
        sysroot_lib.join("libc++abi.a").display()
    );
}

/// Resolve the NDK root from the environment the way `cargo-ndk` does.
fn ndk_home() -> Option<PathBuf> {
    for key in ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return Some(PathBuf::from(value));
            }
        }
    }
    let sdk = std::env::var("ANDROID_HOME")
        .ok()
        .filter(|v| !v.is_empty())?;
    let ndk_dir = PathBuf::from(sdk).join("ndk");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&ndk_dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    versions.pop()
}

/// The NDK prebuilt toolchain directory for the host running the build.
fn host_tag() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else if cfg!(target_os = "windows") {
        "windows-x86_64"
    } else {
        "linux-x86_64"
    }
}
