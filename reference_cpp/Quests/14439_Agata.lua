-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14439;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 973, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 973, NPC);
	else
		EVENT = QuestNum
	end
end   

local savenum = 274;

if (EVENT == 111) then -- Paramun
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 129, NPC, 22, 112, 23, 113);
	else
		SelectMsg(UID, 2, savenum, 129, NPC, 10, -1);
	end
end

if (EVENT == 112) then
	SaveEvent(UID, 28);
end

if (EVENT == 113) then
	SaveEvent(UID, 31);
end

if (EVENT == 115) then
	SaveEvent(UID, 30);
end

if (EVENT == 116) then
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 120, NPC, 18, 117);
	else
		SelectMsg(UID, 4, savenum, 120, NPC, 41, 118, 23, 117);
	end
end

if (EVENT == 117) then
	ShowMap(UID, 79);
end

if (EVENT == 118) then
	QuestStatusCheck = GetQuestStatus(UID, 274) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 120, NPC, 18, 117);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 18);
		SaveEvent(UID, 29);
	else
		RunQuestExchange(UID, 18);
		SaveEvent(UID, 29);
	end
end
end
end

local savenum = 275;

if (EVENT == 120) then -- Brahman
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 142, NPC, 22, 121, 23, 122);
	else
		SelectMsg(UID, 2, savenum, 142, NPC, 10, -1);
	end
end

if (EVENT == 121) then
	SaveEvent(UID, 38);
end

if (EVENT == 122) then
	SaveEvent(UID, 41);
end

if (EVENT == 124) then
	SaveEvent(UID, 40);
end

if (EVENT == 125) then
	MonsterCount = CountMonsterQuestSub(UID, 275, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 142, NPC, 18, 126);
	else
		SelectMsg(UID, 4, savenum, 142, NPC, 41, 127, 23, 126);
	end
end

if (EVENT == 126) then
	ShowMap(UID, 606);
end

if (EVENT == 127) then
	QuestStatusCheck = GetQuestStatus(UID, 275) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 275, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 142, NPC, 18, 126);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 19)
		SaveEvent(UID, 39);
	else
		RunQuestExchange(UID, 19)
		SaveEvent(UID, 39);
	end
end
end
end

local savenum = 276;

if (EVENT == 130) then -- Troll Shaman
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 152, NPC, 22, 131, 23, 132);
	else
		SelectMsg(UID, 2, savenum, 152, NPC, 10, -1);
	end
end

if (EVENT == 131) then
	SaveEvent(UID, 79);
end

if (EVENT == 132) then
	SaveEvent(UID, 82);
end

if (EVENT == 134) then
	SaveEvent(UID, 81);
end

if (EVENT == 135) then
	MonsterCount = CountMonsterQuestSub(UID, 276, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 152, NPC, 18, 136);
	else
		SelectMsg(UID, 4, savenum, 152, NPC, 41, 137, 23, 136);
	end
end

if (EVENT == 136) then
	ShowMap(UID, 85);
end

if (EVENT == 137) then
	QuestStatusCheck = GetQuestStatus(UID, 276) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 276, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 152, NPC, 18, 136);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 20)
		SaveEvent(UID, 80);
	else
		RunQuestExchange(UID, 20)
		SaveEvent(UID, 80);
	end
end
end
end

local savenum = 277;

if (EVENT == 140) then -- Apostle of Piercing Cold
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 171, NPC, 22, 141, 23, 142);
	else
		SelectMsg(UID, 2, savenum, 171, NPC, 10, -1);
	end
end

if (EVENT == 141) then
	SaveEvent(UID, 89);
end

if (EVENT == 142) then
	SaveEvent(UID, 106);
end

if (EVENT == 144) then
	SaveEvent(UID, 105);
end

if (EVENT == 145) then
	MonsterCount = CountMonsterQuestSub(UID, 277, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 171, NPC, 18, 146);
	else
		SelectMsg(UID, 4, savenum, 171, NPC, 41, 147, 23, 146);
	end
end

if (EVENT == 146) then
	ShowMap(UID, 615);
end

if (EVENT == 147) then
	QuestStatusCheck = GetQuestStatus(UID, 277) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 277, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 171, NPC, 18, 146);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 21)
		SaveEvent(UID, 90);
	else
		RunQuestExchange(UID, 21)
		SaveEvent(UID, 90);
	end
end
end
end

local savenum = 985;

if (EVENT == 150) then -- Apostle of Flame
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 194, NPC, 22, 151, 23, 152);
	else
		SelectMsg(UID, 2, savenum, 194, NPC, 10, -1);
	end
