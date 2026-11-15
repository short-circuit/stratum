// Root build script for the Stratum Android host.
//
// The Rust library is compiled outside Gradle with cargo-ndk:
//   scripts/build-android.sh --debug
// which places libstratum_mobile.so under app/src/main/jniLibs/<abi>/ before
// ./gradlew assembleDebug runs.

buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        // AGP 9.1.0 is the cached toolchain in the dev environment. Kotlin is
        // not used: every Java source is a plain Activity/JNI helper.
        classpath("com.android.tools.build:gradle:9.1.0")
    }
}

tasks.register("clean", Delete::class) {
    delete(rootProject.layout.buildDirectory)
}
