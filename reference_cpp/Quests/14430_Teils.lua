-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14430;

if (EVENT == 150) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 8200, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 8202, NPC)
	else
		EVENT = QuestNum
	end
end

local sav = 153;

if (EVENT == 8600) then -- 44 Level Ape
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8910);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8915);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8920);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8925);
	end
end


if (EVENT == 8610) then
	SelectMsg(UID, 2, Sav, 843, NPC, 3002, 8611);
end

if (EVENT == 8611) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 153, 843, NPC, 3018, 8612, 3019, 8619);
	else
		SelectMsg(UID, 2, 153, 843, NPC, 4242, -1);
	end
end

if (EVENT == 8612) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8911);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8916);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8921);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8926);
	end
end

if (EVENT == 8619) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8914);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8919);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8924);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8929);
	end
end

if (EVENT == 8613) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8913);
		EVENT = 8614
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8918);
		EVENT = 8614
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8923);
		EVENT = 8614
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8928);
		EVENT = 8614
	end
end

if (EVENT == 8614) then
	SelectMsg(UID, 2, Sav, 8415, NPC, 4080, -1);
end

if (EVENT == 8615) then
	MonsterCount  = CountMonsterQuestSub(UID, 153, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, Sav, 8190, NPC, 18, 8617);
	else
		SelectMsg(UID, 4, 153, 8413, NPC, 41, 8618, 27, -1);
	end
end

if (EVENT == 8617) then
	ShowMap(UID, 114);
end

if (EVENT == 8618) then
	QuestStatusCheck = GetQuestStatus(UID, 153) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount  = CountMonsterQuestSub(UID, 153, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, Sav, 8190, NPC, 18, 8617);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,924)
		SaveEvent(UID, 8912);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,925)
		SaveEvent(UID, 8917);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,926)
		SaveEvent(UID, 8922);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,927)
		SaveEvent(UID, 8927);
end
end
end
end

if (EVENT == 8270) then -- 45 Level Kongau
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8868);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8873);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8878);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8883);
	end
end

if (EVENT == 8272) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 181, 8405, NPC, 10, 8275);
	else
		SelectMsg(UID, 2, 181, 8406, NPC, 10, -1);
	end
end

if (EVENT == 8275) then
	SelectMsg(UID, 4, 181, 8407, NPC, 22, 8273, 23, 8274);
end

if (EVENT == 8273) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8869);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8874);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8879);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8884);
	end
end

if (EVENT == 8274) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8872);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8877);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8882);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8887);
	end
end

if (EVENT == 8280) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8871);
		EVENT = 8281
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8876);
		EVENT = 8281
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8881);
		EVENT = 8281
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8886);
		EVENT = 8281
	end
end

if (EVENT == 8281) then
	SelectMsg(UID, 2, 181, 8408, NPC, 3002, -1);
end

if (EVENT == 8276) then
	MonsterCount = CountMonsterQuestSub(UID, 181, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 181, 8409, NPC, 18, 8277);
	else 
		SelectMsg(UID, 4, 181, 8410, NPC, 41, 8278, 27, -1);
	end
end

if (EVENT == 8277) then
	ShowMap(UID, 511);
end

if (EVENT == 8278) then
	QuestStatusCheck = GetQuestStatus(UID, 181) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 181, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 181, 8409, NPC, 18, 8277);
	else 
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,985)
		SaveEvent(UID, 8870);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,986)
		SaveEvent(UID, 8875);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,987)
		SaveEvent(UID, 8880);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,988)
		SaveEvent(UID, 8885);
end
end
end
end

if (EVENT == 8500) then -- 46 Level Burning Skeleton
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8952);
		EVENT = 8501
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8957);
		EVENT = 8501
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8962);
		EVENT = 8501
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8967);
		EVENT = 8501
	end
end

if (EVENT == 8501) then
	SelectMsg(UID, 2, 198, 8195, NPC, 56, -1);
end

if (EVENT == 8502) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 198, 8196, NPC, 10, 8505);
	else
		SelectMsg(UID, 2, 198, 8406, NPC, 10, -1);
	end
end

if (EVENT == 8505) then
	SelectMsg(UID, 4, 198, 8197, NPC, 22, 8503, 23, 8504);
end

if (EVENT == 8503) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8953);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8958);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8963);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8968);
	end
