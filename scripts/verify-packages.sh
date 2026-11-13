#!/usr/bin/env bash
# Verify that the packaged installers install and launch on clean systems.
#
# Usage:
#   scripts/verify-packages.sh                 # inspect dist/packages
#   scripts/verify-packages.sh --dir <dir>
#   scripts/verify-packages.sh --skip-launch   # install checks only (faster)
#
# The checks run in throwaway Docker containers, so a package is never
# "verified" by the machine that built it:
#
#   .deb        ubuntu:22.04  apt install from the file, then a headless launch
#   .rpm        fedora:42     dnf install from the file, then a headless launch
#   .AppImage   ubuntu:22.04  extract, then a headless launch through AppRun
#   .apk/.aab   host          apksigner verify + aapt badging (needs the SDK)
#   .msi/.exe   (skipped)     installed by the release workflow's Windows job
#   .zip        (skipped)     unzipped by the release workflow's macOS job
#
# A headless launch starts the shell under Xvfb with the software Vulkan driver
# and requires it to stay alive for the whole probe window: a crash, a missing
# shared library or a panicking renderer ends the run early, so a clean
# timeout is the pass condition.
#
# Requirements: docker (with network access for apt/dnf), and — for the Android
# artifacts — the SDK build-tools. Linux packages must have been built on a
# glibc-2.35 base (Ubuntu 22.04, which is what CI uses); a package built on a
# newer glibc cannot install on the verification image and is reported as such.
set -euo pipefail

DIR="$PWD/dist/packages"
SKIP_LAUNCH=0
PROBE_SECONDS=15

usage() {
    sed -n '2,27p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir) DIR="${2:?--dir needs a value}"; shift 2 ;;
        --dir=*) DIR="${1#*=}"; shift ;;
        --skip-launch) SKIP_LAUNCH=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "error: unknown argument '$1' (try --help)" >&2; exit 2 ;;
    esac
done

if [[ ! -d "$DIR" ]]; then
    echo "error: artifact directory '$DIR' does not exist." >&2
    exit 1
fi
DIR="$(cd "$DIR" && pwd)"

if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker is required for the install checks." >&2
    exit 1
fi

FAILED=0
RESULTS=()

# The launch probe is written to a file and bind-mounted into the container.
# Interpolating it into `docker run ... bash -c "..."` would break on the quotes
# it contains. `timeout` returns 124 when it had to kill the process, which means
# the shell was still running — that is the success condition.
PROBE_FILE=""
cleanup() {
    if [[ -n "$PROBE_FILE" ]]; then
        rm -f "$PROBE_FILE"
    fi
}
trap cleanup EXIT

write_probe() {
    PROBE_FILE="$(mktemp "${TMPDIR:-/tmp}/stratum-probe.XXXXXX.sh")"
    cat >"$PROBE_FILE" <<'PROBE'
set -uo pipefail
export STRATUM_FIXTURE=1
export STRATUM_LOCALE=en
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
# An installed package is probed by name; the AppImage passes the bundle itself
# through STRATUM_PROBE_BIN so the launch goes through AppRun (which sets up
# LD_LIBRARY_PATH for the bundled libraries). The AppImage runtime is an ELF
# wrapper around a squashfs image, so the ldd precheck is skipped for it — the
# launch itself is the real check.
BIN="${STRATUM_PROBE_BIN:-$(command -v stratum || true)}"
if [ -z "$BIN" ]; then
    echo "FAIL: stratum is not on PATH"
    exit 1
fi
if [ "${STRATUM_PROBE_SKIP_LDD:-0}" != "1" ]; then
    MISSING="$(ldd "$BIN" | grep 'not found' || true)"
    if [ -n "$MISSING" ]; then
        echo "FAIL: unresolved shared library:"
        echo "$MISSING"
        exit 1
    fi
fi
xvfb-run -a timeout "$PROBE_SECONDS" "$BIN" > /tmp/stratum.log 2>&1
rc=$?
if [ "$rc" -ne 124 ]; then
    echo "FAIL: the shell exited before the probe window (rc=$rc)"
    tail -20 /tmp/stratum.log
    exit 1
fi
if grep -qiE 'panic|thread .* panicked' /tmp/stratum.log; then
    echo "FAIL: panic in the shell output"
    tail -20 /tmp/stratum.log
    exit 1
fi
echo "OK: the shell stayed up for ${PROBE_SECONDS}s"
PROBE
}

