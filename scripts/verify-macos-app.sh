#!/usr/bin/env bash
# Verify that the packaged macOS .app bundle launches on a clean machine.
#
# Usage:
#   scripts/verify-macos-app.sh --dir dist/packages --arch arm64
#   scripts/verify-macos-app.sh --zip dist/packages/Stratum-macos.zip --arch x86_64
#   scripts/verify-macos-app.sh --dir dist/packages --arch arm64 --skip-launch
#
# The release workflow calls this on a fresh macos-latest runner after the
# bundle is built, so the artifact is never "verified" by the machine that
# produced it. The checks are:
#
#   layout       Contents/MacOS/stratum is executable and Info.plist is valid,
#                carries the app.stratum identifier and the 11.0 minimum
#   arch         the Mach-O binary carries exactly the requested architecture
#   signature    a Developer ID signature must pass `codesign --verify --deep
#                --strict` and `spctl`; an unsigned or ad-hoc bundle is expected
#                to be rejected by Gatekeeper, which is reported together with
#                the first-launch path (right-click Open / xattr -d
#                com.apple.quarantine)
#   launch       the shell starts with STRATUM_FIXTURE=1 and either stays alive
#                for the probe window or exits cleanly (a crash is fatal)
#
# Exit code 0 means the bundle is structurally sound and launches.
set -euo pipefail

DIR=""
ZIP=""
ARCH=""
SKIP_LAUNCH=0
PROBE_SECONDS=15

usage() {
    sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir) DIR="${2:?--dir needs a value}"; shift 2 ;;
        --dir=*) DIR="${1#*=}"; shift ;;
        --zip) ZIP="${2:?--zip needs a value}"; shift 2 ;;
        --zip=*) ZIP="${1#*=}"; shift ;;
        --arch) ARCH="${2:?--arch needs a value}"; shift 2 ;;
        --arch=*) ARCH="${1#*=}"; shift ;;
        --skip-launch) SKIP_LAUNCH=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "error: unknown argument '$1' (try --help)" >&2; exit 2 ;;
    esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "error: the macOS bundle checks require macOS." >&2
    exit 1
fi
if [[ -z "$ARCH" ]]; then
    echo "error: --arch arm64|x86_64 is required." >&2
    exit 2
fi
case "$ARCH" in
    arm64|x86_64) ;;
    *) echo "error: unsupported --arch '$ARCH' (arm64 or x86_64)" >&2; exit 2 ;;
esac