end

if (EVENT == 151) then
	SaveEvent(UID, 255);
end

if (EVENT == 152) then
	SaveEvent(UID, 258);
end

if (EVENT == 154) then
	SaveEvent(UID, 257);
end

if (EVENT == 155) then
	MonsterCount = CountMonsterQuestSub(UID, 985, 1);
	if (MonsterCount < 120) then
		SelectMsg(UID, 2, savenum, 142, NPC, 18, 156);
	else
		SelectMsg(UID, 4, savenum, 142, NPC, 41, 157, 23, 156);
	end
end

if (EVENT == 156) then
	ShowMap(UID, 617);
end

if (EVENT == 157) then
	QuestStatusCheck = GetQuestStatus(UID, 985) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 985, 1);
	if (MonsterCount < 120) then
		SelectMsg(UID, 2, savenum, 142, NPC, 18, 156);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		ExpChange(UID, 1400000)
		SaveEvent(UID, 256);
	else
		ExpChange(UID, 600000)
		SaveEvent(UID, 256);
	end
end
end
end

local savenum = 1015;

if (EVENT == 170) then -- Troll Berserker
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 971, NPC, 22, 171, 23, 172);
	else
		SelectMsg(UID, 2, savenum, 971, NPC, 10, -1);
	end
end

if (EVENT == 171) then
	SaveEvent(UID, 712);
end

if (EVENT == 172) then
	SaveEvent(UID, 715);
end

if (EVENT == 174) then
	SaveEvent(UID, 714);
end

if (EVENT == 175) then
	MonsterCount = CountMonsterQuestSub(UID, 1015, 1);
	if (MonsterCount < 100) then
		SelectMsg(UID, 2, savenum, 142, NPC, 18, 176);
	else
		SelectMsg(UID, 4, savenum, 142, NPC, 41, 177, 23, 176);
	end
end

if (EVENT == 176) then
	ShowMap(UID, 131);
end

if (EVENT == 177) then
	QuestStatusCheck = GetQuestStatus(UID, 1015) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 1015, 1);
	if (MonsterCount < 100) then
		SelectMsg(UID, 2, savenum, 142, NPC, 18, 176);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		ExpChange(UID, 1400000)
		SaveEvent(UID, 713);
	else
		ExpChange(UID, 600000)
		SaveEvent(UID, 713);
	end
end
end
end

local savenum = 278;

if (EVENT == 160) then -- Troll Warrior
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 964, NPC, 22, 161, 23, 162);
	else
		SelectMsg(UID, 2, savenum, 964, NPC, 10, -1);
	end
end

if (EVENT == 161) then
	SaveEvent(UID, 7742);
end

if (EVENT == 162) then
	SaveEvent(UID, 7748);
end

if (EVENT == 164) then
	SaveEvent(UID, 7747);
end

if (EVENT == 165) then
	MonsterCount = CountMonsterQuestSub(UID, 278, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 964, NPC, 18, 166);
	else
		SelectMsg(UID, 4, savenum, 964, NPC, 41, 167, 23, 166);
	end
end

if (EVENT == 166) then
	ShowMap(UID, 129);
end

if (EVENT == 167) then
	QuestStatusCheck = GetQuestStatus(UID, 278) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 278, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 964, NPC, 18, 166);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 110)
		SaveEvent(UID, 7743);
	else
		RunQuestExchange(UID, 110)
		SaveEvent(UID, 7743);
	end
end
end
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 523, 20015, NPC, 22, 1103, 27, -1);
end

if (EVENT == 1103) then
	SaveEvent(UID, 11122);
end

if (EVENT == 1104) then
		SelectMsg(UID, 4, 523, 20177, NPC, 22, 1105, 27, -1);
		SaveEvent(UID, 11124);
end

if (EVENT == 1105) then
	SaveEvent(UID, 11123);
	SaveEvent(UID, 11134);
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 524, 20017, NPC, 22, 1203, 27, -1);
end

if (EVENT == 1203) then
	SaveEvent(UID, 11134);
end

if (EVENT == 1206) then
	SaveEvent(UID, 11136);
end

if (EVENT == 1205) then
	MonsterCount = CountMonsterQuestSub(UID, 524, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 524, 20017, NPC, 18, 1208);
	else
		SelectMsg(UID, 5, 524, 20017, NPC, 41, 1207,23, -1);
	end
end

if (EVENT == 1208) then
	ShowMap(UID, 1181);
end

