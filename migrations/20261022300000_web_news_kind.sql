-- Web panel: "Haberler & Duyurular" and "Güncelleme Notları" are two separate lists on the site.
ALTER TABLE web_news ADD COLUMN IF NOT EXISTS kind varchar(10) NOT NULL DEFAULT 'news';
CREATE INDEX IF NOT EXISTS idx_web_news_kind ON web_news (kind, published, created_at DESC);
-- existing rows that read like release notes go to the update list
UPDATE web_news SET kind = 'update'
 WHERE kind = 'news' AND (title ILIKE '%güncelle%' OR title ILIKE '%yama%' OR title ILIKE '%patch%' OR title ILIKE '%sistem%' OR title ILIKE '%hediye%');
