#!/usr/bin/env bash
# Build the Stratum Android host: cargo-ndk compiles libstratum_mobile.so for
# arm64-v8a and x86_64, then Gradle packages the APK around it.
#
# Usage:
#   scripts/build-android.sh              # release .so + debug APK (developer build)
#   scripts/build-android.sh --debug      # debug .so + debug APK
#   scripts/build-android.sh --release    # release .so + signed APK + AAB (release build)
#   scripts/build-android.sh --install    # build, then adb install on the device
#
# Environment:
#   ANDROID_HOME      Android SDK root (default: $HOME/Android/Sdk)
#   ANDROID_NDK_HOME  NDK root (default: $ANDROID_HOME/ndk/27.1.12297006)
#
# Release signing (only used with --release). Gradle reads these variables
# through app/build.gradle.kts; when ANDROID_KEYSTORE_PATH is unset the release
# build falls back to the debug key so it still runs on a developer machine,
# and the script says so.
#   ANDROID_KEYSTORE_PATH        PKCS#12/JKS keystore with the upload key
#   ANDROID_KEYSTORE_PASSWORD    keystore password
#   ANDROID_KEY_ALIAS            key alias inside the keystore
#   ANDROID_KEY_PASSWORD         key password
#   STRATUM_ANDROID_VERSION_NAME versionName override (defaults to the Gradle value)
#   STRATUM_ANDROID_VERSION_CODE versionCode override (defaults to the Gradle value)
#
# The script fails with a clear message when the SDK, NDK or cargo-ndk is
# missing, and verifies that both ABIs ended up inside the APK.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ANDROID_PROJECT="$ROOT/crates/stratum-mobile/android"
JNI_DIR="$ANDROID_PROJECT/app/src/main/jniLibs"

ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
export ANDROID_HOME
export ANDROID_NDK_HOME
# Stamp the ELF with the API 26 tag the manifest's minSdk expects. cargo-ndk
# defaults to API 21, which has no libnativewindow.so in the sysroot.
export CARGO_NDK_PLATFORM="${CARGO_NDK_PLATFORM:-26}"

DEBUG=0
RELEASE=0
INSTALL=0

usage() {
    sed -n '2,27p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

for arg in "$@"; do
    case "$arg" in
        --debug) DEBUG=1 ;;
        --release) RELEASE=1 ;;
        --install) INSTALL=1 ;;
        -h|--help) usage; exit 0 ;;
        *)
            echo "error: unknown argument '$arg' (try --help)" >&2
            exit 2
            ;;
    esac
done

if [ "$DEBUG" -eq 1 ] && [ "$RELEASE" -eq 1 ]; then
    echo "error: --debug and --release are mutually exclusive." >&2
    exit 2
fi

if [ ! -d "$ANDROID_HOME" ]; then
    echo "error: Android SDK not found at '$ANDROID_HOME'." >&2
    echo "       Set ANDROID_HOME or install the SDK (platform android-34, build-tools 35/36)." >&2
    exit 1
fi

