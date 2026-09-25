#!/usr/bin/env bash
set -euo pipefail

source /etc/ko-server.env
PUBLIC_IP="84.247.183.23"

echo "== current network rows =="
psql "$DATABASE_URL" -c "select * from game_options limit 5;"
psql "$DATABASE_URL" -c "select * from game_server_list order by id limit 10;"

echo "== update known localhost values =="
psql "$DATABASE_URL" -c "update game_options set patch_host = '$PUBLIC_IP' where patch_host in ('127.0.0.1', 'localhost') or patch_host is null;"
psql "$DATABASE_URL" -c "update game_options set login_host = '$PUBLIC_IP' where login_host in ('127.0.0.1', 'localhost') or login_host is null;"
psql "$DATABASE_URL" -c "update game_options set game_host = '$PUBLIC_IP' where game_host in ('127.0.0.1', 'localhost') or game_host is null;"
psql "$DATABASE_URL" -c "update game_server_list set server_ip = '$PUBLIC_IP' where server_ip in ('127.0.0.1', 'localhost') or server_ip is null;"

echo "== after =="
psql "$DATABASE_URL" -c "select * from game_options limit 5;"
psql "$DATABASE_URL" -c "select * from game_server_list order by id limit 10;"

systemctl restart ko-server
sleep 5
systemctl is-active ko-server
ss -lntp | grep -E ':(15001|15100|15101|15102|15103|15104|15105|15106|15107|15108|15109)\b' || true
