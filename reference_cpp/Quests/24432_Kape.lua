-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24432;

if (EVENT == 160) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 8255, NPC, 10, 163);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 8257, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1000) then
	SaveEvent(UID, 2073);
end

if (EVENT == 1002) then
	SelectMsg(UID, 4, 413, 798, NPC, 22, 1003, 23, -1);
end

if (EVENT == 1003) then
	QuestStatus = GetQuestStatus(UID, 413)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2074);
	end
end

if (EVENT == 1010) then
	QuestStatus = GetQuestStatus(UID, 413)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 413, 1);
		if (MonsterCount < 19) then
			SelectMsg(UID, 2, 413, 798, NPC, 18, 1007);
		else
			SaveEvent(UID, 2076);
		end
	end
end

if (EVENT == 1006) then
	QuestStatus = GetQuestStatus(UID, 413)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 413, 1);
		if (MonsterCount < 19) then
			SelectMsg(UID, 2, 413, 798, NPC, 18, 1007);
		else
			SelectMsg(UID, 4, 413, 798, NPC, 41, 1008, 27, -1);
		end
	end
end

if (EVENT == 1007) then
	ShowMap(UID, 107);
end

if (EVENT == 1008) then
	QuestStatus = GetQuestStatus(UID, 413)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 413, 1);
		if (MonsterCount < 19) then
			SelectMsg(UID, 2, 413, 798, NPC, 18, 1007);
		else
			RunQuestExchange(UID,1195);
			SaveEvent(UID, 2075); 
		end
	end
end

if (EVENT == 8752) then
	SelectMsg(UID, 4, 143, 8151, NPC, 22, 8753, 23, -1);
end

if (EVENT == 8753) then
	QuestStatus = GetQuestStatus(UID, 143)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8680);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8685);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8690);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8695);
		end
	end
end

if (EVENT == 8760) then
	QuestStatus = GetQuestStatus(UID, 143)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 143, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 143, 8151, NPC, 18, 8757);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8682);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8687);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8692);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8697);
			end
		end
	end
end

if (EVENT == 8756) then
	QuestStatus = GetQuestStatus(UID, 143)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 143, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 143, 8151, NPC, 18, 8757);
		else
			SelectMsg(UID, 5, 143, 8151, NPC, 41, 8758,23, -1);
		end
	end
end

if (EVENT == 8757) then
	ShowMap(UID, 107);
end

if (EVENT == 8758) then
	QuestStatus = GetQuestStatus(UID, 143)	
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 143, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 143, 8151, NPC, 18, 8757);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,957,STEP,1);
			SaveEvent(UID, 8681);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,958,STEP,1);
			SaveEvent(UID, 8686);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,959,STEP,1);
			SaveEvent(UID, 8691);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,960,STEP,1);
			SaveEvent(UID, 8696);
			end
		end
	end
end

if (EVENT == 8172) then
	SelectMsg(UID, 2, 175, 8390, NPC, 10, 8175);
end

if (EVENT == 8175) then
	SelectMsg(UID, 4, 175, 8391, NPC, 22, 8173, 23, -1);
end

if (EVENT == 8173) then
	QuestStatus = GetQuestStatus(UID, 175)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8806);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8811);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8816);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8821);
		end
	end
end

if (EVENT == 8180) then
	QuestStatus = GetQuestStatus(UID, 175)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 175, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 175, 8376, NPC, 18, 8177);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8808);
			EVENT = 8181
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8813);
			EVENT = 8181
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8818);
			EVENT = 8181
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8823);
			EVENT = 8181
			end
		end
	end
end

if (EVENT == 8181) then
	SelectMsg(UID, 2, 175, 8375, NPC, 3007, -1);
end

if (EVENT == 8176) then
	QuestStatus = GetQuestStatus(UID, 175)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 175, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 175, 8376, NPC, 18, 8177);
		else
			SelectMsg(UID, 4, 175, 8392, NPC, 41, 8178, 27, -1);
		end
	end
end

if (EVENT == 8177) then
	ShowMap(UID, 540);
end

