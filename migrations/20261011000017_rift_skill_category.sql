-- Full Moon Rift: the client only lets skills of category 3500 onto the skill bar inside zone 89.
UPDATE magic SET skill = 3500 WHERE magic_num IN (490336, 490341, 490342, 490343);