if [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo "error: Android NDK not found at '$ANDROID_NDK_HOME'." >&2
    echo "       Set ANDROID_NDK_HOME or install the NDK (27.1.12297006)." >&2
    exit 1
fi

if ! cargo ndk --version >/dev/null 2>&1; then
    echo "error: cargo-ndk is not installed." >&2
    echo "       Install it with: cargo install cargo-ndk --version 4.1.2" >&2
    exit 1
fi

if [ ! -x "$ANDROID_PROJECT/gradlew" ]; then
    echo "error: Gradle wrapper missing at '$ANDROID_PROJECT/gradlew'." >&2
    exit 1
fi

# A release build signs with the upload key when CI provides it; otherwise the
# artifact is only debug-signed and must not be uploaded to a store.
if [ "$RELEASE" -eq 1 ] && [ -z "${ANDROID_KEYSTORE_PATH:-}" ]; then
    echo "warning: ANDROID_KEYSTORE_PATH is unset — the release APK/AAB will be" >&2
    echo "         signed with the debug key (fine for local checks, not for a store)." >&2
fi

BUILD_FLAGS=()
APK_VARIANT="debug"
GRADLE_TASKS=(assembleDebug)
if [ "$DEBUG" -eq 0 ]; then
    BUILD_FLAGS+=(--release)
fi
if [ "$RELEASE" -eq 1 ]; then
    APK_VARIANT="release"
    GRADLE_TASKS=(assembleRelease bundleRelease)
fi

APK="$ANDROID_PROJECT/app/build/outputs/apk/$APK_VARIANT/app-$APK_VARIANT.apk"
AAB="$ANDROID_PROJECT/app/build/outputs/bundle/release/app-release.aab"

mkdir -p "$JNI_DIR"

echo "==> cargo ndk build (-p stratum-mobile, arm64-v8a + x86_64, ${BUILD_FLAGS[*]:-debug})"
cd "$ROOT"
cargo ndk -t arm64-v8a -t x86_64 -o "$JNI_DIR" build "${BUILD_FLAGS[@]}" -p stratum-mobile

# `gpui-mobile` declares a cdylib crate type, so cargo-ndk also emits
# libgpui_mobile.so. The host links it as an rlib, so that copy is dead weight
# (a debug build of it alone is ~350 MB) — drop it from the APK payload.
rm -f "$JNI_DIR"/arm64-v8a/libgpui_mobile.so "$JNI_DIR"/x86_64/libgpui_mobile.so

# The shipped library has to be stripped: a release .so still carries its
# symbol table and a debug .so carries full DWARF (~1 GB per ABI), and AGP keeps
# the symbols as configured in `jniLibs.keepDebugSymbols`. `--strip-unneeded`
# keeps what relocation needs, so backtraces from logcat still resolve.
STRIP="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"
if [ -x "$STRIP" ]; then
    "$STRIP" --strip-unneeded "$JNI_DIR"/arm64-v8a/libstratum_mobile.so \
        "$JNI_DIR"/x86_64/libstratum_mobile.so
else
    echo "warning: llvm-strip not found at '$STRIP' — the APK payload stays unstripped." >&2
fi

echo "==> gradle ${GRADLE_TASKS[*]}"
cd "$ANDROID_PROJECT"
./gradlew --no-daemon "${GRADLE_TASKS[@]}"

if [ ! -f "$APK" ]; then
    echo "error: expected APK not found at '$APK'." >&2
    exit 1
fi

echo "==> APK: $APK"
echo "==> native libraries:"
# Capture the listing once: piping `unzip` into `grep -q` would let grep exit
# early, and with `pipefail` unzip's SIGPIPE would fail the build.
NATIVE_LISTING="$(unzip -l "$APK")"
printf '%s\n' "$NATIVE_LISTING" | grep -E 'lib/(arm64-v8a|x86_64)/libstratum_mobile\.so' || true

for abi in arm64-v8a x86_64; do
    if ! grep -q "lib/$abi/libstratum_mobile.so" <<<"$NATIVE_LISTING"; then
        echo "error: the APK is missing lib/$abi/libstratum_mobile.so." >&2
        exit 1
    fi
done

# apksigner ships with the SDK build-tools; use it to prove the artifact is
# actually signed and to print the certificate a store would see.
APKSIGNER="$(ls -d "$ANDROID_HOME"/build-tools/*/apksigner 2>/dev/null | sort -V | tail -1 || true)"
if [ -n "$APKSIGNER" ] && [ -x "$APKSIGNER" ]; then
    echo "==> apksigner verify"
    "$APKSIGNER" verify --print-certs "$APK"
else
    echo "warning: apksigner not found under $ANDROID_HOME/build-tools — skipping signature check." >&2
fi

if [ "$RELEASE" -eq 1 ]; then
    if [ ! -f "$AAB" ]; then
        echo "error: expected AAB not found at '$AAB'." >&2
        exit 1
    fi
    echo "==> AAB: $AAB"
    ls -la "$AAB"
fi

if [ "$INSTALL" -eq 1 ]; then
    ADB="$ANDROID_HOME/platform-tools/adb"
    if [ ! -x "$ADB" ]; then
        ADB="$(command -v adb || true)"
    fi
    if [ -z "$ADB" ] || [ ! -x "$ADB" ]; then
        echo "error: adb not found (looked in '$ANDROID_HOME/platform-tools' and PATH)." >&2
        exit 1
    fi
    echo "==> adb install -r $APK"
    "$ADB" install -r "$APK"
fi