record() {
    local name="$1" status="$2" detail="$3"
    RESULTS+=("$(printf '%-30s %-6s %s' "$name" "$status" "$detail")")
    # SKIP means another platform verifies the artifact, so only a FAIL is a
    # failure here.
    if [[ "$status" == "FAIL" ]]; then
        FAILED=1
    fi
}

# Shared docker arguments: the artifact directory read-only, the probe mounted
# when a launch check is wanted.
prepare_docker() {
    DOCKER_ARGS=(--rm -v "$DIR":/art:ro -e PROBE_SECONDS="$PROBE_SECONDS")
    PROBE_CMD="true"
    if [[ -n "$PROBE_FILE" ]]; then
        DOCKER_ARGS+=(-v "$PROBE_FILE":/probe.sh:ro)
        PROBE_CMD="bash /probe.sh"
    fi
}

verify_deb() {
    local file="$1" name log
    name="$(basename "$file")"
    log="$(mktemp "${TMPDIR:-/tmp}/verify-deb.XXXXXX.log")"
    echo "==> $name (ubuntu:22.04)"
    prepare_docker
    if docker run "${DOCKER_ARGS[@]}" ubuntu:22.04 bash -c "
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends /art/$name > /tmp/install.log 2>&1 || { cat /tmp/install.log; exit 1; }
apt-get install -y -qq --no-install-recommends xvfb xauth mesa-vulkan-drivers > /dev/null 2>&1
dpkg -s stratum > /dev/null
test -x /usr/bin/stratum
test -f /usr/share/applications/stratum.desktop
test -f /usr/share/doc/stratum/copyright
$PROBE_CMD
" >"$log" 2>&1; then
        record "$name" PASS "installed + launched"
        tail -1 "$log" | sed 's/^/    /'
    else
        record "$name" FAIL "log: $log"
        tail -15 "$log" | sed 's/^/    /'
    fi
}

verify_rpm() {
    local file="$1" name log
    name="$(basename "$file")"
    log="$(mktemp "${TMPDIR:-/tmp}/verify-rpm.XXXXXX.log")"
    echo "==> $name (fedora:42)"
    prepare_docker
    if docker run "${DOCKER_ARGS[@]}" fedora:42 bash -c "
set -euo pipefail
dnf install -y -q /art/$name > /tmp/install.log 2>&1 || { cat /tmp/install.log; exit 1; }
dnf install -y -q xorg-x11-server-Xvfb xauth mesa-vulkan-drivers > /dev/null 2>&1
rpm -q stratum > /dev/null
test -x /usr/bin/stratum
test -f /usr/share/applications/stratum.desktop
$PROBE_CMD
" >"$log" 2>&1; then
        record "$name" PASS "installed + launched"
        tail -1 "$log" | sed 's/^/    /'
    else
        record "$name" FAIL "log: $log"
        tail -15 "$log" | sed 's/^/    /'
    fi
}

verify_appimage() {
    local file="$1" name log
    name="$(basename "$file")"
    log="$(mktemp "${TMPDIR:-/tmp}/verify-appimage.XXXXXX.log")"
    echo "==> $name (ubuntu:22.04)"
    prepare_docker
    # libasound2 is deliberately NOT installed here: linuxdeploy treats it as a
    # system library and skips it, so cargo-packager copies it into the AppDir
    # explicitly (`[package.metadata.packager.appimage] libs`). Its absence from
    # the container proves the bundle carries it. libxcb1 IS installed because
    # linuxdeploy blacklists it and every X11 desktop ships it.
    if docker run "${DOCKER_ARGS[@]}" -e APPIMAGE_EXTRACT_AND_RUN=1 \
            -e STRATUM_PROBE_BIN=/tmp/stratum.AppImage -e STRATUM_PROBE_SKIP_LDD=1 \
            ubuntu:22.04 bash -c "
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends xvfb xauth mesa-vulkan-drivers libxcb1 > /dev/null 2>&1
cp /art/$name /tmp/stratum.AppImage
chmod +x /tmp/stratum.AppImage
cd /tmp
APPIMAGE_EXTRACT_AND_RUN=1 ./stratum.AppImage --appimage-extract > /dev/null
test -x /tmp/squashfs-root/AppRun
test -x /tmp/squashfs-root/usr/bin/stratum
test -f /tmp/squashfs-root/usr/share/applications/stratum.desktop
# cargo-packager copies libasound into usr/lib/<triple>, not usr/lib.
find /tmp/squashfs-root/usr/lib -name 'libasound.so.2' | grep -q .
export APPIMAGE_EXTRACT_AND_RUN=1
$PROBE_CMD
" >"$log" 2>&1; then
        record "$name" PASS "extracted + launched"
        tail -1 "$log" | sed 's/^/    /'
    else
        record "$name" FAIL "log: $log"
        tail -15 "$log" | sed 's/^/    /'
    fi
}

