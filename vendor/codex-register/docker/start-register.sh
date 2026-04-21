#!/bin/sh
set -eu

DISPLAY_VALUE="${DISPLAY:-:99}"
HOTMAIL_HANDOFF_ENABLED="${HOTMAIL_HANDOFF_ENABLED:-0}"
HOTMAIL_HANDOFF_PORT="${HOTMAIL_HANDOFF_PORT:-7900}"
HOTMAIL_HANDOFF_VNC_PORT="${HOTMAIL_HANDOFF_VNC_PORT:-5900}"
HOTMAIL_HANDOFF_SCREEN="${HOTMAIL_HANDOFF_SCREEN:-1920x1080x24}"

XVFB_PID=""
X11VNC_PID=""
WEBSOCKIFY_PID=""

cleanup() {
  for pid in "$WEBSOCKIFY_PID" "$X11VNC_PID" "$XVFB_PID"; do
    if [ -n "$pid" ]; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
}

if [ "$HOTMAIL_HANDOFF_ENABLED" = "1" ]; then
  export DISPLAY="$DISPLAY_VALUE"

  Xvfb "$DISPLAY" -screen 0 "$HOTMAIL_HANDOFF_SCREEN" -ac +extension RANDR >/tmp/xvfb.log 2>&1 &
  XVFB_PID="$!"

  x11vnc \
    -display "$DISPLAY" \
    -forever \
    -shared \
    -rfbport "$HOTMAIL_HANDOFF_VNC_PORT" \
    -nopw >/tmp/x11vnc.log 2>&1 &
  X11VNC_PID="$!"

  websockify \
    --web=/usr/share/novnc/ \
    "$HOTMAIL_HANDOFF_PORT" \
    "127.0.0.1:$HOTMAIL_HANDOFF_VNC_PORT" >/tmp/websockify.log 2>&1 &
  WEBSOCKIFY_PID="$!"

  trap cleanup EXIT INT TERM
  sleep 1
fi

exec python webui.py
