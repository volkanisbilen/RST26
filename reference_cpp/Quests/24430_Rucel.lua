-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24430;

if (EVENT == 150) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 8199, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 8201, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 8610) then
	SelectMsg(UID, 2, 152, 8201, NPC, 3002, 8611);
end

if (EVENT == 8611) then
	SelectMsg(UID, 4, 152, 8181, NPC, 3018, 8612, 3019, -1);
end

if (EVENT == 8612) then
	QuestStatus = GetQuestStatus(UID, 152)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8890);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8895);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8900);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8905);
		end
	end
end

if (EVENT == 8613) then
	QuestStatus = GetQuestStatus(UID, 152)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 152, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 152, 8182, NPC, 18, 8617);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8892);
			EVENT = 8614
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8897);
			EVENT = 8614
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8902);
			EVENT = 8614
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8907);
			EVENT = 8614
			end
		end
	end
end

if (EVENT == 8614) then
	SelectMsg(UID, 2, 152, 8401, NPC, 4080, -1);
end

if (EVENT == 8615) then
	QuestStatus = GetQuestStatus(UID, 152)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 152, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 152, 8182, NPC, 18, 8617);
		else
			SelectMsg(UID, 4, 152, 8411, NPC, 41, 8618, 27, -1);
		end
	end
end

if (EVENT == 8617) then
	ShowMap(UID, 113);
end

if (EVENT == 8618) then
	QuestStatus = GetQuestStatus(UID, 152)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 152, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 152, 8182, NPC, 18, 8617);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,928);
			SaveEvent(UID, 8891);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,929);
			SaveEvent(UID, 8896);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,930);
			SaveEvent(UID, 8901);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,931);
			SaveEvent(UID, 8906);
			end	 	 
		end
	end
end

if (EVENT == 8272) then
	SelectMsg(UID, 2, 180, 8398, NPC, 10, 8275);
end

if (EVENT == 8275) then
	SelectMsg(UID, 4, 180, 8400, NPC, 22, 8273, 23, -1);
end

if (EVENT == 8273) then
	QuestStatus = GetQuestStatus(UID, 180)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8848);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8853);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8858);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8863);
		end
	end
end

if (EVENT == 8280) then
	QuestStatus = GetQuestStatus(UID, 180)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8850);
			EVENT = 8281
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8855);
			EVENT = 8281
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8860);
			EVENT = 8281
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8865);
			EVENT = 8281
		end
	end
end

if (EVENT == 8281) then
	SelectMsg(UID, 2, 180, 8401, NPC, 3002, -1);
end

if (EVENT == 8276) then
	QuestStatus = GetQuestStatus(UID, 180)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 180, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 180, 8402, NPC, 18, 8277);
		else
			SelectMsg(UID, 4, 180, 8403, NPC, 41, 8278, 27, -1);
		end
	end
end

if (EVENT == 8277) then
	ShowMap(UID, 512);
end

if (EVENT == 8278) then
	QuestStatus = GetQuestStatus(UID, 180)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 180, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 180, 8402, NPC, 18, 8277);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,989);
			SaveEvent(UID, 8849);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,990);
			SaveEvent(UID, 8854);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,991);
			SaveEvent(UID, 8859);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,992);
			SaveEvent(UID, 8864);
			end
		end
	end
end

if (EVENT == 8502) then
	SelectMsg(UID, 2, 197, 8192, NPC, 10, 8505);
end

if (EVENT == 8505) then
	SelectMsg(UID, 4, 197, 8193, NPC, 22, 8503, 23, -1);
end

if (EVENT == 8503) then
	QuestStatus = GetQuestStatus(UID, 197)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8932);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8937);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8942);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8947);
		end
	end
end

if (EVENT == 8510) then
	QuestStatus = GetQuestStatus(UID, 197)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 197, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 197, 8402, NPC, 18, 8507);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8934);
			EVENT = 8511
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8939);
			EVENT = 8511
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8944);
			EVENT = 8511
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8949);
			EVENT = 8511
			end
		end
	end
end

if (EVENT == 8511) then
	SelectMsg(UID, 2, 197, 8180, NPC, 3007, -1);
end