end

if (EVENT == 8504) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8956);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8961);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8966);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8971);
	end
end

if (EVENT == 8510) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8955);
		EVENT = 8511
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8960);
		EVENT = 8511
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8965);
		EVENT = 8511
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8970);
		EVENT = 8511
	end
end

if (EVENT == 8511) then
	SelectMsg(UID, 2, 198, 8188, NPC, 3007, -1);
end

if (EVENT == 8506) then
	MonsterCount = CountMonsterQuestSub(UID, 198, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 198, 8409, NPC, 18, 8507);
	else
		SelectMsg(UID, 4, 198, 8198, NPC, 41, 8508, 27, -1);
	end
end

if (EVENT == 8507) then
	ShowMap(UID, 509);
end

if (EVENT == 8508) then
	QuestStatusCheck = GetQuestStatus(UID, 198) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 198, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 198, 8409, NPC, 18, 8507);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,900)	
	SaveEvent(UID, 8954);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,901)
		SaveEvent(UID, 8959);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,902)
		SaveEvent(UID, 8964);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,903)
		SaveEvent(UID, 8969);
	end
end
end
end

if (EVENT == 8070) then -- 48 Level Ash Knight
	SelectMsg(UID, 2, 209, 8148, NPC, 14, 8071);
end

if (EVENT == 8071) then
	SaveEvent(UID, 8991);
end

if (EVENT == 8072) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 209, 8156, NPC, 10, 8080);
	else
		SelectMsg(UID, 2, 209, 8406, NPC, 10, -1);
	end
end

if (EVENT == 8080) then
	SelectMsg(UID, 4, 209, 8174, NPC, 22, 8073, 23, 8074);
end

if (EVENT == 8073) then
	SaveEvent(UID, 8992);
end

if (EVENT == 8074) then
	SaveEvent(UID, 8995);
end

if (EVENT == 8075) then
	SelectMsg(UID, 2, 209, 8215, NPC, 3014, -1);
	SaveEvent(UID, 8994);
end

if (EVENT == 8077) then
	MonsterCount = CountMonsterQuestSub(UID, 209, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 209, 8409, NPC, 18, 8078);
	else
		SelectMsg(UID, 4, 209, 8216, NPC, 41, 8079, 27, -1);
	end
end

if (EVENT == 8078) then
	ShowMap(UID, 503);
end

if (EVENT == 8079) then
	QuestStatusCheck = GetQuestStatus(UID, 209) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 209, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 209, 8409, NPC, 18, 8078);
	else
RunQuestExchange(UID,800)
	SaveEvent(UID, 8993);    
end
end
end

if (EVENT == 8150) then -- 49 Level Haunga
	SelectMsg(UID, 2, 213, 8221, NPC, 3002, 8151);
end

if (EVENT == 8151) then
	SaveEvent(UID, 9003);
end

if (EVENT == 8152) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 213, 8222, NPC, 10, 8160);
	else
		SelectMsg(UID, 2, 213, 8406, NPC, 10, -1);
	end
end

if (EVENT == 8160) then
	SelectMsg(UID, 4, 213, 8223, NPC, 22, 8153, 23, 8154);
end

if (EVENT == 8153) then
	SaveEvent(UID, 9004);
end

if (EVENT == 8154) then
	SaveEvent(UID, 9007);
end

if (EVENT == 8155) then
	SelectMsg(UID, 2, 213, 8215, NPC, 3014, -1);
	SaveEvent(UID, 9006);
end

if (EVENT == 8157) then
	MonsterCount = CountMonsterQuestSub(UID, 213, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 213, 8409, NPC, 18, 8158);
	else
		SelectMsg(UID, 4, 213, 8224, NPC, 41, 8159, 27, -1);
	end
end

if (EVENT == 8158) then
	ShowMap(UID, 515);
end

if (EVENT == 8159) then
	QuestStatusCheck = GetQuestStatus(UID, 213) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 213, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 213, 8409, NPC, 18, 8158);
	else
RunQuestExchange(UID,949)
	SaveEvent(UID, 9005);	 
end
end
end

if (EVENT == 9470) then -- 50 Level Sheriff
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9576);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9581);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9586);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9591);
	end
end

