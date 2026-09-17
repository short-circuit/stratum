#!/usr/bin/env bash
#
# Android on-device smoke test (run by the `android-smoke` CI job inside the
# android-emulator-runner action).
#
# Install the debug APK on the booted emulator, launch the app, WAIT FOR THE
# FIRST RENDERED FRAME, and capture screenshot + logcat as evidence. The job
# FAILS on: APK not found, install failure, app process crashing, app never
# reaching a first frame (timeout), or the launcher request itself erroring.
# Returns non-zero so the CI job is red when any of those happen.
#
# Robustness on software-emulated devices (no KVM, e.g. GitHub-hosted Linux
# runners): the Android input-dispatch watchdog is a compile-time 5s constant
# in InputManagerService (DEFAULT_INPUT_DISPATCHING_TIMEOUT_NANOS) that CANNOT
# be raised via device_config/settings. On a slow unaccelerated emulator the
# system routinely fires `ANR Reason: Input dispatching timed out ... Waited
# 5014ms for TouchModeEvent` on a perfectly healthy app and then force-finishes
# the activity. The platform's OWN first-frame signal — `ActivityTaskManager:
# Displayed app.<pkg>/<act> for user 0: +NsXXXms` — fires when the activity
# actually drew a frame, and we poll logcat for it (the `am start -W` text is
# NOT a reliable pass criterion: on a slow software-emulated device it returns
# before the first frame, so we never parse it for the pass decision).
#
# First-frame detection therefore uses the platform's authoritative logcat
# `Displayed` line; when the system ANR watchdog force-finishes a healthy
# activity before the frame, a bounded relaunch recovers it. The job fails
# only on a genuine crash (process death without a Displayed), an install
# failure, or an app that truly never draws within the deadline.
#
# NOTE: run with `set -euo pipefail` — the action's wrapper does NOT fail on a
# non-zero exit, so the script must fail itself. We deliberately do not use
# `set -e` here; every fallible command below is guarded so a failing frame
# wait still produces the logcat harvest and a distinct non-zero exit.

set -uo pipefail

PKG="app.stratum.debug"
ACTIVITY="app.stratum.MainActivity"
ACTIVITY_PATH="${PKG}/${ACTIVITY}"

# Artifacts are downloaded by the workflow to APK_DIR; evidence is harvested to
# CI_HARVEST_DIR (always uploaded by the workflow, also on failure).
: "${APK_DIR:=.}"
: "${CI_HARVEST_DIR:=android-smoke-evidence}"

# Where the test stands so far; used as the exit code.
SMOKE_RESULT=0

# Overall ceiling for the whole first-frame wait (a generous cushion over the
# per-attempt wait; guards against an adb/emulator hang).
FIRST_FRAME_TIMEOUT_S=240
# Per-attempt ceiling waiting for the platform's `Displayed` logcat signal
# (evidence: a cold software-emulated first frame took ~23s). Bounded so a
# genuinely dead/hung launch is not waited on forever, but long enough to
# absorb a slow cold start without a false failure. Env-overridable so tests
# can shorten it.
: "${PER_ATTEMPT_TIMEOUT_S:=90}"
# After the system force-finishes our activity (the input-dispatch ANR hiccup),
# we relaunch the app up to this many times before declaring a failure.
MAX_LAUNCH_ATTEMPTS=3

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

app_process_dead() {
    [ -z "$(adb shell pidof "${PKG}" 2>/dev/null | tr -d '\r ')" ]
}

