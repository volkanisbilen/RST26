local Ret = 0;
local NPC = 13013;

if (EVENT == 165) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 166, NPC, 10, 168);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 167, NPC);
	else
		EVENT = QuestNum;
	end
end

if (EVENT == 166) then
	SelectMsg(UID, 2, -1, 166, NPC, 10, 168);
end

if (EVENT == 168) then
	Ret = 1;
end

-- ======= Quest 60 (level 1) =======
-- Quest 60, pre-quest
if (EVENT == 170) then
	SaveEvent(UID, 48);
end
if (EVENT == 172) then SaveEvent(UID, 48); end

-- Quest 60, not started
if (EVENT == 175) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 60, 176, NPC, 22, 176, 23, -1);
	else
		SelectMsg(UID, 2, 60, 176, NPC, 10, -1);
	end
end

if (EVENT == 176) then
	SaveEvent(UID, 49);
end

-- Quest 60, in progress
if (EVENT == 185) then
	MonsterCount = CountMonsterQuestSub(UID, 60, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 60, 176, NPC, 18, 186);
	else
		SelectMsg(UID, 4, 60, 176, NPC, 41, 187, 27, -1);
	end
end

if (EVENT == 186) then
	ShowMap(UID, 1);
end

if (EVENT == 187) then
	QuestStatusCheck = GetQuestStatus(UID, 60);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 176, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 60, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 60, 176, NPC, 18, 186);
	else
	RunQuestExchange(UID, 5);
	SaveEvent(UID, 50);
	end
	end
end

-- Quest 60 complete event
if (EVENT == 180) then
	SaveEvent(UID, 51);
end

-- ======= Quest 62 (level 3) =======
-- Quest 62, pre-quest
if (EVENT == 220) then
	SaveEvent(UID, 63);
end
if (EVENT == 223) then SaveEvent(UID, 63); end

-- Quest 62, not started
if (EVENT == 225) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 62, 228, NPC, 22, 226, 23, -1);
	else
		SelectMsg(UID, 2, 62, 228, NPC, 10, -1);
	end
end

if (EVENT == 226) then
	SaveEvent(UID, 64);
end

-- Quest 62, in progress
if (EVENT == 235) then
	MonsterCount = CountMonsterQuestSub(UID, 62, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 62, 228, NPC, 18, 236);
	else
		SelectMsg(UID, 4, 62, 228, NPC, 41, 237, 27, -1);
	end
end

if (EVENT == 236) then
	ShowMap(UID, 1);
end

if (EVENT == 237) then
	QuestStatusCheck = GetQuestStatus(UID, 62);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 228, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 62, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 62, 228, NPC, 18, 236);
	else
	RunQuestExchange(UID, 7);
	SaveEvent(UID, 65);
	end
	end
end

-- Quest 62 complete event
if (EVENT == 231) then
	SaveEvent(UID, 66);
end

-- ======= Quest 65 (level 6) =======
-- Quest 65, pre-quest
if (EVENT == 370) then
	SaveEvent(UID, 3322);
end
if (EVENT == 372) then SaveEvent(UID, 3322); end

-- Quest 65, not started
if (EVENT == 300) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 65, 3153, NPC, 22, 301, 23, -1);
	else
		SelectMsg(UID, 2, 65, 3153, NPC, 10, -1);
	end
end

if (EVENT == 301) then
	SaveEvent(UID, 3323);
end

-- Quest 65, in progress
if (EVENT == 305) then
	MonsterCount = CountMonsterQuestSub(UID, 65, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 65, 3153, NPC, 18, 306);
	else
		SelectMsg(UID, 5, 65, 3159, NPC, 10, 307, 27, -1);
	end
end

if (EVENT == 306) then
	ShowMap(UID, 1);
end

if (EVENT == 307) then
	QuestStatusCheck = GetQuestStatus(UID, 65);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 3153, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 65, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 65, 3153, NPC, 18, 306);
	else
	RunQuestExchange(UID, 320);
	SaveEvent(UID, 3324);
	end
	end
end

-- Quest 65 complete event
if (EVENT == 303) then
	SaveEvent(UID, 3325);