if (EVENT == 8178) then
	QuestStatus = GetQuestStatus(UID, 175)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 175, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 175, 8376, NPC, 18, 8177);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,981);
			SaveEvent(UID, 8807);
		elseif (Class == 2 or Class == 7 or Class == 8) then      
			RunQuestExchange(UID,982);
			SaveEvent(UID, 8812);
		elseif (Class == 3 or Class == 9 or Class == 10) then    
			RunQuestExchange(UID,983);
			SaveEvent(UID, 8817);
		elseif (Class == 4 or Class == 11 or Class == 12) then     
			RunQuestExchange(UID,984);
			SaveEvent(UID, 8822);
			end
		end	 
	end
end

if (EVENT == 9432) then
	SelectMsg(UID, 4, 206, 8760, NPC, 22, 9433, 23, -1);
end

if (EVENT == 9433) then
	QuestStatus = GetQuestStatus(UID, 206)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9472);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9477);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9482);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9487);
		end
	end
end

if (EVENT == 9440) then
	QuestStatus = GetQuestStatus(UID, 206)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 206, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 206, 8376, NPC, 18, 9437);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9474);
			EVENT = 9441
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9479);
			EVENT = 9441
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9484);
			EVENT = 9441
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9489);
			EVENT = 9441
			end
		end
	end
end

if (EVENT == 9441) then
	SelectMsg(UID, 2, 206, 8758, NPC, 3007, -1);
end

if (EVENT == 9436) then
	QuestStatus = GetQuestStatus(UID, 206)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 206, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 206, 8376, NPC, 18, 9437);
		else
			SelectMsg(UID, 5, 206, 8392, NPC, 41, 9438, 27, -1);
		end
	end
end

if (EVENT == 9437) then
	ShowMap(UID, 619);
end

if (EVENT == 9438) then
	QuestStatus = GetQuestStatus(UID, 206)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 206, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 206, 8376, NPC, 18, 9437);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,1106,STEP,1);
			SaveEvent(UID, 9473);
		elseif (Class == 2 or Class == 7 or Class == 8) then      
			RunQuestExchange(UID,1107,STEP,1);
			SaveEvent(UID, 9478);
		elseif (Class == 3 or Class == 9 or Class == 10) then    
			RunQuestExchange(UID,1108,STEP,1);
			SaveEvent(UID, 9483);
		elseif (Class == 4 or Class == 11 or Class == 12) then     
			RunQuestExchange(UID,1109,STEP,1);
			SaveEvent(UID, 9488);
			end
		end	 
	end
end

if (EVENT == 9452) then
	SelectMsg(UID, 4, 227, 8762, NPC, 22, 9453, 23, -1);
end

if (EVENT == 9453) then
	QuestStatus = GetQuestStatus(UID, 227)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9514);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9519);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9524);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9529);
		end
	end
end

if (EVENT == 9460) then
	QuestStatus = GetQuestStatus(UID, 227)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 227, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 227, 8376, NPC, 18, 9457);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9516);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9521);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9526);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9531);
			end
		end
	end
end

if (EVENT == 9456) then
	QuestStatus = GetQuestStatus(UID, 227)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 227, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 227, 8376, NPC, 18, 9457);
		else
			SelectMsg(UID, 5, 227, 8392, NPC, 41, 9458, 27, -1);
		end
	end
end

if (EVENT == 9457) then
	ShowMap(UID, 623);
end

if (EVENT == 9458) then
	QuestStatus = GetQuestStatus(UID, 227)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 227, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 227, 8376, NPC, 18, 9457);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,1114,STEP,1);
			SaveEvent(UID, 9515);
		elseif (Class == 2 or Class == 7 or Class == 8) then      
			RunQuestExchange(UID,1115,STEP,1);
			SaveEvent(UID, 9520);
		elseif (Class == 3 or Class == 9 or Class == 10) then    
			RunQuestExchange(UID,1116,STEP,1);
			SaveEvent(UID, 9525);
		elseif (Class == 4 or Class == 11 or Class == 12) then     
			RunQuestExchange(UID,1117,STEP,1);
			SaveEvent(UID, 9530);
			end
		end	 
	end
end

if (EVENT == 9062) then
	SelectMsg(UID, 2, 265, 8420, NPC, 10, 9070);
end

if (EVENT == 9070) then
	SelectMsg(UID, 4, 265, 8421, NPC, 22, 9063, 23, -1);
end

if (EVENT == 9063) then
	QuestStatus = GetQuestStatus(UID, 265)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9118);
	end
end

if (EVENT == 9065) then
	QuestStatus = GetQuestStatus(UID, 265)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 265, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 265, 8424, NPC, 18, 9068);
		else
			SelectMsg(UID, 2, 265, 8423, NPC, 3014, -1);
			SaveEvent(UID, 9120);
		end
	end