if (EVENT == 9472) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 222, 8196, NPC, 10, 9475);
	else
		SelectMsg(UID, 2, 222, 8406, NPC, 10, -1);
	end
end

if (EVENT == 9475) then
	SelectMsg(UID, 4, 222, 8197, NPC, 22, 9473, 23, 9474);
end

if (EVENT == 9473) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9577);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9582);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9587);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9592);
	end
end

if (EVENT == 9474) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9580);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9585);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9590);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9595);
	end
end

if (EVENT == 9480) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9579);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9584);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9589);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9594);
	end
end

if (EVENT == 9476) then
	MonsterCount = CountMonsterQuestSub(UID, 222, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 222, 8409, NPC, 18, 9477);
	else
		SelectMsg(UID, 5, 222, 8198, NPC, 41, 9478, 27, -1);
	end
end

if (EVENT == 9477) then
	ShowMap(UID, 622);
end

if (EVENT == 9478) then
	QuestStatusCheck = GetQuestStatus(UID, 222) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 222, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 222, 8409, NPC, 18, 9477);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,1126,STEP,1);
		SaveEvent(UID, 9578);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,1127,STEP,1);
		SaveEvent(UID, 9583);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,1128,STEP,1);
		SaveEvent(UID, 9588);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,1129,STEP,1);
		SaveEvent(UID, 9593);
end
end
end
end

if (EVENT == 8550) then -- 52 Level Dragon Tooth Soldier
	SelectMsg(UID, 2, 230, 8005, NPC, 14, 8551);
end

if (EVENT == 8551) then
	SaveEvent(UID, 9051);
end

if (EVENT == 8552) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 230, 8006, NPC, 10, 8560);
	else
		SelectMsg(UID, 2, 230, 8406, NPC, 10, -1);
	end
end

if (EVENT == 8560) then
	SelectMsg(UID, 4, 230, 8007, NPC, 22, 8553, 23, 8554);
end

if (EVENT == 8553) then
	SaveEvent(UID, 9052);
end

if (EVENT == 8554) then
	SaveEvent(UID, 9055);
end

if (EVENT == 8555) then
	SelectMsg(UID, 2, 230, 8008, NPC, 3014, -1);
	SaveEvent(UID, 9054);
end

if (EVENT == 8557) then
	MonsterCount = CountMonsterQuestSub(UID, 230, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 230, 8409, NPC, 18, 8558);
	else
		SelectMsg(UID, 5, 230, 8012, NPC, 41, 8559, 27, -1);
	end
end

if (EVENT == 8558) then
	ShowMap(UID, 584);
end

if (EVENT == 8559) then
	QuestStatusCheck = GetQuestStatus(UID, 230) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 230, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 230, 8409, NPC, 18, 8558);
	else
RunQuestExchange(UID,1002,STEP,1);    
		SaveEvent(UID, 9053);
end
end
end

if (EVENT == 9490) then -- 52 Level Garuna
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9618);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9623);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9628);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9633);
	end
end

if (EVENT == 9492) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 232, 8196, NPC, 10, 9495);
	else
		SelectMsg(UID, 2, 232, 8406, NPC, 10, -1);
	end
end

if (EVENT == 9495) then
	SelectMsg(UID, 4, 232, 8197, NPC, 22, 9493, 23, 9494);
end

if (EVENT == 9493) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9619);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9624);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9629);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9634);
	end
end

if (EVENT == 9494) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9622);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9627);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9632);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9637);
	end
end

if (EVENT == 9500) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9621);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9626);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9631);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9636);
	end
end

if (EVENT == 9496) then
	MonsterCount = CountMonsterQuestSub(UID, 232, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 232, 8409, NPC, 18, 9497);
	else
		SelectMsg(UID, 5, 232, 8198, NPC, 41, 9498, 27, -1);
	end
end

if (EVENT == 9497) then
	ShowMap(UID, 179);
end

if (EVENT == 9498) then
	QuestStatusCheck = GetQuestStatus(UID, 232) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 232, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 232, 8409, NPC, 18, 9497);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,1134,STEP,1);
		SaveEvent(UID, 9620);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,1135,STEP,1);
		SaveEvent(UID, 9625);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,1136,STEP,1);
		SaveEvent(UID, 9630);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,1137,STEP,1);
		SaveEvent(UID, 9635);
end
end
end
end