end

-- ======= Quest 67 (level 8) =======
-- Quest 67, pre-quest
if (EVENT == 470) then
	SaveEvent(UID, 3332);
end

-- Quest 67, not started
if (EVENT == 400) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 67, 3161, NPC, 22, 401, 23, -1);
	else
		SelectMsg(UID, 2, 67, 3161, NPC, 10, -1);
	end
end

if (EVENT == 401) then
	SaveEvent(UID, 3333);
end

-- Quest 67, in progress
if (EVENT == 405) then
	MonsterCount = CountMonsterQuestSub(UID, 67, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 67, 3161, NPC, 18, 406);
	else
		SelectMsg(UID, 5, 67, 3167, NPC, 10, 407, 27, -1);
	end
end

if (EVENT == 406) then
	ShowMap(UID, 1);
end

if (EVENT == 407) then
	QuestStatusCheck = GetQuestStatus(UID, 67);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 3161, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 67, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 67, 3161, NPC, 18, 406);
	else
	RunQuestExchange(UID, 321);
	SaveEvent(UID, 3334);
	end
	end
end

-- Quest 67 complete event
if (EVENT == 403) then
	SaveEvent(UID, 3335);
end

-- ======= Quest 70 (level 9) =======
-- Quest 70, pre-quest
if (EVENT == 570) then
	SaveEvent(UID, 3342);
end

-- Quest 70, not started
if (EVENT == 500) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 70, 3169, NPC, 22, 501, 23, -1);
	else
		SelectMsg(UID, 2, 70, 3169, NPC, 10, -1);
	end
end

if (EVENT == 501) then
	SaveEvent(UID, 3343);
end

-- Quest 70, in progress
if (EVENT == 505) then
	MonsterCount = CountMonsterQuestSub(UID, 70, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 70, 3169, NPC, 18, 506);
	else
		SelectMsg(UID, 5, 70, 3175, NPC, 10, 507, 27, -1);
	end
end

if (EVENT == 506) then
	ShowMap(UID, 1);
end

if (EVENT == 507) then
	QuestStatusCheck = GetQuestStatus(UID, 70);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 3169, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 70, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 70, 3169, NPC, 18, 506);
	else
	RunQuestExchange(UID, 322);
	SaveEvent(UID, 3344);
	end
	end
end

-- Quest 70 complete event
if (EVENT == 503) then
	SaveEvent(UID, 3345);
end

-- ======= Quest 80 (level 10) =======
-- Quest 80, pre-quest
if (EVENT == 670) then
	SaveEvent(UID, 3352);
end
if (EVENT == 672) then SaveEvent(UID, 3352); end

-- Quest 80, not started
if (EVENT == 600) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 80, 3194, NPC, 22, 601, 23, -1);
	else
		SelectMsg(UID, 2, 80, 3194, NPC, 10, -1);
	end
end

if (EVENT == 601) then
	SaveEvent(UID, 3353);
end

-- Quest 80, in progress
if (EVENT == 605) then
	MonsterCount = CountMonsterQuestSub(UID, 80, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 80, 3194, NPC, 18, 606);
	else
		SelectMsg(UID, 5, 80, 3189, NPC, 10, 607, 27, -1);
	end
end

if (EVENT == 606) then
	ShowMap(UID, 1);
end

if (EVENT == 607) then
	QuestStatusCheck = GetQuestStatus(UID, 80);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 3194, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 80, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 80, 3194, NPC, 18, 606);
	else
	RunQuestExchange(UID, 323);
	SaveEvent(UID, 3354);
	end
	end
end

-- Quest 80 complete event
if (EVENT == 603) then
	SaveEvent(UID, 3355);
end

-- ======= Quest 83 (level 11) =======
-- Quest 83, pre-quest
if (EVENT == 770) then
	SaveEvent(UID, 3362);
end
if (EVENT == 772) then SaveEvent(UID, 3362); end

-- Quest 83, not started
if (EVENT == 700) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 83, 3801, NPC, 22, 701, 23, -1);
	else
		SelectMsg(UID, 2, 83, 3801, NPC, 10, -1);
	end
