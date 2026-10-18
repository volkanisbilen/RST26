-- Which ACHIEVE_TITLE row the player picked for the skill title (so bonuses survive relog)
ALTER TABLE user_achieve_summary ADD COLUMN IF NOT EXISTS skill_title_row SMALLINT NOT NULL DEFAULT 0;