if (EVENT == 1207)then
	QuestStatusCheck = GetQuestStatus(UID, 524) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 524, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 524, 20017, NPC, 18, 1208);
	else
RunQuestExchange(UID,3011,STEP,1);
	SaveEvent(UID, 11135);
		SaveEvent(UID, 11146);
end
end
end

if (EVENT == 1302) then
	SelectMsg(UID, 4, 529, 20027, NPC, 22, 1303, 27, -1);
end

if (EVENT == 1303) then
	SaveEvent(UID, 11194);
end

if (EVENT == 1308) then
	SaveEvent(UID, 11196);
end

if (EVENT == 1305) then
	ITEM_COUNT = HowmuchItem(UID, 910215000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 529, 20027, NPC, 18,-1);
	else
		SelectMsg(UID, 5, 529, 20027, NPC, 22, 1307,27, -1); 
	end
end

if (EVENT == 1307)then
	QuestStatusCheck = GetQuestStatus(UID, 529) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910215000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 529, 20027, NPC, 18,-1);
	else
RunQuestExchange(UID,3016)
	SaveEvent(UID,11195)
	SaveEvent(UID,11206)
end
end
end

if (EVENT == 1402) then
	SelectMsg(UID, 4, 530, 20029, NPC, 22, 1403, 27, -1);
end

if (EVENT == 1403) then
	SaveEvent(UID, 11206);
end

if (EVENT == 1408) then
	SaveEvent(UID, 11208);
end

if (EVENT == 1405) then
	ITEM_COUNT = HowmuchItem(UID, 508105000);   
	if (ITEM_COUNT < 5) then
		SelectMsg(UID, 2, 530, 20029, NPC, 18,1406);
	else
		SelectMsg(UID, 4, 530, 20029, NPC, 22, 1407,27, -1); 
	end
end

if (EVENT == 1406) then
	ShowMap(UID, 1177);
end

if (EVENT == 1407)then
	QuestStatusCheck = GetQuestStatus(UID, 530) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 508105000);   
	if (ITEM_COUNT < 5) then
		SelectMsg(UID, 2, 530, 20029, NPC, 18,1406);
	else
SelectMsg(UID, 2, 530, 20241, NPC, 10,-1);
RunQuestExchange(UID,3017)
	SaveEvent(UID,11207);
	SaveEvent(UID,11212);
end
end
end

if (EVENT == 1502) then
	SelectMsg(UID, 4, 533, 20032, NPC, 22, 1503, 27, -1);
end

if (EVENT == 1503) then
	SaveEvent(UID, 11242);
end

if (EVENT == 1508) then
	SaveEvent(UID, 11244);
end

if (EVENT == 1505) then
	ITEM_COUNT = HowmuchItem(UID, 910216000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 533, 20032, NPC, 18,-1);
	else
		SelectMsg(UID, 4, 533, 20032, NPC, 22, 1507,27, -1); 
	end
end

if (EVENT == 1507)then
	QuestStatusCheck = GetQuestStatus(UID, 533) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910216000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 533, 20032, NPC, 18,-1);
	else
RunQuestExchange(UID,3020)
	SaveEvent(UID,11243);
	SaveEvent(UID,11254);
end
end
end

if (EVENT == 1602) then
	SelectMsg(UID, 4, 534, 20037, NPC, 22, 1603, 27, -1);
end

if (EVENT == 1603) then
	SaveEvent(UID, 11254);
end

if (EVENT == 1606) then
	SaveEvent(UID, 11256);
end

if (EVENT == 1605) then
	MonsterCount = CountMonsterQuestSub(UID, 534, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 534, 20037, NPC, 18, 1607);
	else
		SelectMsg(UID, 5, 534, 20037, NPC, 41, 1608,23, -1);
	end
end

if (EVENT == 1608)then
	QuestStatusCheck = GetQuestStatus(UID, 534) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 534, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 534, 20037, NPC, 18, 1607);
	else
RunQuestExchange(UID,3021,STEP,1);
SaveEvent(UID, 11255);
SaveEvent(UID, 11266);
end
end
end

if (EVENT == 1705) then
	CHERICHERO = HowmuchItem(UID, 910229000);
	if (CHERICHERO < 1) then
		SelectMsg(UID, 2, 551, 21624, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 551, 20066, NPC, 10, 1709, 27, -1);
	end
end


if (EVENT == 1709) then
	RELICHERO = HowmuchItem(UID, 910229000);
	if (RELICHERO < 1) then
		SelectMsg(UID, 2, 551, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3041);
	SaveEvent(UID, 11489);
	SaveEvent(UID, 11500);
    end
end