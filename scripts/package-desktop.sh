#!/usr/bin/env bash
# Build the Stratum desktop installers from the gpui shell binary.
#
# Usage:
#   scripts/package-desktop.sh                        # platform default formats
#   scripts/package-desktop.sh --formats deb,appimage,rpm
#   scripts/package-desktop.sh --formats app --out-dir dist/packages
#   scripts/package-desktop.sh --skip-build           # reuse an existing binary
#   scripts/package-desktop.sh --formats app --target aarch64-apple-darwin
#
# Platform defaults: deb + AppImage + rpm on Linux, nsis + wix (MSI) on Windows,
# app on macOS. The formats are produced with cargo-packager, except RPM, which
# cargo-packager does not implement — that one comes from cargo-generate-rpm.
# Both tools read their configuration from
# crates/stratum-ui/Cargo.toml (`[package.metadata.packager]` and
# `[package.metadata.generate-rpm]`), so the packages always carry the workspace
# version and one edit covers every format.
#
# Every artifact is hashed afterwards: sha256sums.txt in the output directory is
# what the release job publishes next to the packages.
#
# Environment:
#   OUT_DIR                   output directory (default: dist/packages)
#   STRATUM_RPM_SIGNING_KEY   PGP private key; signs the RPM with rpm-sign
#   STRATUM_RPM_SIGNING_KEY_ID  key id used for the signature (default: first id)
#   CARGO_TARGET_DIR          honoured by cargo, so a shared target dir works
#   APPLE_SIGNING_IDENTITY    macOS only: codesign identity for the .app bundle
#   APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID
#                             macOS only: notarization credentials; with the
#                             identity set, the bundle is signed and notarized
#
# Requirements: cargo, cargo-packager, cargo-generate-rpm (Linux) and the
# platform toolchain (WiX for MSI, Xcode command line tools for the macOS app).
# CI installs cargo-packager/cargo-generate-rpm in the release workflow.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/dist/packages}"
FORMATS=""
TARGET=""
SKIP_BUILD=0
CRATE="stratum-ui"
BIN="stratum"

usage() {
    sed -n '2,34p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --formats) FORMATS="${2:?--formats needs a value}"; shift 2 ;;
        --formats=*) FORMATS="${1#*=}"; shift ;;
        --out-dir) OUT_DIR="${2:?--out-dir needs a value}"; shift 2 ;;
        --out-dir=*) OUT_DIR="${1#*=}"; shift ;;
        --target) TARGET="${2:?--target needs a value}"; shift 2 ;;
        --target=*) TARGET="${1#*=}"; shift ;;
        --skip-build) SKIP_BUILD=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "error: unknown argument '$1' (try --help)" >&2; exit 2 ;;
    esac
done

case "$(uname -s)" in
    Linux)  DEFAULT_FORMATS="deb,appimage,rpm" ;;
    Darwin) DEFAULT_FORMATS="app" ;;
    MINGW*|MSYS*|CYGWIN*) DEFAULT_FORMATS="nsis,wix" ;;
    *) echo "error: unsupported platform $(uname -s)" >&2; exit 1 ;;
esac
FORMATS="${FORMATS:-$DEFAULT_FORMATS}"

# Split the requested formats: cargo-packager handles everything but rpm.
PACKAGER_FORMATS=()
WANT_RPM=0
IFS=',' read -r -a requested <<<"$FORMATS"
for format in "${requested[@]}"; do
    case "$format" in
        rpm) WANT_RPM=1 ;;
        deb|appimage|nsis|wix|app|dmg|pacman) PACKAGER_FORMATS+=("$format") ;;
        *) echo "error: unknown format '$format'" >&2; exit 2 ;;
    esac
done

if ! cargo packager --version >/dev/null 2>&1; then
    echo "error: cargo-packager is not installed." >&2
    echo "       Install it with: cargo install cargo-packager --locked" >&2
    exit 1
fi
if [[ "$WANT_RPM" -eq 1 ]] && ! cargo generate-rpm --version >/dev/null 2>&1; then
    echo "error: cargo-generate-rpm is not installed." >&2
    echo "       Install it with: cargo install cargo-generate-rpm --locked" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"

# Cross builds (the macOS x86_64 bundle from an arm64 runner) need the target
# threaded through cargo and the packager; an empty TARGET keeps the host
# triple.
TARGET_ARGS=()
if [[ -n "$TARGET" ]]; then
    TARGET_ARGS=(--target "$TARGET")
fi

if [[ "$SKIP_BUILD" -eq 0 ]]; then
    echo "==> cargo build --release -p $CRATE --bin $BIN ${TARGET_ARGS[*]:-}"
    cargo build --release -p "$CRATE" --bin "$BIN" "${TARGET_ARGS[@]}"
fi

# Debian policy wants stripped binaries, and the debug information is a third of
# the payload: strip the release binary in place before it is packaged. Cargo
# does not relink afterwards (the artifact is newer than its inputs), so a later
# `cargo build` keeps working. The flags differ per platform (`--strip-unneeded`
# is GNU binutils only; cctools strip on macOS uses `-x`) and Windows runners
# have no guaranteed `strip`, so an unavailable tool is not an error.
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BIN_PATH="$TARGET_DIR/release/$BIN"
if [[ -n "$TARGET" ]]; then
    BIN_PATH="$TARGET_DIR/$TARGET/release/$BIN"