if (EVENT == 400) then -- 54 Level Dragon Tooth Skeleton Premium
	SaveEvent(UID, 2271);
end

if (EVENT == 402) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 470, 8168, NPC, 22, 403, 23, 404);
	else
		SelectMsg(UID, 2, 470, 8168, NPC, 10, -1);
	end
end

if (EVENT == 403) then
	SaveEvent(UID, 2272);
end

if (EVENT == 404) then
	SaveEvent(UID, 2275);
end

if (EVENT == 405) then
	SaveEvent(UID, 2274);
end

if (EVENT == 407) then
	MonsterCount = CountMonsterQuestSub(UID, 470, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 470, 8168, NPC, 18, 408);
	else
		SelectMsg(UID, 4, 470, 8169, NPC, 41, 409, 23, 408);
	end
end

if (EVENT == 408) then
	ShowMap(UID, 29);
end

if (EVENT == 409) then
	QuestStatusCheck = GetQuestStatus(UID, 470) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 470, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 470, 8168, NPC, 18, 408);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1943)
		SaveEvent(UID, 2273);
	else
		RunQuestExchange(UID,1943)
		SaveEvent(UID, 2273);
	end
end
end
end

if (EVENT == 9000) then -- 54 Level Dragon Tooth Skeleton
	SelectMsg(UID, 2, 240, 8005, NPC, 14, 9001);
end

if (EVENT == 9001) then
	SaveEvent(UID, 9087);
end

if (EVENT == 9002) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 240, 8015, NPC, 10, 9010);
	else
		SelectMsg(UID, 2, 240, 8406, NPC, 10, -1);
	end
end

if (EVENT == 9010) then
	SelectMsg(UID, 4, 240, 8016, NPC, 22, 9003, 23, 9004);
end

if (EVENT == 9003) then
	SaveEvent(UID, 9088);
end

if (EVENT == 9004) then
	SaveEvent(UID, 9091);
end

if (EVENT == 9005) then
	SelectMsg(UID, 2, 240, 8008, NPC, 3014, -1);
	SaveEvent(UID, 9090);
end

if (EVENT == 9007) then
	MonsterCount = CountMonsterQuestSub(UID, 240, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 240, 8409, NPC, 18, 9008);
	else
		SelectMsg(UID, 4, 240, 8224, NPC, 41, 9009, 27, -1);
	end
end

if (EVENT == 9008) then
	ShowMap(UID, 29);
end

if (EVENT == 9009) then
	QuestStatusCheck = GetQuestStatus(UID, 240) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 240, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 240, 8409, NPC, 18, 9008);
	else
RunQuestExchange(UID,943)
	SaveEvent(UID, 9089);   
end
end
end

if (EVENT == 1000) then -- 44 Level Ape  Premium
	SaveEvent(UID, 2115);
end

if (EVENT == 1010) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 420, 843, NPC, 22, 1011, 23, 1012);
	else
		SelectMsg(UID, 2, 420, 844, NPC, 10, -1);
	end
end

if (EVENT == 1011) then
	SaveEvent(UID, 2116);
end

if (EVENT == 1012) then
	SaveEvent(UID, 2119);
end

if (EVENT == 1013) then
	SaveEvent(UID, 2118);
end

if (EVENT == 1015) then
	MonsterCount = CountMonsterQuestSub(UID, 420, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 420, 843, NPC, 18, 1016);
	else
		SelectMsg(UID, 4, 420, 844, NPC, 41, 1017, 23, 1016);
	end
end

if (EVENT == 1016) then
	ShowMap(UID, 29);
end

if (EVENT == 1017) then
	QuestStatusCheck = GetQuestStatus(UID, 420) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 420, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 420, 843, NPC, 18, 1016);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1214)
		SaveEvent(UID, 2117);
	else
		RunQuestExchange(UID,1214)
		SaveEvent(UID, 2117);
	end
end
end
end

if (EVENT == 1100) then -- 45 Level Kangaus  Premium
	SaveEvent(UID, 2139);
end

if (EVENT == 1102) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 424, 8154, NPC, 22, 1103, 23, 1104);
	else
		SelectMsg(UID, 2, 424, 8155, NPC, 10, -1);
	end
end

if (EVENT == 1103) then
	SaveEvent(UID, 2140);
end

