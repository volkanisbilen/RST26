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

echo "== quest scripts"
# The server runs in /opt/ko26xx and reads its Lua scripts from /opt/ko26xx/Quests (the panel's
# Quest Studio edits that folder too), so only the scripts that changed in git since the last
# deploy are copied; scripts edited in the panel are never overwritten wholesale.
MARK=/opt/ko26xx/.deployed_commit
LAST=$(cat "$MARK" 2>/dev/null || echo a2e720c)
if git cat-file -e "$LAST^{commit}" 2>/dev/null; then
  for f in $(git diff --name-only "$LAST" HEAD -- Quests); do
    if [ -f "$f" ]; then
      mkdir -p /opt/ko26xx/Quests
      cp "$f" "/opt/ko26xx/$f" && echo "script updated: $f"
    fi
  done
fi
git rev-parse HEAD > "$MARK"

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