verify_android() {
    local file="$1" name log build_tools apksigner aapt2
    name="$(basename "$file")"
    log="$(mktemp "${TMPDIR:-/tmp}/verify-android.XXXXXX.log")"
    echo "==> $name (host SDK)"
    build_tools=""
    if [[ -n "${ANDROID_HOME:-}" ]]; then
        build_tools="$(ls -d "$ANDROID_HOME"/build-tools/* 2>/dev/null | sort -V | tail -1 || true)"
    fi
    apksigner=""
    aapt2=""
    if [[ -n "$build_tools" ]]; then
        apksigner="$build_tools/apksigner"
        aapt2="$build_tools/aapt2"
    fi
    if [[ ! -x "$apksigner" ]]; then
        record "$name" SKIP "apksigner not found (set ANDROID_HOME)"
        return
    fi
    if [[ "${file##*.}" == "aab" ]]; then
        # An AAB is a zip of modules; the base module carries the manifest and
        # the per-ABI native libraries. The listing is written to a file first:
        # `unzip -l | grep -q` would trip `pipefail` when grep exits early and
        # unzip gets SIGPIPE.
        if unzip -l "$file" >"$log.list" 2>&1 \
            && grep -q "base/manifest/AndroidManifest.xml" "$log.list" \
            && grep -q "base/lib/arm64-v8a/libstratum_mobile.so" "$log.list" \
            && grep -q "base/lib/x86_64/libstratum_mobile.so" "$log.list" \
            && grep -qE "META-INF/[^/]*\.RSA" "$log.list"; then
            record "$name" PASS "App Bundle layout + both ABIs + signed"
        else
            record "$name" FAIL "unexpected App Bundle layout (listing: $log.list)"
            grep -E "manifest|lib/|META-INF/[^/]*\.(RSA|SF)" "$log.list" | head -10 | sed 's/^/    /'
        fi
        return
    fi
    if "$apksigner" verify --print-certs "$file" >"$log" 2>&1; then
        if [[ -x "$aapt2" ]] && "$aapt2" dump badging "$file" >"$log.badging" 2>&1; then
            record "$name" PASS "$(grep -m1 'package:' "$log.badging" | cut -c1-60)"
        else
            record "$name" PASS "signed"
        fi
    else
        record "$name" FAIL "apksigner verify failed (log: $log)"
        tail -10 "$log" | sed 's/^/    /'
    fi
}

if [[ "$SKIP_LAUNCH" -eq 0 ]]; then
    write_probe
fi

echo "==> artifacts in $DIR"
shopt -s nullglob
for file in "$DIR"/*.deb; do verify_deb "$file"; done
for file in "$DIR"/*.rpm; do verify_rpm "$file"; done
for file in "$DIR"/*.AppImage; do verify_appimage "$file"; done
for file in "$DIR"/*.apk "$DIR"/*.aab; do verify_android "$file"; done
for file in "$DIR"/*.msi "$DIR"/*.exe "$DIR"/*.zip "$DIR"/*.dmg; do
    record "$(basename "$file")" SKIP "scripts/verify-windows-installers.ps1 / scripts/verify-macos-app.sh run on the platform release job"
done

if [[ "${#RESULTS[@]}" -eq 0 ]]; then
    echo "error: no artifacts found in $DIR" >&2
    exit 1
fi

echo
echo "==> results"
for line in "${RESULTS[@]}"; do
    echo "    $line"
done

if [[ "$FAILED" -ne 0 ]]; then
    echo
    echo "==> FAILED"
    exit 1
fi

echo
echo "==> all installers verified"