if (EVENT == 1104) then
	SaveEvent(UID, 2143);
end

if (EVENT == 1105) then
	SaveEvent(UID, 2142);
end

if (EVENT == 1106) then
	MonsterCount = CountMonsterQuestSub(UID, 424, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 424, 8154, NPC, 18, 1107);
	else
		SelectMsg(UID, 4, 424, 8155, NPC, 41, 1108, 23, 1107);
	end
end

if (EVENT == 1107) then
	ShowMap(UID, 29);
end

if (EVENT == 1108) then
	QuestStatusCheck = GetQuestStatus(UID, 424) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 424, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 424, 8154, NPC, 18, 1107);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1216)
		SaveEvent(UID, 2141);
	else
		RunQuestExchange(UID,1216)
		SaveEvent(UID, 2141);
	end
end
end
end

if (EVENT == 1200) then -- 46 Level Burning Skeleton  Premium
	SaveEvent(UID, 2151);
end

if (EVENT == 1202) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 426, 8495, NPC, 22, 1203, 23, 1204);
	else
		SelectMsg(UID, 2, 426, 8495, NPC, 10, -1);
	end
end

if (EVENT == 1203) then
	SaveEvent(UID, 2152);
end

if (EVENT == 1204) then
	SaveEvent(UID, 2155);
end

if (EVENT == 1205) then
	SaveEvent(UID, 2154);
end

if (EVENT == 1206) then
	MonsterCount = CountMonsterQuestSub(UID, 426, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 426, 8154, NPC, 18, 1207);
	else
		SelectMsg(UID, 4, 426, 8155, NPC, 41, 1208, 23, 1207);
	end
	end
	
if (EVENT == 1207) then
	ShowMap(UID, 29);
end

if (EVENT == 1208) then
	QuestStatusCheck = GetQuestStatus(UID, 426) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 426, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 426, 8154, NPC, 18, 1207);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1217)
		SaveEvent(UID, 2153);
	else
		RunQuestExchange(UID,1217)
		SaveEvent(UID, 2153);
	end
end
end
end

if (EVENT == 1300) then -- 47 Level Ash Knight  Premium
	SaveEvent(UID, 2175);
end

if (EVENT == 1302) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 430, 8009, NPC, 22, 1303, 23, 1304);
	else
		SelectMsg(UID, 2, 430, 8009, NPC, 10, -1);
	end
end

if (EVENT == 1303) then
	SaveEvent(UID, 2176);
end

if (EVENT == 1304) then
	SaveEvent(UID, 2179);
end

if (EVENT == 1305) then
	SaveEvent(UID, 2178);
end

if (EVENT == 1307) then
	MonsterCount = CountMonsterQuestSub(UID, 430, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 430, 8009, NPC, 18, 1308);
	else
		SelectMsg(UID, 4, 430, 8009, NPC, 22, 1309, 23, 1308);
	end
	end
	
	if (EVENT == 1308) then
	ShowMap(UID, 508);
end

if (EVENT == 1309) then
	QuestStatusCheck = GetQuestStatus(UID, 430) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 430, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 430, 8009, NPC, 18, 1308);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1219)
		SaveEvent(UID, 2177);
	else
		RunQuestExchange(UID,1219)
		SaveEvent(UID, 2177);
	end
end
end
end

if (EVENT == 1400) then -- Haunga  Premium
	SaveEvent(UID, 2187);
end

if (EVENT == 1402) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 432, 8160, NPC, 22, 1403, 23, 1404);
	else
		SelectMsg(UID, 2, 432, 8160, NPC, 10, -1);
	end
end

if (EVENT == 1403) then
	SaveEvent(UID, 2188);
end

if (EVENT == 1404) then
	SaveEvent(UID, 2191);
end

if (EVENT == 1405) then
	SaveEvent(UID, 2190);
end

if (EVENT == 1407) then
	MonsterCount = CountMonsterQuestSub(UID, 432, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 432, 8160, NPC, 18, 1408);
	else
		SelectMsg(UID, 4, 432, 8160, NPC, 22, 1409, 23, 1408);
	end
	end
	
if (EVENT == 1408) then
	ShowMap(UID, 508);
end

if (EVENT == 1409) then
	QuestStatusCheck = GetQuestStatus(UID, 432) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 432, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 432, 8160, NPC, 18, 1408);
	else
		RunQuestExchange(UID,1220)
		SaveEvent(UID, 2189);
	end
