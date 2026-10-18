-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14426;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	SaveEvent(UID, 741);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4605, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 4606, NPC)
	else
		EVENT = QuestNum
	end
end


local savenum = 280;

if (EVENT == 170) then -- 60 Level Darkness Knight
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 244, NPC, 22, 171, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 244, NPC, 10, -1);
	end
end

if (EVENT == 171) then
	SaveEvent(UID, 209);
end

if (EVENT == 172) then
	SaveEvent(UID, 212);
end

if (EVENT == 174) then
	SaveEvent(UID, 211);
	SelectMsg(UID, 2, savenum, 244, NPC, 14, -1);
end

if (EVENT == 175) then
	MonsterCount = CountMonsterQuestSub(UID, 280, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 244, NPC, 18, 177);
	else
		SelectMsg(UID, 4, savenum, 244, NPC, 41, 176, 27, -1);
	end
end

if (EVENT == 177) then
	ShowMap(UID, 80);
end

if (EVENT == 176) then
	MonsterCount = CountMonsterQuestSub(UID, 280, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 244, NPC, 18, 177);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 13)
		SaveEvent(UID, 210);  
	else
		RunQuestExchange(UID, 13)
		SaveEvent(UID, 210); 
	end   
end
end

local savenum = 281;

if (EVENT == 180) then -- 60 Level Troll Shaman
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 261, NPC, 22, 181, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 261, NPC, 10, -1);
	end
end

if (EVENT == 181) then
	SaveEvent(UID, 219);
end

if (EVENT == 182) then
	SaveEvent(UID, 222);
end

if (EVENT == 184) then
	SaveEvent(UID, 221);
	SelectMsg(UID, 2, savenum, 224, NPC, 14, -1);
end

if (EVENT == 185) then
	MonsterCount = CountMonsterQuestSub(UID, 281, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 244, NPC, 18, 187);
	else
		SelectMsg(UID, 4, savenum, 244, NPC, 41, 186, 27, -1);
	end
end

if (EVENT == 187) then
	ShowMap(UID, 86);
end

if (EVENT == 186) then
	MonsterCount = CountMonsterQuestSub(UID, 281, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 244, NPC, 18, 187);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 14)
		SaveEvent(UID, 220);  
	else
		RunQuestExchange(UID, 14)
		SaveEvent(UID, 220); 
	end   
end
end

local savenum = 282;

if (EVENT == 200) then -- 60 Level Crimson Wing
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 270, NPC, 22, 201, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 270, NPC, 10, -1);
	end
end

if (EVENT == 201) then
	SaveEvent(UID, 229);
end

if (EVENT == 202) then
	SaveEvent(UID, 232);
end

if (EVENT == 204) then
	SaveEvent(UID, 231);
	SelectMsg(UID, 2, savenum, 270, NPC, 14, -1);
end

if (EVENT == 205) then
		MonsterCount01 = CountMonsterQuestSub(UID, 282, 1);
	    MonsterCount02 = CountMonsterQuestSub(UID, 282, 2);
	if (MonsterCount01 < 20 and MonsterCount02 < 20) then
		SelectMsg(UID, 2, savenum, 270, NPC, 18, 207);
	else
		SelectMsg(UID, 4, savenum, 270, NPC, 41, 206, 27, -1);
	end
end

if (EVENT == 207) then
	ShowMap(UID, 83);
end

if (EVENT == 206) then
		MonsterCount01 = CountMonsterQuestSub(UID, 282, 1);
	    MonsterCount02 = CountMonsterQuestSub(UID, 282, 2);
	if (MonsterCount01 < 20 and MonsterCount02 < 20) then
		SelectMsg(UID, 2, savenum, 270, NPC, 18, 207);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 15)
		SaveEvent(UID, 230);  
	else
		RunQuestExchange(UID, 15)
		SaveEvent(UID, 230); 
	end   
end
end

local savenum = 283;

if (EVENT == 210) then -- 60 Level Apostle Piercing
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 280, NPC, 22, 211, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 280, NPC, 10, -1);
	end
end

if (EVENT == 211) then
	SaveEvent(UID, 239);
end

if (EVENT == 212) then
	SaveEvent(UID, 242);
end

if (EVENT == 214) then
	SaveEvent(UID, 241);
	SelectMsg(UID, 2, savenum, 280, NPC, 14, -1);
end

if (EVENT == 215) then
	MonsterCount = CountMonsterQuestSub(UID, 283, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 280, NPC, 18, 217);
	else
		SelectMsg(UID, 4, savenum, 280, NPC, 41, 216, 27, -1);
	end
end

if (EVENT == 217) then
	ShowMap(UID, 87);
end

if (EVENT == 216) then
	MonsterCount = CountMonsterQuestSub(UID, 283, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 280, NPC, 18, 217);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 16)
		SaveEvent(UID, 240);  
	else
		RunQuestExchange(UID, 16)
		SaveEvent(UID, 240); 
	end  
end
end