if (EVENT == 8506) then
	QuestStatus = GetQuestStatus(UID, 197)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 197, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 197, 8402, NPC, 18, 8507);
		else
			SelectMsg(UID, 4, 197, 8194, NPC, 41, 8508, 27, -1);
		end
	end
end

if (EVENT == 8507) then
	ShowMap(UID, 510);
end

if (EVENT == 8508) then
	QuestStatus = GetQuestStatus(UID, 197)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 197, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 197, 8402, NPC, 18, 8507);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,920);
			SaveEvent(UID, 8933);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,921);
			SaveEvent(UID, 8938);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,922);
			SaveEvent(UID, 8943);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,923);
			SaveEvent(UID, 8948);
			end
		end 
	end
end

if (EVENT == 8072) then
	SelectMsg(UID, 2, 208, 8155, NPC, 10, 8080);
end

if (EVENT == 8080) then
	SelectMsg(UID, 4, 208, 8173, NPC, 22, 8073, 23, -1);
end

if (EVENT == 8073) then
	QuestStatus = GetQuestStatus(UID, 208)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 8986);
	end
end

if (EVENT == 8075) then
	QuestStatus = GetQuestStatus(UID, 208)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 208, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 208, 8402, NPC, 18, 8078);
		else
			SelectMsg(UID, 2, 208, 8213, NPC, 3014, -1);
			SaveEvent(UID, 8988);
		end
	end
end

if (EVENT == 8077) then
	QuestStatus = GetQuestStatus(UID, 208)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 208, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 208, 8402, NPC, 18, 8078);
		else
			SelectMsg(UID, 4, 208, 8214, NPC, 41, 8079, 27, -1);
		end
	end
end

if (EVENT == 8078) then
	ShowMap(UID, 504);
end

if (EVENT == 8079) then
	QuestStatus = GetQuestStatus(UID, 208)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 208, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 208, 8402, NPC, 18, 8078);
		else
			RunQuestExchange(UID,818);
			SaveEvent(UID, 8987);	 
		end
	end
end

if (EVENT == 8152) then
	SelectMsg(UID, 2, 212, 8218, NPC, 10, 8160);
end

if (EVENT == 8160) then
	SelectMsg(UID, 4, 212, 8219, NPC, 22, 8153, 23, -1);
end

if (EVENT == 8153) then
	QuestStatus = GetQuestStatus(UID, 212)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 8998);
	end
end

if (EVENT == 8155) then
	QuestStatus = GetQuestStatus(UID, 212)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 212, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 212, 8402, NPC, 18, 8158);
		else
			SelectMsg(UID, 2, 212, 8213, NPC, 3014, -1);
			SaveEvent(UID, 9000);
		end
	end
end

if (EVENT == 8157) then
	QuestStatus = GetQuestStatus(UID, 212)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 212, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 212, 8402, NPC, 18, 8158);
		else
			SelectMsg(UID, 4, 212, 8220, NPC, 41, 8159, 27, -1);
		end
	end
end

if (EVENT == 8158) then
	ShowMap(UID, 516);
end

if (EVENT == 8159) then
	QuestStatus = GetQuestStatus(UID, 212)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 212, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 212, 8402, NPC, 18, 8158);
		else
			RunQuestExchange(UID,948);
			SaveEvent(UID, 8999);	
		end
	end
end

if (EVENT == 9472) then
	SelectMsg(UID, 2, 221, 8766, NPC, 10, 9475);
end

if (EVENT == 9475) then
	SelectMsg(UID, 4, 221, 8764, NPC, 22, 9473, 23, -1);
end

if (EVENT == 9473) then
	QuestStatus = GetQuestStatus(UID, 221)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9556);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9561);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9566);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9571);
		end
	end
end

if (EVENT == 9480) then
	QuestStatus = GetQuestStatus(UID, 221)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 221, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 221, 8765, NPC, 18, 9477);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9558);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9563);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9568);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9573);
			end
		end
	end
end

if (EVENT == 9476) then
	QuestStatus = GetQuestStatus(UID, 221)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 221, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 221, 8765, NPC, 18, 9477);
		else
			SelectMsg(UID, 5, 221, 8766, NPC, 41, 9478, 27, -1);
		end
	end
end

if (EVENT == 9477) then
	ShowMap(UID, 621);