end
end

if (EVENT == 300) then -- Dragon Tooth Skeleton Premium
	SaveEvent(UID, 2235);
end

if (EVENT == 302) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 464, 8168, NPC, 22, 303, 23, 304);
	else
		SelectMsg(UID, 2, 464, 8168, NPC, 10, -1);
	end
end

if (EVENT == 303) then
	SaveEvent(UID, 2236);
end

if (EVENT == 304) then
	SaveEvent(UID, 2239);
end

if (EVENT == 305) then
	SaveEvent(UID, 2238);
end

if (EVENT == 307) then
	MonsterCount = CountMonsterQuestSub(UID, 464, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 464, 8168, NPC, 18, 308);
	else
		SelectMsg(UID, 4, 464, 8169, NPC, 41, 309, 23, 308);
	end
end

if (EVENT == 308) then
	ShowMap(UID, 29);
end

if (EVENT == 309) then
	QuestStatusCheck = GetQuestStatus(UID, 464) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 464, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 464, 8168, NPC, 18, 308);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,21002)
		SaveEvent(UID, 2237);
	else
		RunQuestExchange(UID,21002)
		SaveEvent(UID, 2237);
	end
end
end
end


local savenum = 394
local talknum = 8765

if(EVENT == 9900) then -- Sürekli Küpe
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 9901, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == 9901) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1373);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1376);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1379);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1382);
	end
end

if(EVENT == 9905 )then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1374);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1377);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1380);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1383);
	end
end

if(EVENT == 9903) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 < 20) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, 9477);
	else
		SelectMsg(UID, 5, savenum, talknum, NPC, 41, 9906, 27, -1);
	end
end

if(EVENT == 9906 )then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 < 20) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, 9477);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		RunQuestExchange(UID, 11122)
		SaveEvent(UID, 1375);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		RunQuestExchange(UID, 11123)
		SaveEvent(UID, 1378);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		RunQuestExchange(UID, 11124)
		SaveEvent(UID, 1381);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		RunQuestExchange(UID, 11125)
		SaveEvent(UID, 1384);
	end
end
end

local savenum = 399
local talknum = 8767

if(EVENT == 9800) then  -- Sürekli pendat
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 9801, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == 9801) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1433);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1436);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1439);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1442);
	end
end

if(EVENT == 9805) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1434);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1437);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1440);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1443);
	end
end

if(EVENT == 9803) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 < 20) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, 9497);
	else
		SelectMsg(UID, 5, savenum, talknum, NPC, 41, 9806, 27, -1);
	end
end

if (EVENT == 9806) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 < 20) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, 9497);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	RunQuestExchange(UID,11134)
	SaveEvent(UID, 1435);
	elseif (Class == 2 or Class == 7 or Class == 8) then
	RunQuestExchange(UID,11135)
	SaveEvent(UID, 1438);
	elseif (Class == 3 or Class == 9 or Class == 10) then
	RunQuestExchange(UID,11136)
	SaveEvent(UID, 1441);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	RunQuestExchange(UID,11137)
	SaveEvent(UID, 1444);
end
end
end

if (EVENT == 202) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     EVENT = 211
	elseif (Class == 2 or Class == 7 or Class == 8) then
     EVENT = 212
	elseif (Class == 3 or Class == 9 or Class == 10) then
     EVENT = 213
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 EVENT = 214
	end
end

if(EVENT == 211) then
	SelectMsg(UID, 4, 448, 9222, NPC, 22, 215, 23, -1);
end

if(EVENT == 212) then
	SelectMsg(UID, 4, 449, 9222, NPC, 22, 215, 23, -1);
end

if(EVENT == 213) then
	SelectMsg(UID, 4, 450, 9222, NPC, 22, 215, 23, -1);
end

if(EVENT == 214) then
	SelectMsg(UID, 4, 451, 9222, NPC, 22, 215, 23, -1);
end

if (EVENT == 210) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     SaveEvent(UID, 7184);
	elseif (Class == 2 or Class == 7 or Class == 8) then
     SaveEvent(UID, 7189);
	elseif (Class == 3 or Class == 9 or Class == 10) then
     SaveEvent(UID, 7194);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 SaveEvent(UID, 7199);
	end