local savenum = 284;

if (EVENT == 220) then -- 60 Level Apostle Flame
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8650, NPC, 22, 221, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 8650, NPC, 10, -1);
	end
end

if (EVENT == 221) then
	SaveEvent(UID, 249);
end

if (EVENT == 222) then
	SaveEvent(UID, 252);
end

if (EVENT == 224) then
	SaveEvent(UID, 251);
	SelectMsg(UID, 2, savenum, 8650, NPC, 14, -1);
end

if (EVENT == 225) then
	MonsterCount = CountMonsterQuestSub(UID, 284, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 8650, NPC, 18, 227);
	else
		SelectMsg(UID, 4, savenum, 8650, NPC, 41, 226, 27, -1);
	end
end

if (EVENT == 227) then
	ShowMap(UID, 88);
end

if (EVENT == 226) then
	MonsterCount = CountMonsterQuestSub(UID, 284, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 8650, NPC, 18, 227);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 17)
		SaveEvent(UID, 250);  
	else
		RunQuestExchange(UID, 17)
		SaveEvent(UID, 250); 
	end   
end
end

local savenum = 279;

if (EVENT == 160) then -- 60 Level Doom Soldier
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 224, NPC, 22, 161, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 224, NPC, 10, -1);
	end
end

if (EVENT == 161) then
	SaveEvent(UID, 265);
end

if (EVENT == 163) then
	SaveEvent(UID, 268);
end

if (EVENT == 164) then
	SaveEvent(UID, 267);
	SelectMsg(UID, 2, savenum, 224, NPC, 14, -1);
end

if (EVENT == 165) then
	MonsterCount = CountMonsterQuestSub(UID, 279, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 224, NPC, 18, 167);
	else
		SelectMsg(UID, 4, savenum, 224, NPC, 41, 166, 27, -1);
	end
end

if (EVENT == 167) then
	ShowMap(UID, 81);
end

if (EVENT == 166) then
	MonsterCount = CountMonsterQuestSub(UID, 279, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 224, NPC, 18, 167);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 12)
		SaveEvent(UID, 266);
	else
		RunQuestExchange(UID, 12)
		SaveEvent(UID, 266); 
	end   
end
end

local savenum = 341;

if (EVENT == 530) then -- 70 Level 100 Kill
	SaveEvent(UID, 832);
	SelectMsg(UID, 2, savenum, 724, NPC, 4080, -1);
end

if (EVENT == 532) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 724, NPC, 22, 533, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 724, NPC, 10, -1);
	end
end

if (EVENT == 533) then
	SaveEvent(UID, 833);
end

if (EVENT == 534) then
	SaveEvent(UID, 836);
end

if (EVENT == 535) then
	SaveEvent(UID, 835);
	SelectMsg(UID, 2, savenum, 724, NPC, 14, -1);
end

if (EVENT == 536) then
	MonsterCount = CountMonsterQuestSub(UID, 341, 1);
	if (MonsterCount < 100) then
		SelectMsg(UID, 2, savenum, 724, NPC, 18, 538);
	else
		SelectMsg(UID, 4, savenum, 724, NPC, 4172, 537, 4173, -1);
	end
end

if (EVENT == 538) then
	ShowMap(UID, 488);
end

if (EVENT == 537) then
	MonsterCount = CountMonsterQuestSub(UID, 341, 1);
	if (MonsterCount < 100) then
		SelectMsg(UID, 2, savenum, 724, NPC, 18, 538);
	else
	RunQuestExchange(UID,134)
	SaveEvent(UID, 834);   
end
end

local savenum = 342;

if (EVENT == 320) then -- 70 Level Ewil Wizard
	SaveEvent(UID, 911);
end

if (EVENT == 321) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 391, NPC, 22, 322, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 391, NPC, 10, -1);
	end
end

if (EVENT == 322) then
	SaveEvent(UID, 912);
end

if (EVENT == 323) then
	SaveEvent(UID, 915);
end

if (EVENT == 324) then
	SaveEvent(UID, 914);
	SelectMsg(UID, 2, savenum, 391, NPC, 14, -1);
end

if (EVENT == 325) then
	MonsterCount = CountMonsterQuestSub(UID, 342, 1);
	if (MonsterCount < 250) then
		SelectMsg(UID, 2, savenum, 391, NPC, 18, 327);
	else
		SelectMsg(UID, 4, savenum, 391, NPC, 41, 326, 27, -1);
	end
end

if (EVENT == 327) then
	ShowMap(UID, 169);
end

if (EVENT == 326) then
	MonsterCount = CountMonsterQuestSub(UID, 342, 1);
	if (MonsterCount < 250) then
		SelectMsg(UID, 2, savenum, 391, NPC, 18, 327);
	else
	RunQuestExchange(UID,157)
	SaveEvent(UID, 913);   
end
end

local savenum = 356;

if (EVENT == 310) then -- 74 Level Keramash & Manicmash & Nigmash
	SaveEvent(UID, 899);