if [[ -z "$ZIP" ]]; then
    if [[ -z "$DIR" ]]; then
        DIR="$PWD/dist/packages"
    fi
    if [[ ! -d "$DIR" ]]; then
        echo "error: artifact directory '$DIR' does not exist." >&2
        exit 1
    fi
    shopt -s nullglob
    candidates=("$DIR"/*.zip)
    shopt -u nullglob
    if [[ "${#candidates[@]}" -ne 1 ]]; then
        echo "error: expected exactly one .zip in $DIR, found ${#candidates[@]}." >&2
        exit 1
    fi
    ZIP="${candidates[0]}"
fi
if [[ ! -f "$ZIP" ]]; then
    echo "error: '$ZIP' does not exist." >&2
    exit 1
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/stratum-macos.XXXXXX")"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

echo "==> $(basename "$ZIP") ($ARCH)"
ditto -x -k "$ZIP" "$WORK"
shopt -s nullglob
apps=("$WORK"/*.app)
shopt -u nullglob
if [[ "${#apps[@]}" -ne 1 ]]; then
    echo "FAIL: expected exactly one .app in the archive, found ${#apps[@]}" >&2
    exit 1
fi
APP="${apps[0]}"
BIN="$APP/Contents/MacOS/stratum"

fail() {
    echo "FAIL: $*" >&2
    # Workflow annotations are readable without downloading the job log, so the
    # reason travels with the check run even when the log is not.
    echo "::error::$*"
    exit 1
}

[[ -f "$APP/Contents/Info.plist" ]] || fail "Contents/Info.plist is missing"
[[ -x "$BIN" ]] || fail "Contents/MacOS/stratum is missing or not executable"
plutil -lint "$APP/Contents/Info.plist" >/dev/null || fail "Info.plist is not a valid property list"

BUNDLE_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist" 2>/dev/null || true)"
[[ "$BUNDLE_ID" == "app.stratum" ]] || fail "CFBundleIdentifier is '$BUNDLE_ID', expected app.stratum"

MIN_SYSTEM="$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$APP/Contents/Info.plist" 2>/dev/null || true)"
# The metadata pins the minimum at 11.0; accept the 11.x spellings a template
# may normalise it to, but reject anything that silently raises the floor.
[[ "$MIN_SYSTEM" == 11* ]] || fail "LSMinimumSystemVersion is '$MIN_SYSTEM', expected 11.0"

VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist" 2>/dev/null || true)"
[[ -n "$VERSION" ]] || fail "CFBundleShortVersionString is empty"
echo "    bundle: $BUNDLE_ID $VERSION, minimum macOS $MIN_SYSTEM"

# A single-arch bundle is expected: the workflow builds one bundle per target,
# and a stray second architecture would mean the cross build leaked into the
# host target directory.
ARCHES="$(lipo -archs "$BIN" 2>/dev/null || true)"
[[ -n "$ARCHES" ]] || fail "lipo could not read the architectures of $BIN"
if [[ "$ARCHES" != "$ARCH" ]]; then
    fail "the binary carries '$ARCHES', expected exactly '$ARCH'"
fi
echo "    architectures: $ARCHES"

# Gatekeeper: a Developer ID signature has to pass the full assessment. Without
# the Apple secrets the bundle stays unsigned (or ad-hoc signed by the
# toolchain); macOS still runs it when it was not downloaded, and after a
# download the user clears the quarantine flag or uses right-click > Open.
SIGN_INFO="$(codesign -dv --verbose=4 "$APP" 2>&1 || true)"
if grep -q 'Signature=adhoc' <<<"$SIGN_INFO"; then
    SIGNATURE="ad-hoc"
elif grep -q 'Authority=' <<<"$SIGN_INFO"; then
    SIGNATURE="developer-id"
elif grep -q 'code object is not signed at all' <<<"$SIGN_INFO"; then
    SIGNATURE="unsigned"
else
    SIGNATURE="unknown"
fi

case "$SIGNATURE" in
    developer-id)
        codesign --verify --deep --strict --verbose=2 "$APP" || fail "codesign --verify failed"
        spctl -a -vvv -t exec "$APP" || fail "Gatekeeper rejected the signed bundle"
        echo "    signature: Developer ID (codesign + spctl accepted)"
        ;;
    ad-hoc|unsigned|unknown)
        echo "    signature: $SIGNATURE"
        if spctl -a -vvv -t exec "$APP" >/dev/null 2>&1; then
            echo "    Gatekeeper: accepted"
        else
            echo "    Gatekeeper: rejected (expected without a Developer ID);" \
                 "first launch is right-click > Open, or 'xattr -d com.apple.quarantine'"
        fi
        ;;
esac

if [[ "$SKIP_LAUNCH" -eq 0 ]]; then
    LOG="$WORK/stratum-launch.log"
    # The single-instance glue forwards argv to a running copy, so the probe
    # runs before anything else starts the shell.
    STRATUM_FIXTURE=1 STRATUM_LOCALE=en "$BIN" >"$LOG" 2>&1 &
    PID=$!
    waited=0
    while [[ "$waited" -lt "$PROBE_SECONDS" ]]; do
        # A child that already exited stays visible as a zombie until it is
        # reaped, so `kill -0` alone would keep reporting it as alive; the
        # process state is what distinguishes a running shell from a crash.
        state="$(ps -o state= -p "$PID" 2>/dev/null || true)"
        if [[ -z "$state" || "$state" == Z* ]]; then
            rc=0
            wait "$PID" || rc=$?
            if [[ "$rc" -ne 0 ]]; then
                tail -20 "$LOG" >&2 || true
                fail "the shell exited with $rc before the probe window"
            fi
            if grep -qiE 'panic|thread .* panicked' "$LOG"; then
                tail -20 "$LOG" >&2 || true
                fail "panic in the shell output"
            fi
            echo "    launch: exited cleanly (exit 0)"
            echo "==> PASS $(basename "$ZIP")"
            exit 0
        fi
        sleep 1
        waited=$((waited + 1))
    done
    kill "$PID" 2>/dev/null || true
    wait "$PID" 2>/dev/null || true
    if grep -qiE 'panic|thread .* panicked' "$LOG"; then
        tail -20 "$LOG" >&2 || true
        fail "panic in the shell output"
    fi
    window_note=""
    if grep -qi 'failed to open the Stratum shell' "$LOG"; then
        window_note=" (no window on this runner)"
    fi
    echo "    launch: stayed up for ${PROBE_SECONDS}s$window_note"
fi

echo "==> PASS $(basename "$ZIP")"