end

if (EVENT == 215) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     SaveEvent(UID, 7182);
	elseif (Class == 2 or Class == 7 or Class == 8) then
     SaveEvent(UID, 7187);
	elseif (Class == 3 or Class == 9 or Class == 10) then
     SaveEvent(UID, 7192);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 SaveEvent(UID, 7197);
	end
end

if (EVENT == 206) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     EVENT = 216
	elseif (Class == 2 or Class == 7 or Class == 8) then
     EVENT = 217
	elseif (Class == 3 or Class == 9 or Class == 10) then
     EVENT = 218
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 EVENT = 219
	end
end

if(EVENT == 216) then
	MonsterCount1 = CountMonsterQuestSub(UID, 448, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 448, 2);
	if (MonsterCount1 < 10) then
		SelectMsg(UID, 2, 448, 9222, NPC, 18, 220);
	elseif (MonsterCount2 < 10) then
		SelectMsg(UID, 2, 448, 9222, NPC, 18, 221);
	else
		SelectMsg(UID, 4, 448, 9222, NPC, 41, 203, 27, -1);
	end
end

if (EVENT == 220) then
	ShowMap(UID, 707);--706
end

if (EVENT == 221) then
	ShowMap(UID, 709);--708
end

if(EVENT == 217) then
	MonsterCount1 = CountMonsterQuestSub(UID, 449, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 449, 2);
	if (MonsterCount1 < 10) then
		SelectMsg(UID, 2, 449, 9222, NPC, 18, 222);
	elseif (MonsterCount2 < 10) then
		SelectMsg(UID, 2, 449, 9222, NPC, 18, 223);
	else
		SelectMsg(UID, 4, 449, 9222, NPC, 41, 203, 27, -1);
	end
end

if (EVENT == 222) then
	ShowMap(UID, 707);--706
end

if (EVENT == 223) then
	ShowMap(UID, 711);--710
end

if(EVENT == 218) then
	MonsterCount1 = CountMonsterQuestSub(UID, 450, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 450, 2);
	if (MonsterCount1 < 10) then
		SelectMsg(UID, 2, 450, 9222, NPC, 18, 224);
	elseif (MonsterCount2 < 10) then
		SelectMsg(UID, 2, 450, 9222, NPC, 18, 225);
	else
		SelectMsg(UID, 4, 450, 9222, NPC, 41, 203, 27, -1);
	end
end

if (EVENT == 224) then
	ShowMap(UID, 713);--712
end

if (EVENT == 225) then
	ShowMap(UID, 715);--714
end

if(EVENT == 219) then
	MonsterCount1 = CountMonsterQuestSub(UID, 451, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 451, 2);
	if (MonsterCount1 < 10) then
		SelectMsg(UID, 2, 451, 9222, NPC, 18, 226);
	elseif (MonsterCount2 < 10) then
		SelectMsg(UID, 2, 451, 9222, NPC, 18, 227);
	else
		SelectMsg(UID, 4, 451, 9222, NPC, 41, 203, 27, -1);
	end
end

if (EVENT == 226) then
	ShowMap(UID, 715);--714
end

if (EVENT == 227) then
	ShowMap(UID, 711);--710
end

if (EVENT == 203) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     RunQuestExchange(UID,701)
	SaveEvent(UID, 7183);
	elseif (Class == 2 or Class == 7 or Class == 8) then
     RunQuestExchange(UID,702)
	SaveEvent(UID, 7188);
	elseif (Class == 3 or Class == 9 or Class == 10) then
     RunQuestExchange(UID,703)
	SaveEvent(UID, 7193);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 RunQuestExchange(UID,704)
	SaveEvent(UID, 7198);
	end
end
------------------------------------------------------------------------------------------------
if (EVENT == 502) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     EVENT = 511
	elseif (Class == 2 or Class == 7 or Class == 8) then
     EVENT = 512
	elseif (Class == 3 or Class == 9 or Class == 10) then
     EVENT = 513
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 EVENT = 514
	end
end

if(EVENT == 511) then
	SelectMsg(UID, 4, 483, 9222, NPC, 22, 515, 23, -1);
end

if(EVENT == 512) then
	SelectMsg(UID, 4, 484, 9222, NPC, 22, 515, 23, -1);
end

