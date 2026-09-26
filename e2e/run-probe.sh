#!/usr/bin/env bash
# Clean-slate wrapper: kill any orphan app+driver, start fresh driver, run UI probe
set -uo pipefail
WKD=/nix/store/2348bqnh7jhzq48dp6kjvc014rmipr96-webkitgtk-2.52.3+abi=4.1/bin/WebKitWebDriver

# Kill orphans
kill $(pgrep -f 'stratum-tauri' 2>/dev/null) 2>/dev/null
kill $(pgrep -f 'tauri-driver' 2>/dev/null) 2>/dev/null
kill $(pgrep -f 'WebKitWebDriver' 2>/dev/null) 2>/dev/null
sleep 1
echo "=== orphans cleared ==="

# Start driver with controlled env
DISPLAY=:98 HOME=/tmp/stratum-home GDK_BACKEND=x11 WAYLAND_DISPLAY= \
WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 \
WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 LIBGL_ALWAYS_SOFTWARE=1 \
tauri-driver --port 4444 --native-driver "$WKD" >/tmp/td.log 2>&1 &
TD=$!
sleep 3
curl -s -m 3 http://127.0.0.1:4444/status && echo " DRIVER-READY"

# Run the probe
node "$1" 2>&1 | grep -v "INFO webdriver" | grep -v "WARN webdriver" | grep -v "@wdio"

echo "=== cleanup ==="
kill $TD 2>/dev/null
kill $(pgrep -f 'stratum-tauri' 2>/dev/null) 2>/dev/null
exit 0
