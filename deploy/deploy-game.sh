#!/usr/bin/env bash
# One-shot game server deploy: pull, build the ko-server binary, install, restart, verify.
# Usage:  bash /opt/ko26xx-build-2625/deploy/deploy-game.sh          (game server)
#         bash /opt/ko26xx-build-2625/deploy/deploy-game.sh admin    (also ko-admin)
set -euo pipefail

BUILD=/opt/ko26xx-build-2625
cd "$BUILD"

echo "== git"
git fetch origin
git reset --hard origin/codex/2625-client-upgrade
git log --oneline -1

echo "== build ko-server (the binary lives in the ko-server package, not ko-game)"
cargo build --release -p ko-server
echo "binary built: $(stat -c '%y' target/release/ko-server)"

if [ "${1:-}" = "admin" ]; then
  echo "== build ko-admin"
  cargo build --release -p ko-admin
fi

echo "== install"
systemctl stop ko-server
cp target/release/ko-server /opt/ko26xx/ko-server
if [ "${1:-}" = "admin" ]; then
  systemctl stop ko-admin || true
  cp target/release/ko-admin /opt/ko26xx/ko-admin
  systemctl start ko-admin
fi
systemctl start ko-server

echo "== verify"
cmp target/release/ko-server /opt/ko26xx/ko-server && echo "SAME BINARY"
ls -l --time-style=full-iso /opt/ko26xx/ko-server
systemctl is-active ko-server
echo "DEPLOY OK"