end

if (EVENT == 701) then
	SaveEvent(UID, 3363);
end

-- Quest 83, in progress
if (EVENT == 705) then
	MonsterCount = CountMonsterQuestSub(UID, 83, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 83, 3801, NPC, 18, 706);
	else
		SelectMsg(UID, 5, 83, 3807, NPC, 10, 707, 27, -1);
	end
end

if (EVENT == 706) then
	ShowMap(UID, 1);
end

if (EVENT == 707) then
	QuestStatusCheck = GetQuestStatus(UID, 83);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 3801, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 83, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 83, 3801, NPC, 18, 706);
	else
	RunQuestExchange(UID, 324);
	SaveEvent(UID, 3364);
	end
	end
end

-- Quest 83 complete event
if (EVENT == 703) then
	SaveEvent(UID, 3365);
end

-- ======= Quest 85 (level 13) =======
-- Quest 85, pre-quest
if (EVENT == 9200) then
	SaveEvent(UID, 5274);
end
if (EVENT == 9202) then SaveEvent(UID, 5274); end

-- Quest 85, not started
if (EVENT == 9205) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 85, 8667, NPC, 22, 9206, 23, -1);
	else
		SelectMsg(UID, 2, 85, 8667, NPC, 10, -1);
	end
end

if (EVENT == 9206) then
	SaveEvent(UID, 5275);
end

-- Quest 85, in progress
if (EVENT == 9215) then
	MonsterCount = CountMonsterQuestSub(UID, 85, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 85, 8667, NPC, 18, 9216);
	else
		SelectMsg(UID, 5, 85, 8667, NPC, 10, 9217, 27, -1);
	end
end

if (EVENT == 9216) then
	ShowMap(UID, 1);
end

if (EVENT == 9217) then
	QuestStatusCheck = GetQuestStatus(UID, 85);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8667, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 85, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 85, 8667, NPC, 18, 9216);
	else
	RunQuestExchange(UID, 1080);
	SaveEvent(UID, 5276);
	end
	end
end

-- Quest 85, completable
if (EVENT == 190) then
	QuestStatusCheck = GetQuestStatus(UID, 85);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8667, NPC, 10, -1);
	else
		SelectMsg(UID, 5, 85, 8667, NPC, 10, 9217, 27, -1);
	end
end

-- Quest 85 complete event
if (EVENT == 9210) then
	SaveEvent(UID, 5277);
end

-- ======= Quest 86 (level 14) =======
-- Quest 86, pre-quest
if (EVENT == 870) then
	SaveEvent(UID, 3372);
end

-- Quest 86, not started
if (EVENT == 800) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 86, 3809, NPC, 22, 801, 23, -1);
	else
		SelectMsg(UID, 2, 86, 3809, NPC, 10, -1);
	end
end

if (EVENT == 801) then
	SaveEvent(UID, 3373);
end

-- Quest 86, in progress
if (EVENT == 805) then
	MonsterCount = CountMonsterQuestSub(UID, 86, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 86, 3809, NPC, 18, 806);
	else
		SelectMsg(UID, 5, 86, 3815, NPC, 10, 807, 27, -1);
	end
end

if (EVENT == 806) then
	ShowMap(UID, 1);
end

if (EVENT == 807) then
	QuestStatusCheck = GetQuestStatus(UID, 86);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 3809, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 86, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 86, 3809, NPC, 18, 806);
	else
	RunQuestExchange(UID, 325);
	SaveEvent(UID, 3374);
	end
	end
end

-- Quest 86 complete event
if (EVENT == 803) then
	SaveEvent(UID, 3375);
end

-- ======= Quest 87 (level 15) =======
-- Quest 87, pre-quest
if (EVENT == 9220) then
	SaveEvent(UID, 5281);
end
if (EVENT == 9222) then SaveEvent(UID, 5281); end

-- Quest 87, not started
if (EVENT == 9225) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 87, 8668, NPC, 22, 9226, 23, -1);
	else
		SelectMsg(UID, 2, 87, 8668, NPC, 10, -1);
	end
