#!/usr/bin/env bash
#
# Android status-bar overlap regression gate (E5.E1).
#
# Runs the CDP safe-area probe against the app already installed and launched
# on the booted emulator (the android-smoke job drives this AFTER
# smoke-android.sh confirms the first rendered frame). It:
#   1. locates the live app pid and forwards the WebView devtools socket
#   2. discovers the page debugger target via /json
#   3. runs `.github/scripts/safe-area-probe.mjs` in device mode
#   4. harvests the probe result + a screenshot as evidence
#
# Returns 0 when the probe PASSES (chrome offset below the status bar) and
# non-zero when it FAILS (the pre-fix overlap: top bar pinned under the
# status bar), so the CI job is red on regression. Exit 2 means the WebView
# debugger was unreachable — the caller treats that as a non-fatal SKIP (a
# missing debugger is not evidence of overlap, and the local smoke harness
# exercises the control flow without a real device).
#
# Env:
#   PKG              app package (defaults to app.stratum.debug)
#   ACTIVITY         launchable activity (defaults to app.stratum.MainActivity)
#   CI_HARVEST_DIR   evidence output dir (defaults to android-smoke-evidence)
#   EXPECT_VERDICT   'pass' (default) or 'fail' — lets the harness assert the
#                    probe fails on the old build (regression proof).
#
set -uo pipefail

PKG="${PKG:-app.stratum.debug}"
ACTIVITY="${ACTIVITY:-app.stratum.MainActivity}"
CI_HARVEST_DIR="${CI_HARVEST_DIR:-android-smoke-evidence}"
EXPECT_VERDICT="${EXPECT_VERDICT:-pass}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROBE="${SCRIPT_DIR}/safe-area-probe.mjs"

log() { printf '[safe-area-regression] %s\n' "$*"; }

mkdir -p "${CI_HARVEST_DIR}"

# The REQUIRED probe mode for the on-device (Android) target.
MODE=device
log "running status-bar overlap regression probe (${MODE} mode)"

# Exit codes (documented in the header):
#   0 = probe ran, PASS (or, under EXPECT_VERDICT=fail, the old build failed as
#       expected => regression proof, also 0)
#   1 = probe ran and found chrome under the status bar (REGRESSION) or the
#       expected-broken build did NOT fail
#   2 = could not reach the WebView debugger (pid missing / adb forward failed /
#       no /json target) — treated as a non-fatal SKIP by the caller, because a
#       missing debugger is not evidence of overlap
#      (probe-internal 3 = ws conn error, 4 = page eval error are surfaced as-is)

# ── 1. Locate the live app process ─────────────────────────────────────────
# `pidof` can transiently return empty right after launch even though the
# process is alive (the same race hardened in smoke-android.sh, t_…b2336322
# regression). Retry briefly instead of failing on a false negative.
PID=""
for _try in 1 2 3 4 5 6 7 8 9 10; do
    PID="$(adb shell pidof "${PKG}" 2>/dev/null | tr -d '\r ')"
    [ -n "${PID}" ] && break
    sleep 1
done
if [ -z "${PID}" ]; then
    log "SKIP: ${PKG} not running after retries; cannot probe (exit 2)"
    exit 2
fi
log "app pid: ${PID}"

# ── 2. Forward the WebView remote-debugging socket ─────────────────────────
FWD_PORT="$(adb forward tcp:0 "localabstract:webview_devtools_remote_${PID}" 2>/dev/null | tr -d '\r')"
if [ -z "${FWD_PORT}" ]; then
    log "SKIP: adb forward of devtools socket failed (exit 2)"
    exit 2
fi
log "forwarded devtools on host port ${FWD_PORT}"
sleep 2

# ── 3. Discover the page debugger target ───────────────────────────────────
TARGETS="$(curl -fsS "http://127.0.0.1:${FWD_PORT}/json" 2>/dev/null || true)"
printf '%s\n' "${TARGETS}" > "${CI_HARVEST_DIR}/devtools-targets.json"
WSURL="$(printf '%s' "${TARGETS}" | node -e \
  "let s='';process.stdin.on('data',d=>s+=d).on('end',()=>{try{const a=JSON.parse(s);const t=a.find(x=>x.type==='page');if(t)console.log(t.webSocketDebuggerUrl)}catch{}})" 2>/dev/null || true)"
if [ -z "${WSURL}" ]; then
    log "SKIP: no page debugger target found (devtools-forwarded /json) (exit 2)"
    exit 2
fi
log "devtools ws: ${WSURL}"

# ── 4. Run the safe-area probe (device mode) ───────────────────────────────
if ! command -v node >/dev/null 2>&1; then
    log "ERROR: node is required for the safe-area probe"
    exit 1
fi
set +e
node "${PROBE}" "${WSURL}" "${MODE}" > "${CI_HARVEST_DIR}/safe-area-probe.out" 2>&1
PROBE_EXIT=$?
set -e
cat "${CI_HARVEST_DIR}/safe-area-probe.out"

# Screenshot evidence (best-effort)
adb exec-out screencap -p > "${CI_HARVEST_DIR}/safe-area-verify.png" 2>/dev/null || true

if [ "${EXPECT_VERDICT}" = "fail" ]; then
    if [ "${PROBE_EXIT}" -eq 1 ]; then
        log "probe failed on the old build as expected (regression proof)"
        exit 0
    fi
    log "ERROR: probe did NOT fail on the expected-broken build (exit ${PROBE_EXIT})"
    exit 1
fi

if [ "${PROBE_EXIT}" -ne 0 ]; then
    log "ERROR: status-bar overlap regression detected (probe exit ${PROBE_EXIT})"
    exit "${PROBE_EXIT}"
fi
log "status-bar overlap regression probe PASSED"
exit 0
