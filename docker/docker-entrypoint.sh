#!/bin/sh
set -eu

APP_USER="${APP_USER:-codexmanager}"
APP_UID="${APP_UID:-10001}"
DATA_DIR="${CODEXMANAGER_DATA_DIR:-/data}"

if [ "$(id -u)" = "0" ]; then
  mkdir -p "$DATA_DIR"

  # Bind mounts may arrive with restrictive host permissions. Fix them before
  # dropping privileges so service/web can share the same database and RPC token.
  chmod 0775 "$DATA_DIR" 2>/dev/null || true
  chown -R "$APP_UID:$APP_UID" "$DATA_DIR" 2>/dev/null || true

  exec gosu "$APP_USER:$APP_USER" "$@"
fi

exec "$@"