end

if (EVENT == 9226) then
	SaveEvent(UID, 5282);
end

-- Quest 87, in progress
if (EVENT == 9235) then
	MonsterCount = CountMonsterQuestSub(UID, 87, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 87, 8668, NPC, 18, 9236);
	else
		SelectMsg(UID, 5, 87, 3175, NPC, 10, 9237, 27, -1);
	end
end

if (EVENT == 9236) then
	ShowMap(UID, 1);
end

if (EVENT == 9237) then
	QuestStatusCheck = GetQuestStatus(UID, 87);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8668, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 87, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 87, 8668, NPC, 18, 9236);
	else
	RunQuestExchange(UID, 1081);
	SaveEvent(UID, 5283);
	end
	end
end

-- Quest 87 complete event
if (EVENT == 9230) then
	SaveEvent(UID, 5284);
end

-- ======= Quest 90 (level 16) =======
-- Quest 90, pre-quest
if (EVENT == 1070) then
	SaveEvent(UID, 3392);
end
if (EVENT == 1072) then SaveEvent(UID, 3392); end

-- Quest 90, not started
if (EVENT == 1000) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 90, 3836, NPC, 22, 1001, 23, -1);
	else
		SelectMsg(UID, 2, 90, 3836, NPC, 10, -1);
	end
end

if (EVENT == 1001) then
	SaveEvent(UID, 3393);
end

-- Quest 90, in progress
if (EVENT == 1005) then
	MonsterCount = CountMonsterQuestSub(UID, 90, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 90, 3836, NPC, 18, 1006);
	else
		SelectMsg(UID, 5, 90, 3843, NPC, 10, 1007, 27, -1);
	end
end

if (EVENT == 1006) then
	ShowMap(UID, 1);
end

if (EVENT == 1007) then
	QuestStatusCheck = GetQuestStatus(UID, 90);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 3836, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 90, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 90, 3836, NPC, 18, 1006);
	else
	RunQuestExchange(UID, 327);
	SaveEvent(UID, 3394);
	end
	end
end

-- Quest 90 complete event
if (EVENT == 1003) then
	SaveEvent(UID, 3395);
end

-- ======= Quest 92 (level 17) =======
-- Quest 92, pre-quest
if (EVENT == 970) then
	SaveEvent(UID, 3382);
end

-- Quest 92, not started
if (EVENT == 900) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 92, 3818, NPC, 22, 901, 23, -1);
	else
		SelectMsg(UID, 2, 92, 3818, NPC, 10, -1);
	end
end

if (EVENT == 901) then
	SaveEvent(UID, 3383);
end

-- Quest 92, in progress
if (EVENT == 905) then
	MonsterCount = CountMonsterQuestSub(UID, 92, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 92, 3818, NPC, 18, 906);
	else
		SelectMsg(UID, 5, 92, 3823, NPC, 10, 907, 27, -1);
	end
end

if (EVENT == 906) then
	ShowMap(UID, 1);
end

if (EVENT == 907) then
	QuestStatusCheck = GetQuestStatus(UID, 92);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 3818, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 92, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 92, 3818, NPC, 18, 906);
	else
	RunQuestExchange(UID, 326);
	SaveEvent(UID, 3384);
	end
	end
end

-- Quest 92 complete event
if (EVENT == 903) then
	SaveEvent(UID, 3385);
end

-- ======= Quest 96 (level 19) =======
-- Quest 96, pre-quest
if (EVENT == 9240) then
	SaveEvent(UID, 5288);
end
if (EVENT == 9242) then SaveEvent(UID, 5288); end

-- Quest 96, not started
if (EVENT == 9245) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 96, 8669, NPC, 22, 9246, 23, -1);
	else
		SelectMsg(UID, 2, 96, 8669, NPC, 10, -1);
	end
end

if (EVENT == 9246) then
	SaveEvent(UID, 5289);
end

-- Quest 96, in progress
if (EVENT == 9255) then
	MonsterCount = CountMonsterQuestSub(UID, 96, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 96, 8669, NPC, 18, 9256);
	else
		SelectMsg(UID, 4, 96, 8669, NPC, 41, 9257, 27, -1);
	end