end

if (EVENT == 9067) then
	QuestStatus = GetQuestStatus(UID, 265)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 265, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 265, 8424, NPC, 18, 9068);
		else
			SelectMsg(UID, 4, 265, 8425, NPC, 41, 9069, 27, -1);
		end
	end
end

if (EVENT == 9068) then
	ShowMap(UID, 330);
end

if (EVENT == 9069) then
	QuestStatus = GetQuestStatus(UID, 265)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 265, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 265, 8424, NPC, 18, 9068);
		else
			RunQuestExchange(UID,1042);
			SaveEvent(UID, 9119);
		end
	end
end

if (EVENT == 9322) then
	SelectMsg(UID, 4, 267, 8676, NPC, 22, 9323, 23, -1);
end

if (EVENT == 9323) then
	QuestStatus = GetQuestStatus(UID, 267)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9340);
	end
end

if (EVENT == 9325) then
	QuestStatus = GetQuestStatus(UID, 267)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 267, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 267, 8551, NPC, 18, 9328);
		else
			SaveEvent(UID, 9342);
		end
	end
end

if (EVENT == 9327) then
	QuestStatus = GetQuestStatus(UID, 267)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 267, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 267, 8551, NPC, 18, 9328);
		else
			SelectMsg(UID, 4, 267, 8425, NPC, 41, 9329, 27, -1);
		end
	end
end

if (EVENT == 9328) then
	ShowMap(UID, 37);
end

if (EVENT == 9329) then
	QuestStatus = GetQuestStatus(UID, 267)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 267, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 267, 8551, NPC, 18, 9328);
		else
			RunQuestExchange(UID,1089);
			SaveEvent(UID, 9341);
		end
	end
end


if (EVENT == 9342) then
	SelectMsg(UID, 4, 269, 8680, NPC, 22, 9343, 23, -1);
end

if (EVENT == 9343) then
	QuestStatus = GetQuestStatus(UID, 269)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9364);
	end
end

if (EVENT == 9345) then
	QuestStatus = GetQuestStatus(UID, 269)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 269, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 269, 8565, NPC, 18, 9348);
		else
			SaveEvent(UID, 9366);
		end
	end
end

if (EVENT == 9347) then
	QuestStatus = GetQuestStatus(UID, 269)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 269, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 269, 8565, NPC, 18, 9348);
		else
			SelectMsg(UID, 4, 269, 8425, NPC, 41, 9349, 27, -1);
		end
	end
end

if (EVENT == 9348) then
	ShowMap(UID, 704);
end

if (EVENT == 9349) then
	QuestStatus = GetQuestStatus(UID, 269)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 269, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 269, 8565, NPC, 18, 9348);
		else
			RunQuestExchange(UID,1092);
			SaveEvent(UID, 9365);
		end
	end
end

if (EVENT == 9150) then
	SelectMsg(UID, 4, 291, 8435, NPC, 3018, 9152, 3019, -1);
end

if (EVENT == 9152) then
	QuestStatus = GetQuestStatus(UID, 291)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9256);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9261);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9266);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9271);
		end
	end
end

if (EVENT == 9153) then
	QuestStatus = GetQuestStatus(UID, 291)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 291, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 291, 8437, NPC, 18, 9157);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9258);
			EVENT = 9154
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9263);
			EVENT = 9154
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9268);
			EVENT = 9154
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9273);
			EVENT = 9154
			end
		end
	end
end

if (EVENT == 9154) then
	SelectMsg(UID, 2, 291, 8434, NPC, 57, -1);
end

if (EVENT == 9155) then
	QuestStatus = GetQuestStatus(UID, 291)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 291, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 291, 8437, NPC, 18, 9157);
		else
			SelectMsg(UID, 4, 291, 8438, NPC, 41, 9158, 27, -1);
		end
	end
end

if (EVENT == 9157) then
	ShowMap(UID, 17);
end

if (EVENT == 9158) then
	QuestStatus = GetQuestStatus(UID, 291)
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then	
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 291, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 291, 8437, NPC, 18, 9157);
		else
    Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,1052);
			SaveEvent(UID, 9257);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,1053);
			SaveEvent(UID, 9262);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,1054);
			SaveEvent(UID, 9267);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,1055);
			SaveEvent(UID, 9272);
			end
		end	 
	end
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 417, 842, NPC, 22, 1103, 23, -1);
end

