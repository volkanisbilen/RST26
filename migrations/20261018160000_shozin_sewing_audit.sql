-- Shozin (special part sewing) audit, 2026-10-18.
-- Idempotent and live-safe: touches only item_special_sewing (+ a new map table).
-- See docs/SHOZIN_AUDIT.md for the evidence behind every statement.
-- Apply:  psql "$DATABASE_URL" -f migrations/20261018160000_shozin_sewing_audit.sql
--         then restart ko-game (recipes are loaded at startup).
-- Safe to apply BEFORE or AFTER the new binary: the binary treats the map
-- table as optional.

BEGIN;

-- 1. server recipe -> client mixtable row (Shozin result message index).
CREATE TABLE IF NOT EXISTS item_special_sewing_client_map (
    n_index   INTEGER  PRIMARY KEY,   -- item_special_sewing.n_index, or -npc_id for the NPC fallback row
    mix_index SMALLINT NOT NULL       -- 1-based row of the client's mixtable_us.tbl
);

-- 0. data repairs / additions
DELETE FROM item_special_sewing WHERE n_index = 5047 AND npc_id = 1; -- duplicate of healthy row 5028
DELETE FROM item_special_sewing WHERE n_index = 5048 AND npc_id = 1; -- duplicate of healthy row 5029
DELETE FROM item_special_sewing WHERE n_index = 5049 AND npc_id = 1; -- duplicate of healthy row 5030
DELETE FROM item_special_sewing WHERE n_index = 5050 AND npc_id = 1; -- duplicate of healthy row 5031
DELETE FROM item_special_sewing WHERE n_index = 5051 AND npc_id = 1; -- duplicate of healthy row 5032
DELETE FROM item_special_sewing WHERE n_index = 5052 AND npc_id = 1; -- duplicate of healthy row 5033
DELETE FROM item_special_sewing WHERE n_index = 5053 AND npc_id = 1; -- duplicate of healthy row 5034
DELETE FROM item_special_sewing WHERE n_index = 5054 AND npc_id = 1; -- duplicate of healthy row 5035
DELETE FROM item_special_sewing WHERE n_index = 5055 AND npc_id = 1; -- duplicate of healthy row 5036
DELETE FROM item_special_sewing WHERE n_index = 5056 AND npc_id = 1; -- duplicate of healthy row 5037
DELETE FROM item_special_sewing WHERE n_index = 5057 AND npc_id = 1; -- duplicate of healthy row 5038
DELETE FROM item_special_sewing WHERE n_index = 5058 AND npc_id = 1; -- duplicate of healthy row 5039
DELETE FROM item_special_sewing WHERE n_index = 5059 AND npc_id = 1; -- duplicate of healthy row 5040
DELETE FROM item_special_sewing WHERE n_index = 5060 AND npc_id = 1; -- duplicate of healthy row 5041
DELETE FROM item_special_sewing WHERE n_index = 5061 AND npc_id = 1; -- duplicate of healthy row 5042
DELETE FROM item_special_sewing WHERE n_index = 5062 AND npc_id = 1; -- duplicate of healthy row 5043
DELETE FROM item_special_sewing WHERE n_index = 5063 AND npc_id = 1; -- duplicate of healthy row 5044
DELETE FROM item_special_sewing WHERE n_index = 5064 AND npc_id = 1; -- duplicate of healthy row 5045
DELETE FROM item_special_sewing WHERE n_index = 5065 AND npc_id = 1; -- duplicate of healthy row 5046
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3925 AND b.n_index = 2714 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3926 AND b.n_index = 2715 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3927 AND b.n_index = 2716 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3928 AND b.n_index = 2717 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3929 AND b.n_index = 2718 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3930 AND b.n_index = 2719 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3931 AND b.n_index = 2720 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3932 AND b.n_index = 2721 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3933 AND b.n_index = 2722 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3934 AND b.n_index = 2723 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3935 AND b.n_index = 2724 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3936 AND b.n_index = 2725 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3937 AND b.n_index = 2726 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3938 AND b.n_index = 2727 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3939 AND b.n_index = 2728 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3940 AND b.n_index = 2729 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3941 AND b.n_index = 2730 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3942 AND b.n_index = 2731 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3943 AND b.n_index = 2732 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3944 AND b.n_index = 2733 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3945 AND b.n_index = 2734 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3946 AND b.n_index = 2735 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3947 AND b.n_index = 2736 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3948 AND b.n_index = 2737 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3977 AND b.n_index = 2738 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3978 AND b.n_index = 2739 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3979 AND b.n_index = 2740 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3980 AND b.n_index = 2741 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3981 AND b.n_index = 2742 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3982 AND b.n_index = 2743 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 3983 AND b.n_index = 2744 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4318 AND b.n_index = 2699 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4319 AND b.n_index = 2700 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4320 AND b.n_index = 2701 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4321 AND b.n_index = 2702 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4322 AND b.n_index = 2703 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4323 AND b.n_index = 2704 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4324 AND b.n_index = 2705 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4325 AND b.n_index = 2706 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4326 AND b.n_index = 2707 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4327 AND b.n_index = 2708 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4328 AND b.n_index = 2709 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4329 AND b.n_index = 2710 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4330 AND b.n_index = 2711 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4331 AND b.n_index = 2712 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4332 AND b.n_index = 2713 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4992 AND b.n_index = 4985 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 4993 AND b.n_index = 4986 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5006 AND b.n_index = 4987 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5007 AND b.n_index = 4988 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5008 AND b.n_index = 4989 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5009 AND b.n_index = 4990 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5010 AND b.n_index = 4991 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5011 AND b.n_index = 4985 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5012 AND b.n_index = 4986 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5013 AND b.n_index = 4994 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5014 AND b.n_index = 4995 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5015 AND b.n_index = 4996 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5016 AND b.n_index = 4997 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5017 AND b.n_index = 4998 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5018 AND b.n_index = 4999 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5019 AND b.n_index = 5000 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5020 AND b.n_index = 5001 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5021 AND b.n_index = 5002 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5022 AND b.n_index = 5003 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5023 AND b.n_index = 5004 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5024 AND b.n_index = 5005 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5025 AND b.n_index = 4987 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5026 AND b.n_index = 4988 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5027 AND b.n_index = 4989 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5028 AND b.n_index = 4990 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5029 AND b.n_index = 4991 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5030 AND b.n_index = 4985 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5031 AND b.n_index = 4986 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5032 AND b.n_index = 4994 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5033 AND b.n_index = 4995 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5034 AND b.n_index = 4996 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5035 AND b.n_index = 4997 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5036 AND b.n_index = 4998 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5037 AND b.n_index = 4999 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5038 AND b.n_index = 5000 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5039 AND b.n_index = 5001 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5040 AND b.n_index = 5002 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5041 AND b.n_index = 5003 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5042 AND b.n_index = 5004 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5043 AND b.n_index = 5005 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5044 AND b.n_index = 4987 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5045 AND b.n_index = 4988 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
DELETE FROM item_special_sewing a USING item_special_sewing b WHERE a.n_index = 5046 AND b.n_index = 4989 AND a.npc_id = b.npc_id AND a.give_item_id = b.give_item_id AND a.give_item_count = b.give_item_count AND a.success_rate = b.success_rate AND a.is_notice = b.is_notice AND a.is_shadow_success = b.is_shadow_success AND (a.req_item_id_1,a.req_item_count_1,a.req_item_id_2,a.req_item_count_2,a.req_item_id_3,a.req_item_count_3,a.req_item_id_4,a.req_item_count_4,a.req_item_id_5,a.req_item_count_5,a.req_item_id_6,a.req_item_count_6,a.req_item_id_7,a.req_item_count_7,a.req_item_id_8,a.req_item_count_8,a.req_item_id_9,a.req_item_count_9,a.req_item_id_10,a.req_item_count_10) = (b.req_item_id_1,b.req_item_count_1,b.req_item_id_2,b.req_item_count_2,b.req_item_id_3,b.req_item_count_3,b.req_item_id_4,b.req_item_count_4,b.req_item_id_5,b.req_item_count_5,b.req_item_id_6,b.req_item_count_6,b.req_item_id_7,b.req_item_count_7,b.req_item_id_8,b.req_item_count_8,b.req_item_id_9,b.req_item_count_9,b.req_item_id_10,b.req_item_count_10);
INSERT INTO item_special_sewing (n_index, description, req_item_id_1, req_item_count_1, req_item_id_2, req_item_count_2, req_item_id_3, req_item_count_3, req_item_id_4, req_item_count_4, req_item_id_5, req_item_count_5, req_item_id_6, req_item_count_6, req_item_id_7, req_item_count_7, req_item_id_8, req_item_count_8, req_item_id_9, req_item_count_9, req_item_id_10, req_item_count_10, give_item_id, give_item_count, success_rate, npc_id, is_notice, is_shadow_success) VALUES
  (6001, 'Hepas Weapon', 121210001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136350001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Slayer (+1) <- client mixtable row 70 (cat 9),
  (6002, 'Hepas Weapon', 121310001, 1, 508168000, 2, 508245000, 4, 508243000, 5, 508244000, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136360001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Graham (+1) <- client mixtable row 71 (cat 9),
  (6003, 'Hepas Weapon', 121410001, 1, 508168000, 5, 508245000, 8, 508243000, 7, 508244000, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136370001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Exceptional Graham (+1) <- client mixtable row 72 (cat 9),
  (6004, 'Hepas Weapon', 126310001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136380001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Durandal (+1) <- client mixtable row 73 (cat 9),
  (6005, 'Hepas Weapon', 131010001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136410001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Tomahawk (+1) <- client mixtable row 76 (cat 9),
  (6006, 'Hepas Weapon', 131110001, 1, 508168000, 2, 508245000, 4, 508243000, 5, 508244000, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136420001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Deep Scar (+1) <- client mixtable row 77 (cat 9),
  (6007, 'Hepas Weapon', 131210001, 1, 508168000, 5, 508245000, 8, 508243000, 7, 508244000, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136430001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Exceptional Deep Scar (+1) <- client mixtable row 78 (cat 9),
  (6008, 'Hepas Weapon', 136110001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136440001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Bone Cleaver (+1) <- client mixtable row 79 (cat 9),
  (6009, 'Hepas Weapon', 141010001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136470001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Bone Crasher (+1) <- client mixtable row 82 (cat 9),
  (6010, 'Hepas Weapon', 146110001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136500001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Totamic Club (+1) <- client mixtable row 85 (cat 9),
  (6011, 'Hepas Weapon', 151110001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136530001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Harpoon (+1) <- client mixtable row 88 (cat 9),
  (6012, 'Hepas Weapon', 156110001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136560001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Glave (+1) <- client mixtable row 91 (cat 9),
  (6013, 'Hepas Weapon', 168310001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136590001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Iron Crossbow (+1) <- client mixtable row 94 (cat 9),
  (6014, 'Hepas Weapon', 181010001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136620001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Salamander Staff (+1) <- client mixtable row 97 (cat 9),
  (6015, 'Hepas Weapon', 191010001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136650001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Priest Bone crasher (+1) <- client mixtable row 100 (cat 9),
  (6016, 'Hepas Weapon', 191110001, 1, 508168000, 2, 508245000, 4, 508243000, 5, 508244000, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136660001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Priest Impact (+1) <- client mixtable row 101 (cat 9),
  (6017, 'Hepas Weapon', 191210001, 1, 508168000, 5, 508245000, 8, 508243000, 7, 508244000, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136670001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Exceptional Priest Impact (+1) <- client mixtable row 102 (cat 9),
  (6018, 'Hepas Weapon', 171550001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136680001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Hoplon (+1) <- client mixtable row 103 (cat 9),
  (6019, 'Hepas Weapon', 171560001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136690001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Sacred Shield (+1) <- client mixtable row 104 (cat 9),
  (6020, 'Hepas Weapon', 171570001, 1, 508168000, 2, 508245000, 3, 508243000, 5, 508244000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136700001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Sacred Defender (+1) <- client mixtable row 105 (cat 9),
  (6021, 'Hepas Weapon', 171580001, 1, 508168000, 5, 508245000, 4, 508243000, 7, 508244000, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136710001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Tentora Shield (+1) <- client mixtable row 106 (cat 9),
  (6022, 'Hepas Weapon', 171590001, 1, 508168000, 5, 508245000, 4, 508243000, 7, 508244000, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136720001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Tentora Defender (+1) <- client mixtable row 107 (cat 9),
  (6023, 'Hepas Weapon', 171600001, 1, 508168000, 5, 508245000, 4, 508243000, 7, 508244000, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136730001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Edmont Aegis (+1) <- client mixtable row 108 (cat 9),
  (6024, 'Hepas Weapon', 171610001, 1, 508168000, 5, 508245000, 4, 508243000, 7, 508244000, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136740001, 1, 2500, 19073, TRUE, TRUE),  -- Hepa's Blessing Aegis (+1) <- client mixtable row 109 (cat 9),
  (6025, 'Holy Knight Armor Exchange(+1)', 810953000, 1, 890103000, 1, 810959000, 1, 890108000, 3, 810961000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 231511001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Warrior Pauldron (+1) <- client mixtable row 460 (cat 20),
  (6026, 'Holy Knight Armor Exchange(+1)', 810954000, 1, 890103000, 1, 810959000, 1, 890108000, 3, 810961000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 231512001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Warrior Pads (+1) <- client mixtable row 461 (cat 20),
  (6027, 'Holy Knight Armor Exchange(+1)', 810955000, 1, 890103000, 1, 810959000, 1, 890108000, 2, 810961000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 231513001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Warrior Helmet (+1) <- client mixtable row 462 (cat 20),
  (6028, 'Holy Knight Armor Exchange(+1)', 810956000, 1, 890103000, 1, 810959000, 1, 890108000, 1, 810961000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 231514001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Warrior Gauntlets (+1) <- client mixtable row 463 (cat 20),
  (6029, 'Holy Knight Armor Exchange(+1)', 810957000, 1, 890103000, 1, 810959000, 1, 890108000, 1, 810961000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 231515001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Warrior Boots (+1) <- client mixtable row 464 (cat 20),
  (6030, 'Holy Knight Armor Exchange(+1)', 810953000, 1, 890104000, 1, 810959000, 1, 890109000, 3, 810962000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 232511001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Assassin Pauldron (+1) <- client mixtable row 465 (cat 20),
  (6031, 'Holy Knight Armor Exchange(+1)', 810954000, 1, 890104000, 1, 810959000, 1, 890109000, 3, 810962000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 232512001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Assassin Pads (+1) <- client mixtable row 466 (cat 20),
  (6032, 'Holy Knight Armor Exchange(+1)', 810955000, 1, 890104000, 1, 810959000, 1, 890109000, 2, 810962000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 232513001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Assassin Helmet (+1) <- client mixtable row 467 (cat 20),
  (6033, 'Holy Knight Armor Exchange(+1)', 810956000, 1, 890104000, 1, 810959000, 1, 890109000, 1, 810962000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 232514001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Assassin Gauntlets (+1) <- client mixtable row 468 (cat 20),
  (6034, 'Holy Knight Armor Exchange(+1)', 810957000, 1, 890104000, 1, 810959000, 1, 890109000, 1, 810962000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 232515001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Assassin Boots (+1) <- client mixtable row 469 (cat 20),
  (6035, 'Holy Knight Armor Exchange(+1)', 810953000, 1, 890105000, 1, 810959000, 1, 890110000, 3, 810963000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 233511001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Mage Robe (+1) <- client mixtable row 470 (cat 20),
  (6036, 'Holy Knight Armor Exchange(+1)', 810954000, 1, 890105000, 1, 810959000, 1, 890110000, 3, 810963000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 233512001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Mage Pants (+1) <- client mixtable row 471 (cat 20),
  (6037, 'Holy Knight Armor Exchange(+1)', 810955000, 1, 890105000, 1, 810959000, 1, 890110000, 2, 810963000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 233513001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Mage Helmet (+1) <- client mixtable row 472 (cat 20),
  (6038, 'Holy Knight Armor Exchange(+1)', 810956000, 1, 890105000, 1, 810959000, 1, 890110000, 1, 810963000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 233514001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Mage Gloves (+1) <- client mixtable row 473 (cat 20),
  (6039, 'Holy Knight Armor Exchange(+1)', 810957000, 1, 890105000, 1, 810959000, 1, 890110000, 1, 810963000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 233515001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Mage Boots (+1) <- client mixtable row 474 (cat 20),
  (6040, 'Holy Knight Armor Exchange(+1)', 810953000, 1, 890106000, 1, 810959000, 1, 890111000, 3, 810964000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 234511001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Priest Pauldron (+1) <- client mixtable row 475 (cat 20),
  (6041, 'Holy Knight Armor Exchange(+1)', 810954000, 1, 890106000, 1, 810959000, 1, 890111000, 3, 810964000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 234512001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Priest Pads (+1) <- client mixtable row 476 (cat 20),
  (6042, 'Holy Knight Armor Exchange(+1)', 810955000, 1, 890106000, 1, 810959000, 1, 890111000, 2, 810964000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 234513001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Priest Helmet (+1) <- client mixtable row 477 (cat 20),
  (6043, 'Holy Knight Armor Exchange(+1)', 810956000, 1, 890106000, 1, 810959000, 1, 890111000, 1, 810964000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 234514001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Priest Gauntlets (+1) <- client mixtable row 478 (cat 20),
  (6044, 'Holy Knight Armor Exchange(+1)', 810957000, 1, 890106000, 1, 810959000, 1, 890111000, 1, 810964000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 234515001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Priest Boots (+1) <- client mixtable row 479 (cat 20),
  (6045, 'Holy Knight Armor Exchange(+1)', 810953000, 1, 890107000, 1, 810959000, 1, 890112000, 3, 810965000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239511001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Portu Pauldron (+1) <- client mixtable row 480 (cat 20),
  (6046, 'Holy Knight Armor Exchange(+1)', 810954000, 1, 890107000, 1, 810959000, 1, 890112000, 3, 810965000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239512001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Portu Pads (+1) <- client mixtable row 481 (cat 20),
  (6047, 'Holy Knight Armor Exchange(+1)', 810955000, 1, 890107000, 1, 810959000, 1, 890112000, 2, 810965000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239513001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Portu Helmet (+1) <- client mixtable row 482 (cat 20),
  (6048, 'Holy Knight Armor Exchange(+1)', 810956000, 1, 890107000, 1, 810959000, 1, 890112000, 1, 810965000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239514001, 1, 3350, 19073, TRUE, TRUE),  -- Holy Knight Portu Gauntlets (+1) <- client mixtable row 483 (cat 20),
  (6049, 'Holy Knight Armor Exchange(+1)', 810957000, 1, 890107000, 1, 810959000, 1, 890112000, 1, 810965000, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239515001, 1, 3350, 19073, TRUE, TRUE)  -- Holy Knight Portu Boots (+1) <- client mixtable row 484 (cat 20)
ON CONFLICT (n_index) DO NOTHING;

-- 2. client map (derived from client mixtable_us.tbl: same NPC + same material list; result match preferred)
INSERT INTO item_special_sewing_client_map (n_index, mix_index) VALUES
  (1, 1),
  (2, 1),
  (3, 1),
  (4, 1),
  (5, 2),
  (6, 2),
  (7, 2),
  (8, 2),
  (9, 3),
  (10, 3),
  (11, 3),
  (12, 3),
  (13, 4),
  (14, 4),
  (15, 4),
  (16, 4),
  (17, 5),
  (18, 5),
  (19, 5),
  (20, 5),
  (21, 5),
  (22, 6),
  (23, 6),
  (24, 6),
  (25, 7),
  (26, 7),
  (27, 7),
  (28, 8),
  (29, 8),
  (30, 8),
  (31, 9),
  (32, 9),
  (33, 9),
  (34, 10),
  (35, 10),
  (36, 259),
  (37, 259),
  (38, 259),
  (39, 259),
  (40, 11),
  (41, 12),
  (42, 13),
  (43, 14),
  (44, 15),
  (45, 11),
  (46, 12),
  (47, 13),
  (48, 14),
  (49, 15),
  (50, 11),
  (51, 12),
  (52, 13),
  (53, 14),
  (54, 15),
  (55, 11),
  (56, 12),
  (57, 13),
  (58, 14),
  (59, 15),
  (60, 11),
  (61, 12),
  (62, 13),
  (63, 14),
  (64, 15),
  (65, 157),
  (66, 158),
  (67, 159),
  (68, 160),
  (69, 161),
  (70, 162),
  (71, 163),
  (72, 164),
  (73, 165),
  (74, 166),
  (75, 167),
  (76, 168),
  (77, 169),
  (78, 170),
  (79, 171),
  (80, 172),
  (81, 173),
  (82, 174),
  (83, 175),
  (84, 176),
  (85, 254),
  (86, 255),
  (87, 256),
  (88, 257),
  (89, 258),
  (90, 157),
  (91, 158),
  (92, 159),
  (93, 160),
  (94, 161),
  (95, 162),
  (96, 163),
  (97, 164),
  (98, 165),
  (99, 166),
  (100, 167),
  (101, 168),
  (102, 169),
  (103, 170),
  (104, 171),
  (105, 172),
  (106, 173),
  (107, 174),
  (108, 175),
  (109, 176),
  (110, 254),
  (111, 255),
  (112, 256),
  (113, 257),
  (114, 258),
  (115, 157),
  (116, 158),
  (117, 159),
  (118, 160),
  (119, 161),
  (120, 162),
  (121, 163),
  (122, 164),
  (123, 165),
  (124, 166),
  (125, 167),
  (126, 168),
  (127, 169),
  (128, 170),
  (129, 171),
  (130, 172),
  (131, 173),
  (132, 174),
  (133, 175),
  (134, 176),
  (135, 254),
  (136, 255),
  (137, 256),
  (138, 257),
  (139, 258),
  (140, 157),
  (141, 158),
  (142, 159),
  (143, 160),
  (144, 161),
  (145, 162),
  (146, 163),
  (147, 164),
  (148, 165),
  (149, 166),
  (150, 167),
  (151, 168),
  (152, 169),
  (153, 170),
  (154, 171),
  (155, 172),
  (156, 173),
  (157, 174),
  (158, 175),
  (159, 176),
  (160, 254),
  (161, 255),
  (162, 256),
  (163, 257),
  (164, 258),
  (165, 442),
  (166, 443),
  (167, 444),
  (168, 445),
  (169, 446),
  (170, 447),
  (171, 448),
  (172, 449),
  (173, 450),
  (174, 451),
  (175, 452),
  (176, 453),
  (177, 454),
  (178, 455),
  (179, 456),
  (180, 432),
  (181, 433),
  (182, 434),
  (183, 435),
  (184, 436),
  (185, 437),
  (186, 438),
  (187, 439),
  (188, 440),
  (189, 441),
  (190, 442),
  (191, 443),
  (192, 444),
  (193, 445),
  (194, 446),
  (195, 447),
  (196, 448),
  (197, 449),
  (198, 450),
  (199, 451),
  (200, 452),
  (201, 453),
  (202, 454),
  (203, 455),
  (204, 456),
  (205, 432),
  (206, 433),
  (207, 434),
  (208, 435),
  (209, 436),
  (210, 437),
  (211, 438),
  (212, 439),
  (213, 440),
  (214, 441),
  (215, 442),
  (216, 443),
  (217, 444),
  (218, 445),
  (219, 446),
  (220, 447),
  (221, 448),
  (222, 449),
  (223, 450),
  (224, 451),
  (225, 452),
  (226, 453),
  (227, 454),
  (228, 455),
  (229, 456),
  (230, 432),
  (231, 433),
  (232, 434),
  (233, 435),
  (234, 436),
  (235, 437),
  (236, 438),
  (237, 439),
  (238, 440),
  (239, 441),
  (240, 442),
  (241, 443),
  (242, 444),
  (243, 445),
  (244, 446),
  (245, 447),
  (246, 448),
  (247, 449),
  (248, 450),
  (249, 451),
  (250, 452),
  (251, 453),
  (252, 454),
  (253, 455),
  (254, 456),
  (255, 432),
  (256, 433),
  (257, 434),
  (258, 435),
  (259, 436),
  (260, 437),
  (261, 438),
  (262, 439),
  (263, 440),
  (264, 441),
  (391, 150),
  (392, 151),
  (393, 152),
  (394, 153),
  (395, 154),
  (396, 155),
  (397, 156),
  (398, 150),
  (399, 151),
  (400, 152),
  (401, 153),
  (402, 154),
  (403, 155),
  (404, 156),
  (405, 150),
  (406, 151),
  (407, 152),
  (408, 153),
  (409, 154),
  (410, 155),
  (411, 156),
  (412, 150),
  (413, 151),
  (414, 152),
  (415, 153),
  (416, 154),
  (417, 155),
  (418, 156),
  (419, 260),
  (420, 261),
  (421, 262),
  (422, 263),
  (423, 264),
  (424, 265),
  (425, 266),
  (426, 267),
  (427, 268),
  (428, 269),
  (1204, 177),
  (1205, 178),
  (1206, 179),
  (1207, 180),
  (1208, 181),
  (1209, 182),
  (1210, 183),
  (1211, 184),
  (1212, 185),
  (1213, 270),
  (1214, 271),
  (1215, 272),
  (1216, 273),
  (1217, 274),
  (1218, 275),
  (1220, 457),
  (1221, 458),
  (4985, 98),
  (4986, 99),
  (4987, 93),
  (4988, 95),
  (4989, 96),
  (4990, 67),
  (4991, 68),
  (4994, 69),
  (4995, 74),
  (4996, 75),
  (4997, 80),
  (4998, 81),
  (4999, 83),
  (5000, 84),
  (5001, 86),
  (5002, 87),
  (5003, 89),
  (5004, 90),
  (5005, 92),
  (6001, 70),
  (6002, 71),
  (6003, 72),
  (6004, 73),
  (6005, 76),
  (6006, 77),
  (6007, 78),
  (6008, 79),
  (6009, 82),
  (6010, 85),
  (6011, 88),
  (6012, 91),
  (6013, 94),
  (6014, 97),
  (6015, 100),
  (6016, 101),
  (6017, 102),
  (6018, 103),
  (6019, 104),
  (6020, 105),
  (6021, 106),
  (6022, 107),
  (6023, 108),
  (6024, 109),
  (6025, 460),
  (6026, 461),
  (6027, 462),
  (6028, 463),
  (6029, 464),
  (6030, 465),
  (6031, 466),
  (6032, 467),
  (6033, 468),
  (6034, 469),
  (6035, 470),
  (6036, 471),
  (6037, 472),
  (6038, 473),
  (6039, 474),
  (6040, 475),
  (6041, 476),
  (6042, 477),
  (6043, 478),
  (6044, 479),
  (6045, 480),
  (6046, 481),
  (6047, 482),
  (6048, 483),
  (6049, 484),
  (-19073, 1),
  (-31402, 17)
ON CONFLICT (n_index) DO UPDATE SET mix_index = EXCLUDED.mix_index;

COMMIT;