end

if (EVENT == 9478) then
	QuestStatus = GetQuestStatus(UID, 221)
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then	
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 221, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 221, 8765, NPC, 18, 9477);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,1122,STEP,1);
			SaveEvent(UID, 9557);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,1123,STEP,1);
			SaveEvent(UID, 9562);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,1124,STEP,1);
			SaveEvent(UID, 9567);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,1125,STEP,1);
			SaveEvent(UID, 9572);
			end
		end
	end
end

if (EVENT == 8552) then
	SelectMsg(UID, 2, 229, 8001, NPC, 10, 8560);
end

if (EVENT == 8560) then
	SelectMsg(UID, 4, 229, 8002, NPC, 22, 8553, 23, -1);
end

if (EVENT == 8553) then
	QuestStatus = GetQuestStatus(UID, 229)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9046);
	end
end

if (EVENT == 8555) then
	QuestStatus = GetQuestStatus(UID, 229)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 229, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 229, 8402, NPC, 18, 8558);
		else
			SelectMsg(UID, 2, 229, 8003, NPC, 3014, -1);
			SaveEvent(UID, 9048);
		end
	end
end

if (EVENT == 8557) then
	QuestStatus = GetQuestStatus(UID, 229)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 229, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 229, 8402, NPC, 18, 8558);
		else
			SelectMsg(UID, 5, 229, 8004, NPC, 41, 8559, 27, -1);
		end
	end
end

if (EVENT == 8558) then
	ShowMap(UID, 585);
end

if (EVENT == 8559) then
	QuestStatus = GetQuestStatus(UID, 229)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 229, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 229, 8402, NPC, 18, 8558);
		else
			RunQuestExchange(UID,1001,STEP,1); 
			SaveEvent(UID, 9047);
		end
	end
end

if (EVENT == 9492) then
	SelectMsg(UID, 2, 231, 8192, NPC, 10, 9495);
end

if (EVENT == 9495) then
	SelectMsg(UID, 4, 231, 8193, NPC, 22, 9493, 23, -1);
end

if (EVENT == 9493) then
	QuestStatus = GetQuestStatus(UID, 231)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9598);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9603);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9608);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9613);
		end
	end
end

if (EVENT == 9500) then
	QuestStatus = GetQuestStatus(UID, 231)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 231, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 231, 8402, NPC, 18, 9497);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9600);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9605);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9610);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9615);
			end
		end
	end
end

if (EVENT == 9496) then
	QuestStatus = GetQuestStatus(UID, 231)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 231, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 231, 8402, NPC, 18, 9497);
		else
			SelectMsg(UID, 5, 231, 8194, NPC, 41, 9498, 27, -1);
		end
	end
end

if (EVENT == 9497) then
	ShowMap(UID, 178);
end

if (EVENT == 9498) then
	QuestStatus = GetQuestStatus(UID, 231)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 231, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 231, 8402, NPC, 18, 9497);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,1130,STEP,1);
			SaveEvent(UID, 9599);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,1131,STEP,1);
			SaveEvent(UID, 9604);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,1132,STEP,1);
			SaveEvent(UID, 9609);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,1133,STEP,1);
			SaveEvent(UID, 9614);
			end
		end
	end
end

if (EVENT == 402) then
	SelectMsg(UID, 4, 469, 8167, NPC, 22, 403, 23, -1);
end

if (EVENT == 403) then
	QuestStatus = GetQuestStatus(UID, 469)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2266);
	end
end

if (EVENT == 405) then
	QuestStatus = GetQuestStatus(UID, 469)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 469, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 469, 8167, NPC, 18, 408);
		else
			SaveEvent(UID, 2268);
		end
	end
end

if (EVENT == 407) then
	QuestStatus = GetQuestStatus(UID, 469)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 469, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 469, 8167, NPC, 18, 408);
		else
			SelectMsg(UID, 4, 469, 8167, NPC, 41, 409, 23, -1);
		end
	end
end

if (EVENT == 408) then
	ShowMap(UID, 186);
end

if (EVENT == 409) then
	QuestStatus = GetQuestStatus(UID, 469)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 469, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 469, 8167, NPC, 18, 408);
		else
			RunQuestExchange(UID,1942);
			SaveEvent(UID, 2267);
		end

	end
