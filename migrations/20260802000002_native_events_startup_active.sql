-- Attendance participates in the same runtime activation model as the other
-- native v2615 panels. The server also re-enables all rows on each startup;
-- GM close commands therefore remain effective until the next restart.
INSERT INTO native_event_config (event_key, active, cost, reset_hour)
VALUES ('attendance', TRUE, 0, 0)
ON CONFLICT (event_key) DO UPDATE SET
    active = TRUE,
    updated_at = NOW();

UPDATE native_event_config
SET active = TRUE, updated_at = NOW()
WHERE event_key IN ('attendance', 'roulette', 'jigsaw', 'coin', 'marble');
