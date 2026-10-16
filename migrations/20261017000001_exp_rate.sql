-- Global monster EXP rate knob (percent, 100 = unchanged). Applied once, before any of the
-- event/premium/clan/item/burning % bonuses in level.rs::apply_exp_bonuses, so those still
-- stack multiplicatively on top of the scaled base rather than being overridden by it.
ALTER TABLE server_settings ADD COLUMN IF NOT EXISTS exp_rate SMALLINT NOT NULL DEFAULT 100;
