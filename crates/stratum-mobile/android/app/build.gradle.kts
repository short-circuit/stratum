// App module for the Stratum Android host.
//
// The module packages the pre-compiled Rust native library into an APK that
// hosts the gpui shell through Android's NativeActivity. The library is built
// by scripts/build-android.sh and copied into app/src/main/jniLibs/<abi>/.

plugins {
    id("com.android.application")
}

android {
    namespace = "com.stratum.app"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.stratum.app"
        // Vulkan 1.0 is mandatory from API 26 and libnativewindow.so only
        // exists in the API 26 NDK sysroot.
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"

        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }

        // Tell NativeActivity which .so to load (libstratum_mobile.so).
        manifestPlaceholders["nativeLibraryName"] = "stratum_mobile"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
    }

    // The native library is compiled externally via cargo-ndk, so Gradle must
    // not look for a CMakeLists.txt or Android.mk.
    externalNativeBuild {
        // Intentionally left empty.
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // Tell Gradle where the pre-built .so files live.
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packaging {
        // Keep the Rust symbols: cargo already strips in release mode and a
        // second strip would break backtraces from logcat.
        jniLibs {
            keepDebugSymbols += listOf(
                "*/arm64-v8a/libstratum_mobile.so",
                "*/x86_64/libstratum_mobile.so"
            )
        }
    }

    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

dependencies {
    // NotificationCompat for GpuiNotifications.
    implementation("androidx.core:core:1.12.0")
    // SplashScreen compat used by GpuiActivity to hold the splash until the
    // native library signals readiness.
    implementation("androidx.core:core-splashscreen:1.0.1")
    // MediaSessionCompat for GpuiMediaSession (system media controls).
    implementation("androidx.media:media:1.7.1")
}