# Launch the app (bounded so a hung launcher cannot wedge the job) and then
# poll logcat for the platform's authoritative first-frame signal:
#   ActivityTaskManager: Displayed app.stratum.debug/app.stratum.MainActivity
# The `am start -W` text is NOT reliable on a slow software-emulated device
# (it returns before the first frame), so we never parse it for the pass
# decision. Returns:
#   0  first frame seen (Displayed in logcat) and process alive
#   1  launched but no Displayed before the per-attempt wait (crashed /
#      force-finished by the system ANR watchdog before the frame)
#   2  even the launcher request errored / no activity started
launch_once() {
    local out rc attempt_start
    # Clear any stale Displayed line from a previous attempt so it is not
    # mistaken for the current launch (logcat is per-device, not per-process).
    adb logcat -c 2>/dev/null || true
    out="$(timeout 90 adb shell am start -n "${ACTIVITY_PATH}" 2>&1)"
    rc=$?
    if [ "${rc}" -ne 0 ]; then
        log "WARN: am start exited ${rc}: ${out}"
        return 2
    fi

    # Wait (bounded) for the platform's Displayed first-frame signal.
    attempt_start=$SECONDS
    while [ $((SECONDS - attempt_start)) -lt "${PER_ATTEMPT_TIMEOUT_S}" ]; do
        if adb logcat -d -v brief 2>/dev/null \
            | grep -q "Displayed ${ACTIVITY_PATH}"; then
            return 0
        fi
        # A hard crash before the frame is a terminal signal; stop early.
        if app_process_dead && ! adb logcat -d -v brief 2>/dev/null \
            | grep -q "Displayed ${ACTIVITY_PATH}"; then
            return 1
        fi
        sleep 2
    done
    return 1
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

# ── 3. Launch the app and wait for the first rendered frame ──────────────
# The system's 5s input-dispatch watchdog can force-finish a perfectly healthy
# activity on a software-emulated device before it ever becomes visible; we
# retry the launch (bounded) in that case. If the process is GONE that is the
# one signal that is never recoverable -- fail immediately with forensics.
SMOKE_RESULT=1
ATTEMPT=0
FRAME_SEEN=0
LOOP_DEADLINE=$((SECONDS + FIRST_FRAME_TIMEOUT_S))

while [ "${SECONDS}" -lt "${LOOP_DEADLINE}" ] \
    && [ "${ATTEMPT}" -lt "${MAX_LAUNCH_ATTEMPTS}" ] \
    && [ "${FRAME_SEEN}" -ne 1 ]; do
    SMOKE_RESULT=1
    ATTEMPT=$((ATTEMPT + 1))
    log "launch attempt ${ATTEMPT}/${MAX_LAUNCH_ATTEMPTS}: ${ACTIVITY_PATH}"

    RC_LAUNCH="$(launch_once; echo ":${?}")"
    LAUNCH_RC="${RC_LAUNCH##*:}"
    log "launch outcome rc=${LAUNCH_RC}"
    case "${LAUNCH_RC}" in
        0)
            # Authoritative: the platform logged `Displayed <activity>` in
            # logcat, meaning the activity actually drew its first frame.
            log "first frame seen (platform Displayed signal in logcat)"
            FRAME_SEEN=1
            ;;
        1)
            # Launched but never became visible (crashed or force-finished).
            log "WARN: launch handoff did not become visible"
            ;;
        2)
            # Launcher handoff error -> retry.
            log "WARN: launcher handoff error (am start -W)"
            ;;
    esac

    # ── 4. Post-launch verification & evidence ───────────────────────────
    # We only reach here once a first frame was seen. Give the WebView a
    # moment to settle, then capture real-UI evidence.
    if [ "${FRAME_SEEN}" -eq 1 ]; then
        sleep 3
        snapshot "first-frame"
        # Record the platform's own first-frame timestamp into the log for the
        # evidence bundle (helps reviewers rule out a slow-but-healthy boot).
        adb logcat -d -v brief 2>/dev/null | grep -m1 "Displayed ${ACTIVITY_PATH}" || true
        # Authoritative process-alive check: the app must still be running.
        FINAL_PROC="$(adb shell pidof "${PKG}" 2>/dev/null | tr -d '\r')"
        if [ -z "${FINAL_PROC}" ]; then
            log "ERROR: ${PKG} process is gone after first frame (post-launch crash?)"
            SMOKE_RESULT=1
            FRAME_SEEN=0
            break
        fi
        log "process alive: pid ${FINAL_PROC}"
        SMOKE_RESULT=0
        break
    fi

    # ── No first frame yet ───────────────────────────────────────────────
    # If the app process is gone, that is a hard crash (can't be recovered by
    # relaunching).
    if app_process_dead; then
        log "ERROR: ${PKG} process is GONE after launch attempt ${ATTEMPT} (crash?)"
        break
    fi
    # Otherwise the system likely ANR'd the healthy app mid-handoff; give the
    # window manager a moment to settle then retry the launch fresh.
    log "WARN: no first frame on attempt ${ATTEMPT}; app still alive, retrying"
    sleep 5
done

harvest_logcat

# ── 5. Failure forensics when we fell through ────────────────────────────
if [ "${SMOKE_RESULT}" -ne 0 ]; then
    log "ERROR: smoke test FAILED (crash or timeout after ${ATTEMPT} launch attempt(s)). Dumping diagnostics:"
    adb shell dumpsys activity processes | grep -i -E "stratum|crash" || true
    # Show whether any AndroidRuntime FATAL / SIGSEGV / SIGABRT was the cause.
    adb logcat -d -v brief 2>/dev/null | grep -i -E "stratum|FATAL EXCEPTION|AndroidRuntime|SIGSEGV|SIGABRT|crash" \
        | tail -80 || true
fi

log "smoke test exit=${SMOKE_RESULT}"
exit "${SMOKE_RESULT}"
