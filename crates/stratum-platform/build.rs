//! Build script: set the `desktop` cfg for the OS-glue modules.
//!
//! The crate keeps every OS call behind `#[cfg(desktop)]` so the Android host
//! (E10) compiles the same shell without the desktop crates. Cargo has no
//! built-in "desktop" predicate, so the build script derives it from the target
//! OS the same way the Tauri crates do.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-check-cfg=cfg(desktop)");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let desktop = matches!(
        target_os.as_str(),
        "linux" | "windows" | "macos" | "freebsd" | "dragonfly" | "netbsd" | "openbsd"
    );
    if desktop {
        println!("cargo:rustc-cfg=desktop");
    }
}