end

if (EVENT == 9002) then
	SelectMsg(UID, 2, 239, 8013, NPC, 10, 9010);
end

if (EVENT == 9010) then
	SelectMsg(UID, 4, 239, 8014, NPC, 22, 9003, 23, -1);
end

if (EVENT == 9003) then
	QuestStatus = GetQuestStatus(UID, 239)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9082);
	end
end

if (EVENT == 9005) then
	QuestStatus = GetQuestStatus(UID, 239)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 239, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 239, 8402, NPC, 18, 9008);
		else
			SelectMsg(UID, 2, 239, 8003, NPC, 3014, -1);
			SaveEvent(UID, 9084);
		end
	end
end

if (EVENT == 9007) then
	QuestStatus = GetQuestStatus(UID, 239)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 239, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 239, 8402, NPC, 18, 9008);
		else
			SelectMsg(UID, 4, 239, 8220, NPC, 41, 9009, 27, -1);
		end
	end
end

if (EVENT == 9008) then
	ShowMap(UID, 186);
end

if (EVENT == 9009) then
	QuestStatus = GetQuestStatus(UID, 239)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 239, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 239, 8402, NPC, 18, 9008);
		else
			RunQuestExchange(UID,942);
			SaveEvent(UID, 9083);	 
		end
	end
end

if (EVENT == 1010) then
	SelectMsg(UID, 4, 419, 8495, NPC, 22, 1011, 23, -1);
end

if (EVENT == 1011) then
	QuestStatus = GetQuestStatus(UID, 419)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2110);
	end
end

if (EVENT == 1013) then
	QuestStatus = GetQuestStatus(UID, 419)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 419, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 419, 8495, NPC, 18, 1016);
		else
			SaveEvent(UID, 2112);
		end
	end
end

if (EVENT == 1015) then
	QuestStatus = GetQuestStatus(UID, 419)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 419, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 419, 8495, NPC, 18, 1016);
		else
			SelectMsg(UID, 4, 419, 8496, NPC, 41, 1017, 23, -1);
		end
	end
end

if (EVENT == 1016) then
	ShowMap(UID, 29);
end

if (EVENT == 1017) then
	QuestStatus = GetQuestStatus(UID, 419)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 419, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 419, 8495, NPC, 18, 1016);
		else
		RunQuestExchange(UID,1198);
		SaveEvent(UID, 2111);
		end
	end
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 423, 8153, NPC, 22, 1103, 23, -1);
end

if (EVENT == 1103) then
	QuestStatus = GetQuestStatus(UID, 423)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2134);
	end
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 423)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 423, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 423, 8153, NPC, 18, 1107);
		else
			SaveEvent(UID, 2136);
		end
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 423)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 423, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 423, 8153, NPC, 18, 1107);
		else
			SelectMsg(UID, 4, 423, 8153, NPC, 41, 1108, 23, -1);
		end
	end
end

if (EVENT == 1107) then
	ShowMap(UID, 29);
end

if (EVENT == 1108) then
	QuestStatus = GetQuestStatus(UID, 423)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 423, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 423, 8153, NPC, 18, 1107);
		else
		RunQuestExchange(UID,1200);
		SaveEvent(UID, 2135);
		end
	end
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 425, 8494, NPC, 22, 1203, 23, -1);
end

if (EVENT == 1203) then
	QuestStatus = GetQuestStatus(UID, 425)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2146);
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 425)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 425, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 425, 8154, NPC, 18, 1207);
		else
			SaveEvent(UID, 2148);
		end
	end
end

if (EVENT == 1206) then
	QuestStatus = GetQuestStatus(UID, 425)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 425, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 425, 8154, NPC, 18, 1207);
		else
			SelectMsg(UID, 4, 425, 8155, NPC, 41, 1208, 23, -1);
		end
	end
end
	
if (EVENT == 1107) then
	ShowMap(UID, 29);
end

if (EVENT == 1208) then
	QuestStatus = GetQuestStatus(UID, 425)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 425, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 425, 8154, NPC, 18, 1207);
		else
			RunQuestExchange(UID,1201);
			SaveEvent(UID, 2147);
		end
	end
end

