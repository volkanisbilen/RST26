-- Moradon MORANKER statue/ranking test accounts.
--
-- The 2615 client sends the password value in the same encoded form stored in
-- tb_user.str_passwd. Copying the value inside PostgreSQL avoids exposing or
-- re-encoding the GM account password.
DO $$
DECLARE
    source_password VARCHAR(34);
BEGIN
    SELECT str_passwd
      INTO source_password
      FROM tb_user
     WHERE str_account_id = 'a';

    IF NOT FOUND THEN
        RAISE EXCEPTION
            'Source account "a" was not found; MORANKER test accounts were not created';
    END IF;

    INSERT INTO tb_user (
        str_account_id,
        str_passwd,
        str_authority,
        account_check,
        punishment_date,
        punishment_period
    )
    SELECT
        'a' || account_no::TEXT,
        source_password,
        1,
        1,
        NULL,
        0
    FROM generate_series(1, 10) AS account_no
    ON CONFLICT (str_account_id) DO UPDATE
       SET str_passwd        = EXCLUDED.str_passwd,
           str_authority     = 1,
           account_check     = 1,
           punishment_date   = NULL,
           punishment_period = 0;
END
$$;
DO $$
DECLARE
    matching_accounts INTEGER;
BEGIN
    SELECT COUNT(*)
      INTO matching_accounts
      FROM tb_user AS target
      JOIN tb_user AS source
        ON source.str_account_id = 'a'
       AND target.str_passwd = source.str_passwd
     WHERE target.str_account_id IN (
        'a1', 'a2', 'a3', 'a4', 'a5',
        'a6', 'a7', 'a8', 'a9', 'a10'
     )
       AND target.str_authority = 1
       AND target.account_check = 1;

    IF matching_accounts <> 10 THEN
        RAISE EXCEPTION
            'MORANKER account verification failed: expected 10, found %',
            matching_accounts;
    END IF;
END
$$;
