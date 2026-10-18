#!/usr/bin/env bash
set -euo pipefail

source /etc/ko-server.env

echo "== last migrations =="
psql "$DATABASE_URL" -c "\d _sqlx_migrations"
psql "$DATABASE_URL" -c "select * from _sqlx_migrations order by version desc limit 8;"

echo "== clear failed migration rows if needed =="
psql "$DATABASE_URL" -c "delete from _sqlx_migrations where success = false;"
psql "$DATABASE_URL" -c "delete from _sqlx_migrations where version = 20260925000001;"

echo "== restart =="
systemctl restart ko-server
sleep 2
systemctl is-active ko-server
systemctl status ko-server --no-pager -n 30 -l