if (EVENT == 1302) then
	SelectMsg(UID, 4, 429, 8018, NPC, 22, 1303, 23, -1);
end

if (EVENT == 1303) then
	QuestStatus = GetQuestStatus(UID, 429)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2170);
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 429)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2172);
	end
end

if (EVENT == 1307) then
	QuestStatus = GetQuestStatus(UID, 429)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 429, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 429, 8018, NPC, 18, 1308);
		else
			SelectMsg(UID, 4, 429, 8018, NPC, 22, 1309, 23, -1);
		end
	end
end
	
if (EVENT == 1308) then
	ShowMap(UID, 508);
end

if (EVENT == 1309) then
	QuestStatus = GetQuestStatus(UID, 429)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 429, 1);
		if (MonsterCount < 20) then
		SelectMsg(UID, 2, 429, 8018, NPC, 18, 1308);
		else
		RunQuestExchange(UID,1203);
		SaveEvent(UID, 2171);
		end
	end
end

if (EVENT == 1402) then
	SelectMsg(UID, 4, 431, 8159, NPC, 22, 1403, 23, -1);
end

if (EVENT == 1403) then
	QuestStatus = GetQuestStatus(UID, 431)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2182);
	end
end

if (EVENT == 1405) then
	QuestStatus = GetQuestStatus(UID, 431)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 431, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 431, 8159, NPC, 18, 1408);
		else
			SaveEvent(UID, 2184);
		end
	end
end

if (EVENT == 1407) then
	QuestStatus = GetQuestStatus(UID, 431)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 431, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 431, 8159, NPC, 18, 1408);
		else
			SelectMsg(UID, 4, 431, 8159, NPC, 22, 1409, 23, -1);
		end
	end
end
	
if (EVENT == 1408) then
	ShowMap(UID, 508);
end

if (EVENT == 1409) then
	QuestStatus = GetQuestStatus(UID, 431)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 431, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 431, 8159, NPC, 18, 1408);
		else
			RunQuestExchange(UID,1204);
			SaveEvent(UID, 2183);
		end
	end
end

if (EVENT == 302) then
	SelectMsg(UID, 4, 463, 8167, NPC, 22, 303, 23, -1);
end

if (EVENT == 303) then
	QuestStatus = GetQuestStatus(UID, 463)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2230);
	end
end

if (EVENT == 305) then
	QuestStatus = GetQuestStatus(UID, 463)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 463, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 463, 8167, NPC, 18, 308);
		else
			SaveEvent(UID, 2232);
		end
	end
end

if (EVENT == 307) then
	QuestStatus = GetQuestStatus(UID, 463)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 463, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 463, 8167, NPC, 18, 308);
		else
			SelectMsg(UID, 4, 463, 8169, NPC, 41, 309, 23, -1);
		end
	end
end

if (EVENT == 308) then
	ShowMap(UID, 29);
end

if (EVENT == 309) then
	QuestStatus = GetQuestStatus(UID, 463)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 463, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 463, 8167, NPC, 18, 308);
		else
			RunQuestExchange(UID,21001);
			SaveEvent(UID, 2231);
		end
	end
end

if (EVENT == 9900) then
	SelectMsg(UID, 4, 395, 8764, NPC, 22, 9901, 23, -1);
end

if (EVENT == 9901) then
	QuestStatus = GetQuestStatus(UID, 395)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or  Class == 5 or Class == 6) then
			SaveEvent(UID, 1385);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 1388);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 1391);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 1394);
		end
	end
end

if(EVENT == 9905) then
	QuestStatus = GetQuestStatus(UID, 395)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 395, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 395, 8764, NPC, 18, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or  Class == 5 or Class == 6) then
			SaveEvent(UID, 1386);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 1389);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 1392);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 1395);
			end
		end
	end
end

if(EVENT == 9903) then
	QuestStatus = GetQuestStatus(UID, 395)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 395, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 395, 8764, NPC, 18, -1);
		else
			SelectMsg(UID, 4, 395, 8764, NPC, 41, 9904, 27, -1);
		end
	end
end