if (EVENT == 1103) then
	QuestStatus = GetQuestStatus(UID, 417)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
		SaveEvent(UID, 2098);
	end
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 417)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 417, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 417, 842, NPC, 18, 1107);
		else
			SaveEvent(UID, 2100);
		end
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 417)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 417, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 417, 842, NPC, 18, 1107);
		else
			SelectMsg(UID, 4, 417, 842, NPC, 41, 1108, 23, -1);
		end
	end
end

if (EVENT == 1107) then
	ShowMap(UID, 29);
end

if (EVENT == 1108) then
	QuestStatus = GetQuestStatus(UID, 417)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 417, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 417, 842, NPC, 18, 1107);
		else
			RunQuestExchange(UID,1197);
			SaveEvent(UID, 2099);
		end
	end
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 421, 1199, NPC, 22, 1203, 23, -1);
end

if (EVENT == 1203) then
	QuestStatus = GetQuestStatus(UID, 421)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2122);
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 421)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 421, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 421, 1199, NPC, 18, 1207);
		else
			SaveEvent(UID, 2124);
		end
	end
end

if (EVENT == 1206) then
	QuestStatus = GetQuestStatus(UID, 421)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 421, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 421, 1199, NPC, 18, 1207);
		else
			SelectMsg(UID, 4, 421, 1199, NPC, 41, 1208, 23, -1);
		end
	end
end
	
if (EVENT == 1207) then
	ShowMap(UID, 29);
end

if (EVENT == 1208) then
	QuestStatus = GetQuestStatus(UID, 421)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 421, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 421, 1199, NPC, 18, 1207);
		else
			RunQuestExchange(UID,1199);
			SaveEvent(UID, 2123);
		end
	end
end

if (EVENT == 202) then
	SelectMsg(UID, 4, 475, 8169, NPC, 22, 203, 23, -1);
end

if (EVENT == 203) then
	QuestStatus = GetQuestStatus(UID, 475)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2302);
	end
end

if (EVENT == 205) then
	QuestStatus = GetQuestStatus(UID, 475)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 475, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 475, 8169, NPC, 10, 206);
		else
			SaveEvent(UID, 2304);
		end
	end
end

if (EVENT == 207) then
	QuestStatus = GetQuestStatus(UID, 475)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 475, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 475, 8169, NPC, 10, 206);
		else
			SelectMsg(UID, 4, 475, 8169, NPC, 41, 208, 23, -1);
		end
	end
end
	
if (EVENT == 208) then
	QuestStatus = GetQuestStatus(UID, 475)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 475, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 475, 8169, NPC, 10, 206);
		else
			RunQuestExchange(UID,21042);
			SaveEvent(UID, 2303);
		end
	end
end

if (EVENT == 302) then
	SelectMsg(UID, 4, 478, 8676, NPC, 22, 303, 23, -1);
end

if (EVENT == 303) then
	QuestStatus = GetQuestStatus(UID, 478)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2326);
	end
end

if (EVENT == 305) then
	QuestStatus = GetQuestStatus(UID, 478)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2328);
	end
end

if (EVENT == 307) then
	QuestStatus = GetQuestStatus(UID, 478)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 478, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 478, 8676, NPC, 10, 306);
		else
			SelectMsg(UID, 4, 478, 8676, NPC, 41, 308, 23, -1);
		end
	end
end
	
if (EVENT == 308) then
	QuestStatus = GetQuestStatus(UID, 478)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 478, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 478, 8676, NPC, 10, 306);
		else
			RunQuestExchange(UID,11089);
			SaveEvent(UID, 2327);
		end
	end
end

if (EVENT == 402) then
	SelectMsg(UID, 4, 481, 8680, NPC, 22, 403, 23, -1);
end

if (EVENT == 403) then
	QuestStatus = GetQuestStatus(UID, 481)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2350);
	end
end

if (EVENT == 405) then
	QuestStatus = GetQuestStatus(UID, 481)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 481, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 481, 8680, NPC, 10, 406);
		else
			SaveEvent(UID, 2352);
		end
	end
end

if (EVENT == 407) then
	QuestStatus = GetQuestStatus(UID, 481)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 481, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 481, 8680, NPC, 10, 406);
		else
			SelectMsg(UID, 4, 481, 8680, NPC, 41, 408, 23, -1);
		end
	end
