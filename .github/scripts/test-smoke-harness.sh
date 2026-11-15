#!/usr/bin/env bash
# Local harness for smoke-android.sh control flow (no emulator needed).
# Provides a mock `adb` executable on PATH (scenario chosen via env SCENARIO).
#   A) platform logs "Displayed" for the activity -> PASS
#   B) process dies after launch, no Displayed -> FAIL (crash)
#   C) never Displayed, process alive -> per-attempt wait then retries -> FAIL
#   D) registry lists app but pidof transiently empty (the observed CI false
#      failure): must NOT be declared dead; retries until pass (no Displayed
#      here, so ends in the timeout-fail path but NEVER a crash-fail)
set -u
SCENARIO="${SCENARIO:?set SCENARIO=A|B|C}"
# Per-attempt wait inside launch_once is PER_ATTEMPT_TIMEOUT_S (90s); shorten
# it via env so the fail path completes quickly in the harness.
export PER_ATTEMPT_TIMEOUT_S="${PER_ATTEMPT_TIMEOUT_S:-4}"

# Must match crates/stratum-mobile/android/app/build.gradle.kts and
# AndroidManifest.xml (the smoke script's defaults, and the mock's answers).
PKG="com.stratum.app"
ACTIVITY="dev.gpui.mobile.GpuiActivity"
ACTIVITY_PATH="${PKG}/${ACTIVITY}"

HARNESS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MOCKBIN="$(mktemp -d)"
export APK_DIR="$(mktemp -d)"
export CI_HARVEST_DIR="$(mktemp -d)"
printf 'x' > "${APK_DIR}/app-universal-debug.apk"

cat > "${MOCKBIN}/adb" <<MOCK
#!/usr/bin/env bash
case "\$1" in
  install) exit 0 ;;
  exec-out) printf 'PNGDATA'; exit 0 ;;
  logcat)
    shift
    case "\$*" in
      -c*) exit 0 ;;
      -d*) if [ "$SCENARIO" = "A" ]; then printf 'ActivityTaskManager: Displayed ${ACTIVITY_PATH} for user 0: +1s\\\\n'; fi; exit 0 ;;
      *) exit 0 ;;
    esac ;;
  shell)
    shift
    case "\$*" in
      "pidof ${PKG}")
        # B = process truly dead: pidof empty. D = transient pidof miss (empty)
        # while the registry lists it — the observed CI false-failure. A/C = alive.
        if [ "$SCENARIO" = "B" ] || [ "$SCENARIO" = "D" ]; then :; else printf '12345'; fi ;;
      "cmd package resolve-activity --brief ${PKG}")
        printf '${ACTIVITY_PATH}' ;;
      "am start -n ${ACTIVITY_PATH}")
        exit 0 ;;
      "dumpsys activity processes")
        # B = registry does not list the package (dead). A/C/D = alive (D has
        # the transient pidof miss but the registry is authoritative).
        if [ "$SCENARIO" = "B" ]; then :; else printf '  * APP: ${PKG}/1001 (top-activity)'; fi ;;
      *) exit 0 ;;
    esac ;;
esac
exit 0
MOCK
chmod +x "${MOCKBIN}/adb"

export PATH="${MOCKBIN}:${PATH}"
cd "${HARNESS_DIR}"
bash .github/scripts/smoke-android.sh
echo "HARNESS_EXIT=$?"
