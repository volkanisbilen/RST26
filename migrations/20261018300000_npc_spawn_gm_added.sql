-- npc_spawn.gm_added: marks rows created by the GM command +ekle (circle placement). Idempotent.
-- Table owner must stay koserver (ALTER ADD COLUMN does not change ownership).

ALTER TABLE npc_spawn ADD COLUMN IF NOT EXISTS gm_added BOOLEAN NOT NULL DEFAULT FALSE;