end
	
if (EVENT == 408) then
	QuestStatus = GetQuestStatus(UID, 481)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 481, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 481, 8680, NPC, 10, 406);
		else
			RunQuestExchange(UID,11092);
			SaveEvent(UID, 2351);
		end
	end
end

if (EVENT == 8952) then
	SelectMsg(UID, 4, 149, 842, NPC, 22, 8953, 23, -1);
end

if (EVENT == 8953) then
	QuestStatus = GetQuestStatus(UID, 149)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8764);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8769);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8774);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8779);
		end
	end
end

if (EVENT == 8960) then
	QuestStatus = GetQuestStatus(UID, 149)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 149, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 149, 842, NPC, 18, 8957);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8766);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8771);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8776);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8781);
			end
		end
	end
end

if (EVENT == 8956) then
	QuestStatus = GetQuestStatus(UID, 149)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 149, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 149, 842, NPC, 18, 8957);
		else
			SelectMsg(UID, 4, 149, 842, NPC, 41, 8958, 23, -1);
		end
	end
end

if (EVENT == 8958) then
	QuestStatus = GetQuestStatus(UID, 149)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 149, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 149, 842, NPC, 18, 8957);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,973);
			SaveEvent(UID, 8765);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,974);
			SaveEvent(UID, 8770);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,975);
			SaveEvent(UID, 8775);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,976);
			SaveEvent(UID, 8780);
			end
		end
	end
end

if (EVENT == 9900) then
	SelectMsg(UID, 4, 392, 8760, NPC, 22, 9901, 23, -1);
end

if(EVENT == 9901) then
	QuestStatus = GetQuestStatus(UID, 392)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 1301);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 1304);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 1307);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 1310);
		end
	end
end

if(EVENT == 9905) then
	QuestStatus = GetQuestStatus(UID, 392)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 392, 1);
		if (MonsterCount1 < 3) then
			SelectMsg(UID, 2, 392, 8760, NPC, 10, 217);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 1302);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 1305);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 1308);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 1311);
			end
		end
	end
end

if(EVENT == 9903) then
	QuestStatus = GetQuestStatus(UID, 392)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 392, 1);
		if (MonsterCount1 < 3) then
			SelectMsg(UID, 2, 392, 8760, NPC, 10, 217);
		else
			SelectMsg(UID, 4, 392, 8760, NPC, 41, 9906, 27, -1);
		end
	end
end

if (EVENT == 9906) then
	QuestStatus = GetQuestStatus(UID, 392)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 392, 1);
		if (MonsterCount1 < 3) then
			SelectMsg(UID, 2, 392, 8760, NPC, 10, 217);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,11106);
			SaveEvent(UID, 1303);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,11107);
			SaveEvent(UID, 1306);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,11108);
			SaveEvent(UID, 1309);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,11110);
			SaveEvent(UID, 1312);
			end
		end
	end
end

if (EVENT == 9800) then
	SelectMsg(UID, 4, 396, 8762, NPC, 22, 9801, 23, -1);
end

if(EVENT == 9801) then
	QuestStatus = GetQuestStatus(UID, 396)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 1397);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 1400);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 1403);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 1406);
		end
	end
end

if(EVENT == 9805) then
	QuestStatus = GetQuestStatus(UID, 396)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 396, 1);
		if (MonsterCount1 < 20) then
			SelectMsg(UID, 2, 396, 8762, NPC, 10, 217);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 1398);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 1401);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 1404);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 1407);
			end
		end
	end
end

if(EVENT == 9803) then
	QuestStatus = GetQuestStatus(UID, 396)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 396, 1);
		if (MonsterCount1 < 20) then
			SelectMsg(UID, 2, 396, 8762, NPC, 10, 217);
		else
			SelectMsg(UID, 4, 396, 8762, NPC, 41, 9806, 27, -1);
		end
	end
end

if (EVENT == 9806) then
	QuestStatus = GetQuestStatus(UID, 396)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 396, 1);
		if (MonsterCount1 < 20) then
			SelectMsg(UID, 2, 396, 8762, NPC, 10, 217);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,11114);
			SaveEvent(UID, 1399);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,11115);
			SaveEvent(UID, 1402);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,11116);
			SaveEvent(UID, 1405);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,11117);
			SaveEvent(UID, 1408);
			end
		end
	end
end