end

if (EVENT == 9256) then
	ShowMap(UID, 1);
end

if (EVENT == 9257) then
	QuestStatusCheck = GetQuestStatus(UID, 96);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8669, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 96, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 96, 8669, NPC, 18, 9256);
	else
	RunQuestExchange(UID, 1082);
	SaveEvent(UID, 5290);
	end
	end
end

-- Quest 96 complete event
if (EVENT == 9250) then
	SaveEvent(UID, 5291);
end

-- ======= Quest 97 (level 20) =======
-- Quest 97, pre-quest
if (EVENT == 9431) then
	SelectMsg(UID, 2, 97, 3798, NPC, 10, 9432);
end

if (EVENT == 9432) then
	SaveEvent(UID, 5295);
end

if (EVENT == 9433) then
	SelectMsg(UID, 2, 97, 3799, NPC, 10, 9432);
end

-- Quest 97, not started
if (EVENT == 9434) then
	SelectMsg(UID, 2, 97, 8671, NPC, 10, 9435);
end

if (EVENT == 9435) then
	SelectMsg(UID, 4, 97, 8672, NPC, 22, 9436, 23, -1);
end

if (EVENT == 9436) then
	SelectMsg(UID, 2, 97, 3802, NPC, 14, -1);
	SaveEvent(UID, 5296);
end

-- Quest 97, in progress
if (EVENT == 9439) then
	ItemCount = HowmuchItem(UID, 810418000);
	if (ItemCount < 5) then
		SelectMsg(UID, 2, 97, 8671, NPC, 10, 9440);
	else
		SelectMsg(UID, 4, 97, 8672, NPC, 10, 9441, 27, 9440);
	end
end

if (EVENT == 9440) then
	ShowMap(UID, 523);
end

if (EVENT == 9441) then
	QuestStatusCheck = GetQuestStatus(UID, 97);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		ItemCount = HowmuchItem(UID, 810418000);
		if (ItemCount < 5) then
			SelectMsg(UID, 2, 97, 8671, NPC, 10, 9440);
		else
			if (RunQuestExchange(UID, 1083)) then
				SaveEvent(UID, 5297);
			end
		end
	end
end

-- Quest 97 complete event
if (EVENT == 9437) then
	SaveEvent(UID, 5298);
	Nation = CheckNation(UID);
	if (Nation == 1) then
		SelectMsg(UID, 2, 97, 8671, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 97, 8672, NPC, 14, -1);
	end
end

-- ======= Quest 98 (level 20) =======
-- Quest 98, pre-quest
if (EVENT == 9260) then
	SaveEvent(UID, 5302);
end
if (EVENT == 9262) then SaveEvent(UID, 5302); end

-- Quest 98, not started
if (EVENT == 9265) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 98, 8671, NPC, 22, 9266, 23, -1);
	else
		SelectMsg(UID, 2, 98, 8671, NPC, 10, -1);
	end
end

if (EVENT == 9266) then
	SaveEvent(UID, 5303);
end

-- Quest 98, in progress
if (EVENT == 9275) then
	MonsterCount = CountMonsterQuestSub(UID, 98, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 98, 8671, NPC, 18, 9276);
	else
		SelectMsg(UID, 4, 98, 8671, NPC, 41, 9277, 27, -1);
	end
end

if (EVENT == 9276) then
	ShowMap(UID, 1);
end

if (EVENT == 9277) then
	QuestStatusCheck = GetQuestStatus(UID, 98);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8671, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 98, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 98, 8671, NPC, 18, 9276);
	else
	RunQuestExchange(UID, 1084);
	SaveEvent(UID, 5304);
	end
	end
end

-- Quest 98 complete event
if (EVENT == 9270) then
	SaveEvent(UID, 5305);
end

-- ======= Quest 99 (level 22) =======
-- Quest 99, pre-quest
if (EVENT == 9280) then
	SaveEvent(UID, 5309);
end
if (EVENT == 9282) then SaveEvent(UID, 5309); end

