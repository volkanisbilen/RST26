$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$container = "korust-postgres"

$running = docker ps --filter "name=^/$container$" --format "{{.Names}}"
if ($running -ne $container) {
    docker compose -f (Join-Path $repo "docker-compose.local.yml") up -d postgres
}

$sql = @'
DO $$
DECLARE
    selected_count integer;
BEGIN
    WITH candidates AS (
        SELECT u.str_user_id,
               u.nation,
               ROW_NUMBER() OVER (
                   PARTITION BY u.nation
                   ORDER BY length(ac.str_account_id), ac.str_account_id,
                            lower(u.str_user_id), u.str_user_id
               ) AS nation_rank
          FROM userdata u
          JOIN account_char ac
            ON u.str_user_id IN (
                ac.str_char_id1, ac.str_char_id2,
                ac.str_char_id3, ac.str_char_id4
            )
         WHERE ac.str_account_id ~ '^a([1-9]|10)$'
           AND u.nation IN (1, 2)
    ), chosen AS (
        SELECT * FROM candidates WHERE nation_rank <= 3
    )
    SELECT count(*) INTO selected_count FROM chosen;

    IF selected_count <> 6 THEN
        RAISE EXCEPTION
            'Expected exactly 3 Karus and 3 Human characters on a1-a10; found % total',
            selected_count;
    END IF;

    WITH candidates AS (
        SELECT u.str_user_id,
               u.nation,
               ROW_NUMBER() OVER (
                   PARTITION BY u.nation
                   ORDER BY length(ac.str_account_id), ac.str_account_id,
                            lower(u.str_user_id), u.str_user_id
               ) AS nation_rank
          FROM userdata u
          JOIN account_char ac
            ON u.str_user_id IN (
                ac.str_char_id1, ac.str_char_id2,
                ac.str_char_id3, ac.str_char_id4
            )
         WHERE ac.str_account_id ~ '^a([1-9]|10)$'
           AND u.nation IN (1, 2)
    ), chosen AS (
        SELECT * FROM candidates WHERE nation_rank <= 3
    )
    UPDATE userdata u
       SET loyalty = CASE chosen.nation_rank
                         WHEN 1 THEN 2100000000
                         WHEN 2 THEN 2099000000
                         ELSE 2098000000
                     END,
           str_memo = LEFT(format(
               '%s #%s rank memo',
               CASE chosen.nation WHEN 1 THEN 'Karus' ELSE 'Human' END,
               chosen.nation_rank
           ), 20),
           dt_update_time = NOW()
      FROM chosen
     WHERE u.str_user_id = chosen.str_user_id;
END $$;

SELECT nation,
       ROW_NUMBER() OVER (
           PARTITION BY nation
           ORDER BY loyalty DESC, loyalty_monthly DESC,
                    lower(str_user_id), str_user_id
       ) AS rank,
       str_user_id,
       loyalty,
       COALESCE(str_memo, '') AS memo
  FROM userdata
 WHERE nation IN (1, 2)
 ORDER BY nation, rank
 LIMIT 20;
'@

Write-Host "Preparing MORANKER NP and memo test data..." -ForegroundColor Cyan
$sql | docker exec -i $container psql -v ON_ERROR_STOP=1 -U koserver -d ko_server
if ($LASTEXITCODE -ne 0) {
    throw "MORANKER test data preparation failed."
}

Write-Host "MORANKER test data is ready." -ForegroundColor Green
Write-Host "Use +reloadranks in game, or restart the server." -ForegroundColor Yellow
