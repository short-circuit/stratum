fn main() {
    #[cfg(target_os = "android")]
    {
        // cpal links -laaudio; find the NDK's libaaudio.so and add its
        // directory to the linker search path.
        if let Ok(ndk) = std::env::var("ANDROID_NDK_HOME") {
            let prebuilt = std::path::PathBuf::from(&ndk)
                .join("toolchains/llvm/prebuilt/linux-x86_64");
            // NDK r21+: libaaudio.so lives under sysroot/usr/lib/<target>/
            for target in &["aarch64-linux-android", "arm-linux-androideabi"] {
                let lib_dir = prebuilt.join("sysroot/usr/lib").join(target);
                if lib_dir.join("libaaudio.so").exists() || lib_dir.join("libaaudio.a").exists() {
                    println!("cargo:rustc-link-search=native={}", lib_dir.display());
                    break;
                }
            }
            // Fallback: NDK's toolchain lib dir also contains shared libs
            let tc_lib = prebuilt.join("lib");
            println!("cargo:rustc-link-search=native={}", tc_lib.display());
        }
    }
}