fi
if [[ -f "$BIN_PATH" ]]; then
    case "$(uname -s)" in
        Linux)
            if command -v strip >/dev/null 2>&1; then
                echo "==> strip --strip-unneeded $BIN_PATH"
                strip --strip-unneeded "$BIN_PATH"
            else
                echo "==> strip not found — leaving $BIN_PATH unstripped" >&2
            fi
            ;;
        Darwin)
            if command -v strip >/dev/null 2>&1; then
                echo "==> strip -x $BIN_PATH"
                strip -x "$BIN_PATH"
            fi
            ;;
        *)
            echo "==> leaving $BIN_PATH unstripped on $(uname -s)"
            ;;
    esac
fi

if [[ "${#PACKAGER_FORMATS[@]}" -gt 0 ]]; then
    JOINED="$(IFS=','; echo "${PACKAGER_FORMATS[*]}")"
    # linuxdeploy runs itself and its AppImage output plugin as AppImages. Make
    # both extract-and-run so the build works without FUSE (containers, CI
    # runners and root shells have no fuse device).
    export APPIMAGE_EXTRACT_AND_RUN=1
    echo "==> cargo packager -p $CRATE --release -f $JOINED"
    # `-o` overrides the configured outDir so a container or CI job can write to
    # a mounted volume while the binary stays in the cargo target directory.
    cargo packager -p "$CRATE" --release -f "$JOINED" -o "$OUT_DIR" "${TARGET_ARGS[@]}"
fi

if [[ "$WANT_RPM" -eq 1 ]]; then
    echo "==> cargo generate-rpm -p crates/$CRATE"
    # cargo-generate-rpm's `-p` takes the crate *directory*, not the package
    # name, so it is given the path relative to the repository root.
    RPM_ARGS=(-p "crates/$CRATE" -o "$OUT_DIR")
    if [[ -n "$TARGET" ]]; then
        RPM_ARGS+=(--target "$TARGET")
    fi
    if [[ -n "${STRATUM_RPM_SIGNING_KEY:-}" ]]; then
        RPM_ARGS+=(--signing_key "$STRATUM_RPM_SIGNING_KEY")
        if [[ -n "${STRATUM_RPM_SIGNING_KEY_ID:-}" ]]; then
            RPM_ARGS+=(--signing_key_id "$STRATUM_RPM_SIGNING_KEY_ID")
        fi
    fi
    cargo generate-rpm "${RPM_ARGS[@]}"
fi

# cargo-packager emits a .app directory on macOS. Sign it when an identity is
# available (the release workflow exports APPLE_SIGNING_IDENTITY from a secret),
# notarize it when the Apple credentials are present, then zip it so the
# download is a single file and the checksum covers the whole bundle.
if [[ "$(uname -s)" == "Darwin" ]]; then
    for app in "$OUT_DIR"/*.app; do
        [[ -d "$app" ]] || continue
        if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
            echo "==> codesign $(basename "$app")"
            codesign --force --deep --options runtime --timestamp \
                --sign "$APPLE_SIGNING_IDENTITY" "$app"
        fi
        if [[ -n "${APPLE_SIGNING_IDENTITY:-}" && -n "${APPLE_ID:-}" \
              && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
            # Notarization works on a zip of the signed bundle; the ticket is
            # stapled to the .app so it also validates offline.
            echo "==> notarize $(basename "$app")"
            NOTARIZE_ZIP="$OUT_DIR/$(basename "${app%.app}")-notarize.zip"
            ditto -c -k --keepParent "$app" "$NOTARIZE_ZIP"
            xcrun notarytool submit "$NOTARIZE_ZIP" --wait \
                --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
                --team-id "$APPLE_TEAM_ID"
            xcrun stapler staple "$app"
            rm -f "$NOTARIZE_ZIP"
        fi
        echo "==> ditto -c -k --keepParent $(basename "$app")"
        ditto -c -k --keepParent "$app" "$OUT_DIR/$(basename "${app%.app}")-macos.zip"
    done
fi

echo "==> checksums"
CHECKSUM_TOOL="sha256sum"
if ! command -v "$CHECKSUM_TOOL" >/dev/null 2>&1; then
    CHECKSUM_TOOL="shasum -a 256"
fi
# Only hash the artifacts themselves: the packager leaves its intermediates in
# $OUT_DIR/.cargo-packager, which must not leak into the published list.
ARTIFACTS=()
while IFS= read -r file; do
    ARTIFACTS+=("$file")
done < <(find "$OUT_DIR" -maxdepth 1 -type f \
    \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \
       -o -name '*.msi' -o -name '*.exe' -o -name '*.zip' -o -name '*.dmg' \) \
    ! -name 'sha256sums.txt' | sort)

if [[ "${#ARTIFACTS[@]}" -eq 0 ]]; then
    echo "error: no artifacts were produced in $OUT_DIR" >&2
    exit 1
fi

# Relative names keep the checksum file portable: verify with
# `cd <dir> && sha256sum -c sha256sums.txt`.
(
    cd "$OUT_DIR"
    for file in "${ARTIFACTS[@]}"; do
        $CHECKSUM_TOOL "$(basename "$file")"
    done
) > "$OUT_DIR/sha256sums.txt"

echo "==> artifacts:"
ls -la "$OUT_DIR" | sed 's/^/    /'
echo "==> checksums written to $OUT_DIR/sha256sums.txt"
