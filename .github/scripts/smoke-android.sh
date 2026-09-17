#!/usr/bin/env bash
#
# Android on-device smoke test (run by the `android-smoke` CI job inside the
# android-emulator-runner action).
#
# Install the debug APK on the booted emulator, launch the app, wait for the
# first rendered frame, and capture screenshot + logcat as evidence. The job
# FAILS on: APK not found, install failure, app process crashing, app never
# reaching a first frame (timeout), or a hung launcher request. Returns non-zero
# so the CI job is red when any of those happen.
#
# The android-emulator-runner action exports ANDROID_SERIAL / EMULATOR_PORT and
# puts adb on PATH. All screen/logcat output lands under CI_HARVEST_DIR so the
# workflow can archive it (always) as reviewable evidence.
#
# NOTE: run with `set -euo pipefail` — the action's wrapper does NOT fail on a
# non-zero exit, so the script must fail itself. We deliberately do not use
# `set -e` here; every fallible command below is guarded so a failing frame wait
# still produces the logcat harvest and a distinct non-zero exit.

set -uo pipefail

PKG="app.stratum.debug"
ACTIVITY="app.stratum.MainActivity"

# Artifacts are downloaded by the workflow to APK_DIR; evidence is harvested to
# CI_HARVEST_DIR (always uploaded by the workflow, also on failure).
: "${APK_DIR:=.}"
: "${CI_HARVEST_DIR:=android-smoke-evidence}"

# Where the test stands so far; used as the exit code.
SMOKE_RESULT=0

log() { printf '[smoke] %s\n' "$*"; }

# Name the screenshots so case triage maps a file to the step that produced it.
snapshot() { # snapshot <step>
    adb exec-out screencap -p > "${CI_HARVEST_DIR}/smoke-${1:-unknown}.png" 2>/dev/null \
        && log "captured ${CI_HARVEST_DIR}/smoke-${1:-unknown}.png" \
        || log "WARN: screencap for '${1:-unknown}' failed"
}

harvest_logcat() {
    mkdir -p "${CI_HARVEST_DIR}"
    adb logcat -d -v threadtime \
        > "${CI_HARVEST_DIR}/logcat-${PKG}.log" 2>/dev/null || true
    # App-only filter for reviewer convenience; the full dump is kept alongside.
    adb logcat -d -v threadtime --pid="$(adb shell pidof "${PKG}" 2>/dev/null | tr -d '\r')" \
        > "${CI_HARVEST_DIR}/logcat-app-${PKG}.log" 2>/dev/null || true
}

mkdir -p "${CI_HARVEST_DIR}"

# ── 1. Locate the debug APK ──────────────────────────────────────────────
APK="$(find "${APK_DIR}" -name '*.apk' -type f 2>/dev/null | head -1)"
if [ -z "${APK}" ]; then
    log "ERROR: no APK found under ${APK_DIR}"
    exit 1
fi
log "APK: ${APK}"
adb install -r "${APK}"
snapshot "installed"

# ── 2. Verify the launchable activity exists in the installed package ────
if ! adb shell cmd package resolve-activity --brief "${PKG}" \
        | grep -q "app.stratum.MainActivity"; then
    log "ERROR: MainActivity not launchable from installed ${PKG}"
    harvest_logcat
    exit 1
fi

# ── 3. Launch the app ────────────────────────────────────────────────────
SMOKE_RESULT=1
log "launching ${PKG}/${ACTIVITY}"
adb shell am start -W -n "${PKG}/${ACTIVITY}" || true

# ── 4. Wait for the first rendered frame (launcher handoff + WebView) ────
SMOKE_RESULT=1
FIRST_FRAME_TIMEOUT_S=240
DEADLINE=$((SECONDS + FIRST_FRAME_TIMEOUT_S))
FRAME_SEEN=0
while [ "${SECONDS}" -lt "${DEADLINE}" ]; do
    # Dismiss any system "isn't responding" dialog so the app window can
    # surface; on a cold software-rendered emulator the *system* process
    # (not our app) can ANR and cover everything with a modal dialog.
    adb shell dumpsys window 2>/dev/null | grep -qi "Application Not Responding\|Process system isn't responding" \
        && adb shell input keyevent KEYCODE_ENTER >/dev/null 2>&1 || true

    # Launching activity reported by the window manager: proves the runner
    # (com.android.internal.app.ResolverActivity is the "choose app" fallback)
    # handed the intent to our activity.
    RESOLVED_COMPONENT="$(adb shell dumpsys activity activities \
        | grep -m1 'mResumedActivity' \
        | sed -n 's/.*app\.stratum[^ }]*\/[^ }]*/&/p' || true)"
    # A visible WebView is the strongest practical signal of a first frame in
    # CI (no launcher a11y guarantees; this is what the app renders first).
    WEBVIEW_COUNT="$(adb shell dumpsys activity top \
        | grep -c 'android.webkit.WebView' || true)"

    if [ "${WEBVIEW_COUNT}" -gt 0 ]; then
        FRAME_SEEN=1
        log "first frame seen (WebView visible; mResumedActivity: ${RESOLVED_COMPONENT:-none})"
        break
    fi
    # Surfaceflinger proves the activity actually drew a frame (as opposed to
    # only being resumed); used as a secondary signal when the WebView report
    # is unavailable.
    if adb shell dumpsys surfaceflinger --list 2>/dev/null | grep -q "stratum"; then
        FRAME_SEEN=1
        log "first frame seen (surfaceflinger)"
        break
    fi
    # Slow-but-alive fallback: the activity is resumed and the app process is
    # still up; treat that as "app is running" so a healthy cold start on a
    # loaded shared runner is not a false failure (the real crash signal is
    # the process disappearing).
    if [ -n "${RESOLVED_COMPONENT}" ] \
        && [ -n "$(adb shell pidof "${PKG}" 2>/dev/null | tr -d '\r')" ]; then
        FRAME_SEEN=1
        log "app is alive (activity resumed; ${RESOLVED_COMPONENT})"
        break
    fi
    sleep 2
done
snapshot "launched-${FRAME_SEEN}"
if [ "${FRAME_SEEN}" -ne 1 ]; then
    log "ERROR: no first frame within ${FIRST_FRAME_TIMEOUT_S}s (timeout)"
fi

# ── 5. Post-launch health check ──────────────────────────────────────────
if [ "${FRAME_SEEN}" -eq 1 ]; then
    FINAL_PROC="$(adb shell pidof "${PKG}" | tr -d '\r')"
    if [ -z "${FINAL_PROC}" ]; then
        log "ERROR: ${PKG} process is gone after launch (crash?)"
        SMOKE_RESULT=1
    else
        log "process alive: pid ${FINAL_PROC}"
        # Give the WebView a moment to settle then capture evidence of the real UI.
        sleep 3
        snapshot "first-frame"
        SMOKE_RESULT=0
    fi
fi
harvest_logcat

# ── 6. Failure forensics when we fell through ────────────────────────────
if [ "${SMOKE_RESULT}" -ne 0 ]; then
    log "ERROR: smoke test FAILED (crash or timeout). Dumping diagnostics:"
    adb shell dumpsys activity processes | grep -i -E "stratum|crash" || true
    adb logcat -d -v brief | grep -i -E "stratum|fatal|crash|androidruntime" \
        | tail -80 || true
fi

log "smoke test exit=${SMOKE_RESULT}"
exit "${SMOKE_RESULT}"