if(EVENT == 513) then
	SelectMsg(UID, 4, 485, 9222, NPC, 22, 515, 23, -1);
end

if(EVENT == 514) then
	SelectMsg(UID, 4, 486, 9222, NPC, 22, 515, 23, -1);
end

if (EVENT == 510) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     SaveEvent(UID, 2385);
	elseif (Class == 2 or Class == 7 or Class == 8) then
     SaveEvent(UID, 2390);
	elseif (Class == 3 or Class == 9 or Class == 10) then
     SaveEvent(UID, 2395);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 SaveEvent(UID, 2400);
	end
end

if (EVENT == 515) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     SaveEvent(UID, 2383);
	elseif (Class == 2 or Class == 7 or Class == 8) then
     SaveEvent(UID, 2388);
	elseif (Class == 3 or Class == 9 or Class == 10) then
     SaveEvent(UID, 2393);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 SaveEvent(UID, 2398);
	end
end

if (EVENT == 506) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     EVENT = 516
	elseif (Class == 2 or Class == 7 or Class == 8) then
     EVENT = 517
	elseif (Class == 3 or Class == 9 or Class == 10) then
     EVENT = 518
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 EVENT = 519
	end
end

if(EVENT == 516) then
	MonsterCount1 = CountMonsterQuestSub(UID, 483, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 483, 2);
	if (MonsterCount1 < 2) then
		SelectMsg(UID, 2, 483, 9222, NPC, 18, 520);
	elseif (MonsterCount2 < 2) then
		SelectMsg(UID, 2, 483, 9222, NPC, 18, 521);
	else
		SelectMsg(UID, 4, 483, 9222, NPC, 41, 503, 27, -1);
	end
end

if (EVENT == 520) then
	ShowMap(UID, 707);--706
end

if (EVENT == 521) then
	ShowMap(UID, 709);--708
end

if(EVENT == 517) then
	MonsterCount1 = CountMonsterQuestSub(UID, 484, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 484, 2);
	if (MonsterCount1 < 2) then
		SelectMsg(UID, 2, 484, 9222, NPC, 18, 522);
	elseif (MonsterCount2 < 2) then
		SelectMsg(UID, 2, 484, 9222, NPC, 18, 523);
	else
		SelectMsg(UID, 4, 484, 9222, NPC, 41, 503, 27, -1);
	end
end

if (EVENT == 522) then
	ShowMap(UID, 713);--706
end

if (EVENT == 523) then
	ShowMap(UID, 711);--710
end

if(EVENT == 518) then
	MonsterCount1 = CountMonsterQuestSub(UID, 485, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 485, 2);
	if (MonsterCount1 < 2) then
		SelectMsg(UID, 2, 485, 9222, NPC, 18, 524);
	elseif (MonsterCount2 < 2) then
		SelectMsg(UID, 2, 485, 9222, NPC, 18, 525);
	else
		SelectMsg(UID, 4, 485, 9222, NPC, 41, 503, 27, -1);
	end
end

if (EVENT == 524) then
	ShowMap(UID, 713);--712
end

if (EVENT == 525) then
	ShowMap(UID, 715);--714
end

if(EVENT == 519) then
	MonsterCount1 = CountMonsterQuestSub(UID, 486, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 486, 2);
	if (MonsterCount1 < 2) then
		SelectMsg(UID, 2, 486, 9222, NPC, 18, 526);
	elseif (MonsterCount2 < 2) then
		SelectMsg(UID, 2, 486, 9222, NPC, 18, 527);
	else
		SelectMsg(UID, 4, 486, 9222, NPC, 41, 503, 27, -1);
	end
end

if (EVENT == 526) then
	ShowMap(UID, 715);--714
end

if (EVENT == 527) then
	ShowMap(UID, 711);--710
end

if (EVENT == 503) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
     RunQuestExchange(UID,1701)
	SaveEvent(UID, 2384);
	elseif (Class == 2 or Class == 7 or Class == 8) then
     RunQuestExchange(UID,1702)
	SaveEvent(UID, 2389);
	elseif (Class == 3 or Class == 9 or Class == 10) then
     RunQuestExchange(UID,1703)
	SaveEvent(UID, 2394);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	 RunQuestExchange(UID,1704)
	SaveEvent(UID, 2399);
	end
end