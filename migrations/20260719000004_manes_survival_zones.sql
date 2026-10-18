-- Official Manes Survival client zone mapping.
-- Client Zones.tbl stores 570/580/590/600; game-server zone IDs are 57/58/59/60.

BEGIN;

INSERT INTO zone_info (
    zone_no, smd_name, zone_name, zone_type, min_level, max_level,
    init_x, init_z, init_y
) VALUES
    (57, 'manes_survival.smd', 'Manes Survival I',   1, 1, 83, 512000, 512000, 0),
    (58, 'manes_survival.smd', 'Manes Survival II',  1, 1, 83, 512000, 512000, 0),
    (59, 'manes_survival.smd', 'Manes Survival III', 1, 1, 83, 512000, 512000, 0),
    (60, 'manes_survival.smd', 'Manes Survival IV',  1, 1, 83, 512000, 512000, 0)
ON CONFLICT (zone_no) DO UPDATE SET
    smd_name  = EXCLUDED.smd_name,
    zone_name = EXCLUDED.zone_name,
    zone_type = EXCLUDED.zone_type,
    min_level = EXCLUDED.min_level,
    max_level = EXCLUDED.max_level,
    init_x    = EXCLUDED.init_x,
    init_z    = EXCLUDED.init_z,
    init_y    = EXCLUDED.init_y;

INSERT INTO start_position (
    zone_id, karus_x, karus_z, elmorad_x, elmorad_z,
    karus_gate_x, karus_gate_z, elmo_gate_x, elmo_gate_z, range_x, range_z
) VALUES
    (57, 512, 512, 512, 512, 512, 512, 512, 512, 0, 0),
    (58, 512, 512, 512, 512, 512, 512, 512, 512, 0, 0),
    (59, 512, 512, 512, 512, 512, 512, 512, 512, 0, 0),
    (60, 512, 512, 512, 512, 512, 512, 512, 512, 0, 0)
ON CONFLICT (zone_id) DO UPDATE SET
    karus_x = EXCLUDED.karus_x,
    karus_z = EXCLUDED.karus_z,
    elmorad_x = EXCLUDED.elmorad_x,
    elmorad_z = EXCLUDED.elmorad_z,
    karus_gate_x = EXCLUDED.karus_gate_x,
    karus_gate_z = EXCLUDED.karus_gate_z,
    elmo_gate_x = EXCLUDED.elmo_gate_x,
    elmo_gate_z = EXCLUDED.elmo_gate_z,
    range_x = EXCLUDED.range_x,
    range_z = EXCLUDED.range_z;

-- Remove only the temporary incorrect mapping created for this feature.
DELETE FROM zone_info
WHERE zone_no = 96
  AND lower(smd_name) = 'manes_survival.smd';

DO $$
BEGIN
    IF (SELECT COUNT(*) FROM zone_info
        WHERE zone_no BETWEEN 57 AND 60
          AND lower(smd_name) = 'manes_survival.smd') <> 4 THEN
        RAISE EXCEPTION 'Official Manes Survival zone mapping installation failed';
    END IF;
END $$;

COMMIT;
