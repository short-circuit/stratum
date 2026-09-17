#!/usr/bin/env bash
# Local harness for smoke-android.sh control flow (no emulator needed).
# Provides a mock `adb` executable on PATH (scenario chosen via env SCENARIO).
#   A) am start -W Displayed -> PASS
#   B) process dies after launch -> FAIL (crash)
#   C) never visible, process alive -> retries 3x then FAIL (timeout)
set -u
SCENARIO="${SCENARIO:?set SCENARIO=A|B|C}"

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
  logcat) exit 0 ;;
  shell)
    shift
    case "\$*" in
      "pidof app.stratum.debug")
        if [ "$SCENARIO" = "B" ]; then echo -n ""; else echo -n "12345"; fi ;;
      "cmd package resolve-activity --brief app.stratum.debug")
        echo "app.stratum.debug/app.stratum.MainActivity" ;;
      "am start -W -n app.stratum.debug/app.stratum.MainActivity")
        case "$SCENARIO" in
          A) printf 'Status: ok\\nActivity: app.stratum.debug/app.stratum.MainActivity Displayed' ;;
          *) printf 'Status: ok\\nActivity: app.stratum.debug/app.stratum.MainActivity finished' ;;
        esac ;;
      "dumpsys activity processes")
        echo "  * APP: app.stratum.debug/1001 (top-activity)" ;;
      *) echo "  " ;;
    esac ;;
esac
exit 0
MOCK
chmod +x "${MOCKBIN}/adb"

export PATH="${MOCKBIN}:${PATH}"
cd "${HARNESS_DIR}"
bash .github/scripts/smoke-android.sh
echo "HARNESS_EXIT=$?"
