-- Restore the exact LoginServer/GameServer wire version captured in
-- ko-server_20260730_160209.log, where character selection and InGame
-- completed successfully: [01][2614][10][16-byte AES key][00].
-- Client Server.ini Files=2613 is a patch/files value, not this wire value.
UPDATE server_settings
SET game_version = 2614
WHERE server_no = 1;