-- Quest 99, not started
if (EVENT == 9285) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 99, 8672, NPC, 22, 9286, 23, -1);
	else
		SelectMsg(UID, 2, 99, 8672, NPC, 10, -1);
	end
end

if (EVENT == 9286) then
	SaveEvent(UID, 5310);
end

-- Quest 99, in progress
if (EVENT == 9295) then
	MonsterCount = CountMonsterQuestSub(UID, 99, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 99, 8672, NPC, 18, 9296);
	else
		SelectMsg(UID, 5, 99, 3815, NPC, 10, 9297, 27, -1);
	end
end

if (EVENT == 9296) then
	ShowMap(UID, 1);
end

if (EVENT == 9297) then
	QuestStatusCheck = GetQuestStatus(UID, 99);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8672, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 99, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 99, 8672, NPC, 18, 9296);
	else
	RunQuestExchange(UID, 1085);
	SaveEvent(UID, 5311);
	end
	end
end

-- Quest 99 complete event
if (EVENT == 9290) then
	SaveEvent(UID, 5312);
end

-- ======= Quest 111 (level 23) =======
-- Quest 111, not started
if (EVENT == 8632) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 111, 8090, NPC, 22, 8633, 23, -1);
	else
		SelectMsg(UID, 2, 111, 8090, NPC, 10, -1);
	end
end

if (EVENT == 8633) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then -- Warrior
		SaveEvent(UID, 8146);
	elseif (Class == 2 or Class == 7 or Class == 8) then -- Rogue
		SaveEvent(UID, 8151);
	elseif (Class == 3 or Class == 9 or Class == 10) then -- Mage
		SaveEvent(UID, 8156);
	elseif (Class == 4 or Class == 11 or Class == 12) then -- Priest
		SaveEvent(UID, 8161);
	end
end

-- Quest 111, in progress
if (EVENT == 8636) then
	MonsterCount = CountMonsterQuestSub(UID, 111, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 111, 8090, NPC, 18, 8637);
	else
		SelectMsg(UID, 5, 111, 8090, NPC, 41, 8638, 23, -1);
	end
end

if (EVENT == 8637) then
	ShowMap(UID, 1);
end

if (EVENT == 8638) then
	QuestStatusCheck = GetQuestStatus(UID, 111);
	if (QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8090, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 111, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 111, 8090, NPC, 18, 8637);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then -- Warrior
		RunQuestExchange(UID, 840);
		SaveEvent(UID, 8147);
	elseif (Class == 2 or Class == 7 or Class == 8) then -- Rogue
		RunQuestExchange(UID, 841);
		SaveEvent(UID, 8152);
	elseif (Class == 3 or Class == 9 or Class == 10) then -- Mage
		RunQuestExchange(UID, 842);
		SaveEvent(UID, 8157);
	elseif (Class == 4 or Class == 11 or Class == 12) then -- Priest
		RunQuestExchange(UID, 843);
		SaveEvent(UID, 8162);
	end
	end
	end
end

-- Quest 111 complete event
if (EVENT == 8640) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then -- Warrior
		SaveEvent(UID, 8148);
	elseif (Class == 2 or Class == 7 or Class == 8) then -- Rogue
		SaveEvent(UID, 8153);
	elseif (Class == 3 or Class == 9 or Class == 10) then -- Mage
		SaveEvent(UID, 8158);
	elseif (Class == 4 or Class == 11 or Class == 12) then -- Priest
		SaveEvent(UID, 8163);
	end
end

-- ======= Quest 1204 (level 4) =======
-- Quest 1204, not started
if (EVENT == 1102) then
	SelectMsg(UID, 2, 1204, 43631, NPC, 22, 1103, 23, -1);
end

if (EVENT == 1103) then
	SaveEvent(UID, 7335);
end

-- Quest 1204, in progress
if (EVENT == 1105) then
	SelectMsg(UID, 2, 1204, 43631, NPC, 10, -1);
end

-- Quest 1204 complete event
if (EVENT == 1106) then
	SaveEvent(UID, 7337);
end

if (EVENT == 3001) then
	Ret = 1;
end