end

if (EVENT == 311) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 371, NPC, 22, 312, 23, 313);
	else
		SelectMsg(UID, 2, savenum, 371, NPC, 10, -1);
	end
end

if (EVENT == 312) then
	SaveEvent(UID, 900);
end

if (EVENT == 323) then
	SaveEvent(UID, 903);
end

if (EVENT == 314) then
	SaveEvent(UID, 902);
	SelectMsg(UID, 2, savenum, 371, NPC, 14, -1);
end


if (EVENT == 315) then
	MonsterCount01 = CountMonsterQuestSub(UID, 356, 1); --kera
	MonsterCount02 = CountMonsterQuestSub(UID, 356, 2); --medik
	MonsterCount03 = CountMonsterQuestSub(UID, 356, 3); --nigmash
	if (MonsterCount01 > 59 and MonsterCount02 > 59 and MonsterCount03 > 79) then 
		SelectMsg(UID, 4, savenum, 371, NPC, 41, 316, 27, -1);
	else
		if     ( MonsterCount01 < 60) then
			SelectMsg(UID, 2, savenum, 4626, NPC, 18, 317);
		elseif ( MonsterCount02 < 60) then
			SelectMsg(UID, 2, savenum, 4627, NPC, 18, 318);
		elseif ( MonsterCount03 < 80) then
			SelectMsg(UID, 2, savenum, 4628, NPC, 18, 319);
		end
	end
end
	

if (EVENT == 317) then
	ShowMap(UID, 168);
end

if (EVENT == 318) then
	ShowMap(UID, 166);
end

if (EVENT == 319) then
	ShowMap(UID, 167);
end

if (EVENT == 316) then
	MonsterCount01 = CountMonsterQuestSub(UID, 356, 1); --kera
	MonsterCount02 = CountMonsterQuestSub(UID, 356, 2); --medik
	MonsterCount03 = CountMonsterQuestSub(UID, 356, 3); --nigmash
	if (MonsterCount01 > 59 and MonsterCount02 > 59 and MonsterCount03 > 79) then 
	RunQuestExchange(UID,156)
	SaveEvent(UID, 901);   
		else
		if ( MonsterCount01 < 60) then
			SelectMsg(UID, 2, savenum, 4626, NPC, 18, 317);
		elseif ( MonsterCount02 < 60) then
			SelectMsg(UID, 2, savenum, 4627, NPC, 18, 318);
		elseif ( MonsterCount03 < 80) then
			SelectMsg(UID, 2, savenum, 4628, NPC, 18, 319);
end
end
end

local savenum = 370;

if (EVENT == 300) then -- 80 Level Enigma & Cruel
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 872);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 877);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 882);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 887);
	end
end

if (EVENT == 302) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 351, NPC, 22, 303, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 351, NPC, 10, -1);
	end
end

if (EVENT == 303) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 873);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 878);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 883);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 888);
	end
end

if (EVENT == 304) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 876);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 881);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 886);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 891);
	end
end

if (EVENT == 305) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 875);
		SelectMsg(UID, 2, savenum, 351, NPC, 14, -1);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 880);
		SelectMsg(UID, 2, savenum, 351, NPC, 14, -1);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 885);
		SelectMsg(UID, 2, savenum, 351, NPC, 14, -1);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 890);
		SelectMsg(UID, 2, savenum, 351, NPC, 14, -1);
	end
end

if (EVENT == 307) then
	MonsterCount1 = CountMonsterQuestSub(UID, 370, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 370, 2);
	if (MonsterCount1 < 1) then
		SelectMsg(UID, 2, savenum, 351, NPC, 18, 306);
	elseif (MonsterCount2 < 1) then
		SelectMsg(UID, 2, savenum, 351, NPC, 18, 309);
	else
		SelectMsg(UID, 5, savenum, 351, NPC, 41, 308,27, -1);
	end
end

if (EVENT == 306) then
	ShowMap(UID, 165);
end

if (EVENT == 309) then
	ShowMap(UID, 163);
end

if (EVENT == 308) then
	MonsterCount1 = CountMonsterQuestSub(UID, 370, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 370, 2);
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif (MonsterCount1 < 1) then
		SelectMsg(UID, 2, savenum, 351, NPC, 18, 306);
	elseif (MonsterCount2 < 1) then
		SelectMsg(UID, 2, savenum, 351, NPC, 18, 309);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,152,STEP,1);
		SaveEvent(UID, 874);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,153,STEP,1);
		SaveEvent(UID, 879);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,154,STEP,1);
		SaveEvent(UID, 884);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,155,STEP,1);
		SaveEvent(UID, 889);
end
end
end

if (EVENT == 400) then
SelectMsg(UID, 4, 444, 6145, NPC, 22, 401,23,-1);
end

if (EVENT == 401) then
SelectMsg(UID, 15, savenum, -1, NPC);
SaveEvent(UID, 7156);
RunQuestExchange(UID,536);
end