if(EVENT == 9904) then
	QuestStatus = GetQuestStatus(UID, 395)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 395, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 395, 8764, NPC, 18, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or  Class == 5 or Class == 6) then
			RunQuestExchange(UID, 11126)
			SaveEvent(UID, 1387);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID, 11127)
			SaveEvent(UID, 1390);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID, 11128)
			SaveEvent(UID, 1393);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID, 11129)
			SaveEvent(UID, 1396);
			end
		end
	end
end

if (EVENT == 9800) then 
	SelectMsg(UID, 4, 398, 8766, NPC, 22, 9801, 23, -1);
end

if (EVENT == 9801) then
	QuestStatus = GetQuestStatus(UID, 398)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or  Class == 5 or Class == 6) then
			SaveEvent(UID, 1421);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 1424);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 1427);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 1430);
		end
	end
end

if(EVENT == 9805) then
	QuestStatus = GetQuestStatus(UID, 398)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 398, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 398, 8766, NPC, 18, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or  Class == 5 or Class == 6) then
			SaveEvent(UID, 1422);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 1425);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 1428);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 1431);
			end
		end
	end
end

if(EVENT == 9803) then
	QuestStatus = GetQuestStatus(UID, 398)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 398, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 398, 8766, NPC, 18, -1);
		else
			SelectMsg(UID, 4, 398, 8766, NPC, 41, 9804, 27, -1);
		end
	end
end

if (EVENT == 9804) then
	QuestStatus = GetQuestStatus(UID, 398)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 398, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 398, 8766, NPC, 18, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,11130);
			SaveEvent(UID, 1423);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,11131);
			SaveEvent(UID, 1426);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,11132);
			SaveEvent(UID, 1429);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,11133);
			SaveEvent(UID, 1429);
			end
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
			SaveEvent(UID, 7163);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 7168);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 7173);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 7178);
	end
end

if (EVENT == 215) then
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 7161);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 7166);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 7171);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 7178);
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
	ShowMap(UID, 706);
end

if (EVENT == 221) then
	ShowMap(UID, 708);
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
	ShowMap(UID, 706);
end

if (EVENT == 223) then
	ShowMap(UID, 710);
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
	ShowMap(UID, 712);
end

if (EVENT == 225) then
	ShowMap(UID, 714);
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
	ShowMap(UID, 714);
end

if (EVENT == 227) then
	ShowMap(UID, 710);
end

if (EVENT == 203) then
	QuestStatus = GetQuestStatus(UID, 448)	
	QuestStatus2 = GetQuestStatus(UID, 449)
	QuestStatus3 = GetQuestStatus(UID, 450)
	QuestStatus4 = GetQuestStatus(UID, 451)
		if(QuestStatus == 2 or QuestStatus2 == 2 or QuestStatus3 == 2 or QuestStatus4 == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,701);
			SaveEvent(UID, 7162);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,702);
			SaveEvent(UID, 7167);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,703);
			SaveEvent(UID, 7172);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,704);
			SaveEvent(UID, 7177);
		end
	end
end
---------------------------------------------------------------------------------------------------------------------
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
			SaveEvent(UID, 2364);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 2369);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 2374);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 2379);
	end
end

if (EVENT == 515) then
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 2362);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 2367);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 2372);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 2377);
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
	ShowMap(UID, 706);--706
end

if (EVENT == 521) then
	ShowMap(UID, 708);--708
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
	ShowMap(UID, 706);--706
end

if (EVENT == 523) then
	ShowMap(UID, 712);--710
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
	ShowMap(UID, 712);--712
end

if (EVENT == 525) then
	ShowMap(UID, 714);--714
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
	ShowMap(UID, 714);--714
end

if (EVENT == 527) then
	ShowMap(UID, 710);--710
end

if (EVENT == 503) then
	QuestStatus = GetQuestStatus(UID, 511)	
	QuestStatus2 = GetQuestStatus(UID, 512)
	QuestStatus3 = GetQuestStatus(UID, 513)
	QuestStatus4 = GetQuestStatus(UID, 514)
		if(QuestStatus == 2 or QuestStatus2 == 2 or QuestStatus3 == 2 or QuestStatus4 == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,1701);
			SaveEvent(UID, 2363);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,1702);
			SaveEvent(UID, 2368);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,1703);
			SaveEvent(UID, 2373);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,1704);
			SaveEvent(UID, 2378);
		end
	end
end