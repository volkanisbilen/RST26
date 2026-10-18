-- Restore Moradon NPC facing through database values.
--
-- The 2615 client reads GetNpcInfo direction as a byte angle. The server now
-- preserves npc_spawn.direction as 0..255, so this migration can rotate the
-- Moradon non-monster NPC rows without adding packet-side direction hacks.

UPDATE npc_spawn
   SET direction = MOD(MOD(direction, 256) + 128, 256)
 WHERE zone_id = 21
   AND is_monster = FALSE;

CREATE TABLE IF NOT EXISTS moraranker_statue_slot (
    slot_index SMALLINT PRIMARY KEY,
    npc_type CHAR(1) NOT NULL UNIQUE,
    nation SMALLINT NOT NULL,
    x REAL NOT NULL,
    z REAL NOT NULL,
    direction SMALLINT NOT NULL
);

INSERT INTO moraranker_statue_slot (slot_index, npc_type, nation, x, z, direction) VALUES
    (0, 'R', 1, 790.0, 561.0, 130),
    (1, 'S', 1, 782.0, 561.0, 130),
    (2, 'T', 1, 773.0, 561.0, 130),
    (3, 'U', 2, 842.0, 561.0, 134),
    (4, 'V', 2, 849.0, 561.0, 134),
    (5, 'W', 2, 858.0, 561.0, 134)
ON CONFLICT (slot_index) DO UPDATE SET
    npc_type = EXCLUDED.npc_type,
    nation = EXCLUDED.nation,
    x = EXCLUDED.x,
    z = EXCLUDED.z,
    direction = EXCLUDED.direction;
