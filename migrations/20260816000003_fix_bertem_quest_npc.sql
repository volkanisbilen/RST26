-- [Tomb Keeper] Bertem (sSid 24441)
-- Evidence: KO_DATABASE_SERVER_001.dbo.K_NPC2369.  The PostgreSQL import had
-- byGroup/byActType/byType = 0/1/61, which makes the v2615 client classify the
-- quest keeper as a hostile entity.  Native source values are 3/4/64.
UPDATE npc_template
SET by_group = 3,
    by_act_type = 4,
    by_type = 64
WHERE s_sid = 24441
  AND is_monster = FALSE;

-- K_NPCPOS2369 identifies every Bertem spawn as mon_summon=0. Preserve the
-- NPC classification even if a previous local data patch changed a spawn.
UPDATE npc_spawn
SET is_monster = FALSE
WHERE npc_id = 24441;
