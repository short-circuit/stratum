// Gradle settings for the Stratum Android host.
//
// A single-module project that packages the Rust native library (compiled
// separately via cargo-ndk, see scripts/build-android.sh) into an APK hosted by
// the vendored NativeActivity subclass.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "StratumAndroid"
include(":app")
