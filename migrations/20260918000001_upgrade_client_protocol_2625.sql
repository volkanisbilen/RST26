-- KnightOnline.exe v2625 protocol baseline.
--
-- Evidence from the supplied unpacked executable:
--   sub_7AE420 returns 2625 (required GameServer wire version)
--   sub_7B5420 compares the version response against 2625
--   on mismatch the client rolls Server.ini back to Files=2624
-- Launcher and GameServer therefore both advertise 2625.
UPDATE server_settings
SET game_version = 2625,
    launcher_version = 2625
WHERE server_no = 1;
