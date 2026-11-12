#!/usr/bin/env bash
# Build the Stratum Android host: cargo-ndk compiles libstratum_mobile.so for
# arm64-v8a and x86_64, then Gradle packages the debug APK around it.
#
# Usage:
#   scripts/build-android.sh            # release .so + debug APK
#   scripts/build-android.sh --debug    # debug .so + debug APK
#   scripts/build-android.sh --install  # build, then adb install on the device
#
# Environment:
#   ANDROID_HOME      Android SDK root (default: $HOME/Android/Sdk)
#   ANDROID_NDK_HOME  NDK root (default: $ANDROID_HOME/ndk/27.1.12297006)
#
# The script fails with a clear message when the SDK, NDK or cargo-ndk is
# missing, and verifies that both ABIs ended up inside the APK.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ANDROID_PROJECT="$ROOT/crates/stratum-mobile/android"
JNI_DIR="$ANDROID_PROJECT/app/src/main/jniLibs"
APK="$ANDROID_PROJECT/app/build/outputs/apk/debug/app-debug.apk"

ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
export ANDROID_HOME
export ANDROID_NDK_HOME
# Stamp the ELF with the API 26 tag the manifest's minSdk expects. cargo-ndk
# defaults to API 21, which has no libnativewindow.so in the sysroot.
export CARGO_NDK_PLATFORM="${CARGO_NDK_PLATFORM:-26}"

DEBUG=0
INSTALL=0

usage() {
    sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

for arg in "$@"; do
    case "$arg" in
        --debug) DEBUG=1 ;;
        --install) INSTALL=1 ;;
        -h|--help) usage; exit 0 ;;
        *)
            echo "error: unknown argument '$arg' (try --help)" >&2
            exit 2
            ;;
    esac
done

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

BUILD_FLAGS=()
if [ "$DEBUG" -eq 0 ]; then
    BUILD_FLAGS+=(--release)
fi

mkdir -p "$JNI_DIR"

echo "==> cargo ndk build (-p stratum-mobile, arm64-v8a + x86_64, ${BUILD_FLAGS[*]:-debug})"
cd "$ROOT"
cargo ndk -t arm64-v8a -t x86_64 -o "$JNI_DIR" build "${BUILD_FLAGS[@]}" -p stratum-mobile

# `gpui-mobile` declares a cdylib crate type, so cargo-ndk also emits
# libgpui_mobile.so. The host links it as an rlib, so that copy is dead weight
# (a debug build of it alone is ~350 MB) — drop it from the APK payload.
rm -f "$JNI_DIR"/arm64-v8a/libgpui_mobile.so "$JNI_DIR"/x86_64/libgpui_mobile.so

# Debug libraries carry full DWARF and are ~1 GB per ABI. Strip the symbols
# that are not needed for relocation so a debug APK stays installable.
if [ "$DEBUG" -eq 1 ]; then
    STRIP="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"
    if [ -x "$STRIP" ]; then
        "$STRIP" --strip-unneeded "$JNI_DIR"/arm64-v8a/libstratum_mobile.so \
            "$JNI_DIR"/x86_64/libstratum_mobile.so
    fi
fi

echo "==> gradle assembleDebug"
cd "$ANDROID_PROJECT"
./gradlew --no-daemon assembleDebug

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
