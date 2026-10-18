-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24431;

if (EVENT == 155) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 8255, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 8257, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 8252) then
	SelectMsg(UID, 2, 218, 8232, NPC, 10, 8260);
end

if (EVENT == 8260) then
	SelectMsg(UID, 4, 218, 8233, NPC, 22, 8253, 23, -1);
end

if (EVENT == 8253) then
	QuestStatus = GetQuestStatus(UID, 218)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9010);
	end
end

if (EVENT == 8255) then
	QuestStatus = GetQuestStatus(UID, 218)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 218, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 218, 8263, NPC, 18, 8258);
		else
			SelectMsg(UID, 2, 218, 8262, NPC, 3007, -1);
			SaveEvent(UID, 9012);
		end
	end
end

if (EVENT == 8257) then
	QuestStatus = GetQuestStatus(UID, 218)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 218, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 218, 8263, NPC, 18, 8258);
		else
			SelectMsg(UID, 5, 218, 8264, NPC, 41, 8259, 27, -1);
		end
	end
end

if (EVENT == 8258) then
	ShowMap(UID, 182);
end

if (EVENT == 8259) then
	QuestStatus = GetQuestStatus(UID, 218)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 218, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 218, 8263, NPC, 18, 8258);
		else
			RunQuestExchange(UID,1003,STEP,1);
			SaveEvent(UID, 9011);
		end
	end
end

if (EVENT == 8352) then
	SelectMsg(UID, 2, 223, 8272, NPC, 10, 8360);
end

if (EVENT == 8360) then
	SelectMsg(UID, 4, 223, 8273, NPC, 22, 8353, 23, -1);
end

if (EVENT == 8353) then
	QuestStatus = GetQuestStatus(UID, 223)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9022);
	end
end

if (EVENT == 8355) then
	QuestStatus = GetQuestStatus(UID, 223)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 223, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 223, 8263, NPC, 18, 8358);
		else
			SelectMsg(UID, 2, 223, 8262, NPC, 3007, -1);
			SaveEvent(UID, 9024);
		end
	end
end

if (EVENT == 8357) then
	QuestStatus = GetQuestStatus(UID, 223)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 223, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 223, 8263, NPC, 18, 8358);
		else
			SelectMsg(UID, 5, 223, 8264, NPC, 41, 8359, 27, -1);
		end
	end
end

if (EVENT == 8358) then
	ShowMap(UID, 544);
end

if (EVENT == 8359) then
	QuestStatus = GetQuestStatus(UID, 223)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 223, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 223, 8263, NPC, 18, 8358);
		else
			RunQuestExchange(UID,1005,STEP,1);
			SaveEvent(UID, 9023);

		end
	end
end

if (EVENT == 8660) then
	SelectMsg(UID, 2, 233, 8284, NPC, 3002, 8661);
end

if (EVENT == 8661) then
	SelectMsg(UID, 4, 233, 8285, NPC, 3018, 8662, 3019, -1);
end

if (EVENT == 8662) then
	QuestStatus = GetQuestStatus(UID, 233)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9058);
	end
end

if (EVENT == 8663) then
	QuestStatus = GetQuestStatus(UID, 233)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 233, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 233, 8289, NPC, 18, 8669);
		else
			SelectMsg(UID, 2, 233, 8288, NPC, 4080, -1);
			SaveEvent(UID, 9060);
		end
	end
end

if (EVENT == 8665) then
	QuestStatus = GetQuestStatus(UID, 233)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 233, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 233, 8289, NPC, 18, 8669);
		else
			SelectMsg(UID, 4, 233, 8290, NPC, 41, 8667, 27, -1);
		end
	end
end

if (EVENT == 8669) then
	ShowMap(UID, 506);
end

if (EVENT == 8667) then
	QuestStatus = GetQuestStatus(UID, 233)
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then	
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 233, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 233, 8289, NPC, 18, 8669);
		else
			RunQuestExchange(UID,944);
			SaveEvent(UID, 9059);
		end
	end
end

if (EVENT == 9182) then
	SelectMsg(UID, 2, 237, 8303, NPC, 10, 9190);
end

if (EVENT == 9190) then
	SelectMsg(UID, 4, 237, 8304, NPC, 22, 9183, 23, -1);
end

if (EVENT == 9183) then
	QuestStatus = GetQuestStatus(UID, 237)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9070);
	end
end

if (EVENT == 9185) then
	QuestStatus = GetQuestStatus(UID, 237)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SelectMsg(UID, 2, 237, 8262, NPC, 3007, -1);
			SaveEvent(UID, 9072);
	end
end

if (EVENT == 9187) then
	QuestStatus = GetQuestStatus(UID, 237)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 237, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 237, 8263, NPC, 18, 9188);
		else
			SelectMsg(UID, 5, 237, 8264, NPC, 41, 9189, 27, -1);
		end
	end
end

if (EVENT == 9188) then
	ShowMap(UID, 547);
end

if (EVENT == 9189) then
	QuestStatus = GetQuestStatus(UID, 237)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 237, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 237, 8263, NPC, 18, 9188);
		else
			RunQuestExchange(UID,946,STEP,1);
			SaveEvent(UID, 9071);
		end
	end
end

if (EVENT == 9022) then
	SelectMsg(UID, 2, 241, 8448, NPC, 10, 9030);
end

if (EVENT == 9030) then
	SelectMsg(UID, 4, 241, 8448, NPC, 22, 9023, 23, -1);
end

if (EVENT == 9023) then
	QuestStatus = GetQuestStatus(UID, 241)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9094);
	end
end

if (EVENT == 9025) then
	QuestStatus = GetQuestStatus(UID, 241)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 241, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 241, 8448, NPC, 18, 9028);
		else
			SelectMsg(UID, 2, 241, 8262, NPC, 3007, -1);
			SaveEvent(UID, 9096);
		end
	end
end

if (EVENT == 9027) then
	QuestStatus = GetQuestStatus(UID, 241)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 241, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 241, 8448, NPC, 18, 9028);
		else
			SelectMsg(UID, 4, 241, 8448, NPC, 41, 9029, 27, -1);
		end
	end
end

if (EVENT == 9028) then
	ShowMap(UID, 587);
end

if (EVENT == 9029) then
	QuestStatus = GetQuestStatus(UID, 241)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 241, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 241, 8448, NPC, 18, 9028);
		else
			RunQuestExchange(UID,1007);
			SaveEvent(UID, 9095);
		end
	end
end

if (EVENT == 9042) then
	SelectMsg(UID, 2, 263, 8454, NPC, 10, 9050);
end

if (EVENT == 9050) then
	SelectMsg(UID, 4, 263, 8455, NPC, 22, 9043, 23, -1);
end

if (EVENT == 9043) then
	QuestStatus = GetQuestStatus(UID, 263)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9106);
	end
end

if (EVENT == 9045) then
	QuestStatus = GetQuestStatus(UID, 263)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 263, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 263, 8263, NPC, 18, 9048);
		else
			SelectMsg(UID, 2, 263, 8262, NPC, 3007, -1);
			SaveEvent(UID, 9108);
		end
	end
end

if (EVENT == 9047) then
	QuestStatus = GetQuestStatus(UID, 263)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 263, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 263, 8263, NPC, 18, 9048);
		else
			SelectMsg(UID, 4, 263, 8264, NPC, 41, 9049, 27, -1);
		end
	end
end

if (EVENT == 9048) then
	ShowMap(UID, 551);
end

if (EVENT == 9049) then
	QuestStatus = GetQuestStatus(UID, 263)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 263, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 263, 8263, NPC, 18, 9048);
		else
			RunQuestExchange(UID,950);
			SaveEvent(UID, 9107);
		end
	end
end

if (EVENT == 9082) then
	SelectMsg(UID, 2, 285, 8459, NPC, 10, 9085);
end

if (EVENT == 9085) then
	SelectMsg(UID, 4, 285, 8460, NPC, 22, 9083, 23, -1);
end

if (EVENT == 9083) then
	QuestStatus = GetQuestStatus(UID, 285)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9130);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9135);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9140);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9145);
		end
	end
end

if (EVENT == 9090) then
	QuestStatus = GetQuestStatus(UID, 285)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 285, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 285, 8462, NPC, 18, 9087);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9132);
			EVENT = 9091
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9137);
			EVENT = 9091
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9142);
			EVENT = 9091
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9147);
			EVENT = 9091
			end
		end
	end
end

if (EVENT == 9091) then
	SelectMsg(UID, 2, 285, 8461, NPC, 3002, -1);
end

if (EVENT == 9086) then
	QuestStatus = GetQuestStatus(UID, 285)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 285, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 285, 8462, NPC, 18, 9087);
		else
			SelectMsg(UID, 4, 285, 8463, NPC, 41, 9088, 27, -1);
		end
	end
end

if (EVENT == 9087) then
	ShowMap(UID, 518);
end

if (EVENT == 9088) then
	QuestStatus = GetQuestStatus(UID, 285)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 285, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 285, 8462, NPC, 18, 9087);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,932);
			SaveEvent(UID, 9131);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,933);
			SaveEvent(UID, 9136);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,934);
			SaveEvent(UID, 9141);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,935);
			SaveEvent(UID, 9146);
			end
		end 
	end
end

if (EVENT == 9102) then
	SelectMsg(UID, 2, 287, 8473, NPC, 10, 9105);
end

if (EVENT == 9105) then
	SelectMsg(UID, 4, 287, 8474, NPC, 22, 9103, 23, -1);
end

if (EVENT == 9103) then
	QuestStatus = GetQuestStatus(UID, 287)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9172);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9177);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9182);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9187);
		end
	end
end

if (EVENT == 9110) then
	QuestStatus = GetQuestStatus(UID, 287)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9174);
			EVENT = 9111
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9179);
			EVENT = 9111
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9184);
			EVENT = 9111
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9189);
			EVENT = 9111
		end
	end
end

if (EVENT == 9111) then
	SelectMsg(UID, 2, 287, 8461, NPC, 3002, -1);
end

if (EVENT == 9106) then
	QuestStatus = GetQuestStatus(UID, 287)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 287, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 287, 8462, NPC, 18, 9107);
		else
			SelectMsg(UID, 4, 287, 8475, NPC, 41, 9108, 27, -1);
		end
	end
end

if (EVENT == 9107) then
	ShowMap(UID, 553);
end

if (EVENT == 9108) then
	QuestStatus = GetQuestStatus(UID, 287)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 287, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 287, 8462, NPC, 18, 9107);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,912);
			SaveEvent(UID, 9173);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,913);
			SaveEvent(UID, 9178);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,914);
			SaveEvent(UID, 9183);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,915);
			SaveEvent(UID, 9188);
			end
		end
	end
end

if (EVENT == 9122) then
	SelectMsg(UID, 2, 289, 8480, NPC, 10, 9125);
end

if (EVENT == 9125) then
	SelectMsg(UID, 4, 289, 8481, NPC, 22, 9123, 23, -1);
end

if (EVENT == 9123) then
	QuestStatus = GetQuestStatus(UID, 289)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9214);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9219);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9224);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9229);
		end
	end
end

if (EVENT == 9130) then
	QuestStatus = GetQuestStatus(UID, 289)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 289, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 289, 8462, NPC, 18, 9127);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9216);
			EVENT = 9131
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9221);
			EVENT = 9131
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9226);
			EVENT = 9131
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9231);
			EVENT = 9131
			end
		end
	end
end

if (EVENT == 9131) then
	SelectMsg(UID, 2, 289, 8461, NPC, 29, -1);
end

if (EVENT == 9126) then
	QuestStatus = GetQuestStatus(UID, 289)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 289, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 289, 8462, NPC, 18, 9127);
		else
			SelectMsg(UID, 4, 289, 8482, NPC, 41, 9128, 27, -1);
		end
	end
end

if (EVENT == 9127) then
	ShowMap(UID, 555);
end

if (EVENT == 9128) then
	QuestStatus = GetQuestStatus(UID, 289)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 289, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 289, 8462, NPC, 18, 9127);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,1044);
			SaveEvent(UID, 9215);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,1045);
			SaveEvent(UID, 9220);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,1046);
			SaveEvent(UID, 9225);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,1047);
			SaveEvent(UID, 9230);
			end
		end
	end
end


if (EVENT == 9162) then
	SelectMsg(UID, 2, 293, 8486, NPC, 10, 9165);
end

if (EVENT == 9165) then
	SelectMsg(UID, 4, 293, 8487, NPC, 22, 9163, 23, -1);
end

if (EVENT == 9163) then
	QuestStatus = GetQuestStatus(UID, 293)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9298);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9303);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9308);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9313);
		end
	end
end

if (EVENT == 9170) then
	QuestStatus = GetQuestStatus(UID, 293)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 293, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 293, 8462, NPC, 18, 9167);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9300);
			EVENT = 9171
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9305);
			EVENT = 9171
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9310);
			EVENT = 9171
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9315);
			EVENT = 9171
			end
		end
	end
end

if (EVENT == 9171) then
	SelectMsg(UID, 2, 293, 8461, NPC, 29, -1);
end

if (EVENT == 9166) then
	QuestStatus = GetQuestStatus(UID, 293)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 293, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 293, 8462, NPC, 18, 9167);
		else
			SelectMsg(UID, 4, 293, 8488, NPC, 41, 9168, 27, -1);
		end
	end
end

if (EVENT == 9167) then
	ShowMap(UID, 557);
end

if (EVENT == 9168) then
	QuestStatus = GetQuestStatus(UID, 293)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 293, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 293, 8462, NPC, 18, 9167);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,993);
			SaveEvent(UID, 9299);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,994);
			SaveEvent(UID, 9304);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,995);
			SaveEvent(UID, 9309);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,996);
			SaveEvent(UID, 9314);
			end
		end
	end
end


if (EVENT == 202) then
	SelectMsg(UID, 4, 457, 8161, NPC, 22, 203, 23, -1);
end

if (EVENT == 203) then
	QuestStatus = GetQuestStatus(UID, 457)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2194);
	end
end

if (EVENT == 205) then
	QuestStatus = GetQuestStatus(UID, 457)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 457, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 457, 8161, NPC, 18, 208);
		else
			SaveEvent(UID, 2196);
		end
	end
end

if (EVENT == 207) then
	QuestStatus = GetQuestStatus(UID, 457)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 457, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 457, 8161, NPC, 18, 208);
		else
			SelectMsg(UID, 4, 457, 8161, NPC, 41, 209, 23, -1);
		end
	end
end


if (EVENT == 209) then
	QuestStatus = GetQuestStatus(UID, 457)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 457, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 457, 8161, NPC, 18, 208);
		else
			RunQuestExchange(UID,21003);
			SaveEvent(UID, 2195);
		end
	end
end

if (EVENT == 302) then
	SelectMsg(UID, 4, 459, 8269, NPC, 22, 303, 23, -1);
end

if (EVENT == 303) then
	QuestStatus = GetQuestStatus(UID, 459)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2206);
	end
end

if (EVENT == 305) then
	QuestStatus = GetQuestStatus(UID, 459)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 459, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 459, 8269, NPC, 18, 308);
		else
			SaveEvent(UID, 2208);
		end
	end
end

if (EVENT == 307) then
	QuestStatus = GetQuestStatus(UID, 459)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 459, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 459, 8269, NPC, 18, 308);
		else
			SelectMsg(UID, 4, 459, 8269, NPC, 41, 309, 23, -1);
		end
	end
end


if (EVENT == 309) then
	QuestStatus = GetQuestStatus(UID, 459)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 459, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 459, 8269, NPC, 18, 308);
		else
			RunQuestExchange(UID,21005);
			SaveEvent(UID, 2207);
		end
	end
end

if (EVENT == 410) then
	SelectMsg(UID, 4, 465, 8163, NPC, 22, 411, 23, -1);
end

if (EVENT == 411) then
	QuestStatus = GetQuestStatus(UID, 465)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2242);
	end
end

if (EVENT == 413) then
	QuestStatus = GetQuestStatus(UID, 465)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 465, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 465, 8163, NPC, 18, 414);
		else
			SaveEvent(UID, 2244);
		end
	end
end

if (EVENT == 415) then
	QuestStatus = GetQuestStatus(UID, 465)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 465, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 465, 8163, NPC, 18, 414);
		else
			SelectMsg(UID, 4, 465, 8163, NPC, 41, 416, 23, -1);
		end
	end
end

if (EVENT == 416) then
	QuestStatus = GetQuestStatus(UID, 465)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 465, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 465, 8163, NPC, 18, 414);
		else
			RunQuestExchange(UID,1944);
			SaveEvent(UID, 2243);
		end
	end
end

if (EVENT == 502) then
	SelectMsg(UID, 4, 467, 8307, NPC, 22, 503, 23, -1);
end

if (EVENT == 503) then
	QuestStatus = GetQuestStatus(UID, 467)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2254);
	end
end

if (EVENT == 505) then
	QuestStatus = GetQuestStatus(UID, 467)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 467, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 467, 8307, NPC, 18, 508);
		else
			SaveEvent(UID, 2256);
		end
	end
end

if (EVENT == 507) then
	QuestStatus = GetQuestStatus(UID, 467)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 467, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 467, 8307, NPC, 18, 508);
		else
			SelectMsg(UID, 4, 467, 8307, NPC, 41, 509, 23, -1);
		end
	end
end


if (EVENT == 509) then
	QuestStatus = GetQuestStatus(UID, 467)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 467, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 467, 8307, NPC, 18, 508);
		else
			RunQuestExchange(UID,1946);
			SaveEvent(UID, 2255);
		end
	end
end

if (EVENT == 602) then
	SelectMsg(UID, 4, 471, 8448, NPC, 22, 603, 23, -1);
end

if (EVENT == 603) then
	QuestStatus = GetQuestStatus(UID, 471)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2278);
	end
end

if (EVENT == 605) then
	QuestStatus = GetQuestStatus(UID, 471)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 471, 1);
		if (MonsterCount < 15) then
			SelectMsg(UID, 2, 471, 8448, NPC, 18, 608);
		else
			SaveEvent(UID, 2280);
		end
	end
end

if (EVENT == 607) then
	QuestStatus = GetQuestStatus(UID, 471)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 471, 1);
		if (MonsterCount < 15) then
			SelectMsg(UID, 2, 471, 8448, NPC, 18, 608);
		else
			SelectMsg(UID, 4, 471, 8448, NPC, 41, 609, 23, -1);
		end
	end
end


if (EVENT == 609) then
	QuestStatus = GetQuestStatus(UID, 471)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 471, 1);
		if (MonsterCount < 15) then
			SelectMsg(UID, 2, 471, 8448, NPC, 18, 608);
		else
			RunQuestExchange(UID,21007);
			SaveEvent(UID, 2279);
		end
	end
end

if (EVENT == 702) then
	SelectMsg(UID, 4, 473, 8452, NPC, 22, 703, 23, -1);
end

if (EVENT == 703) then
	QuestStatus = GetQuestStatus(UID, 473)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2290);
	end
end

if (EVENT == 705) then
	QuestStatus = GetQuestStatus(UID, 473)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 473, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 473, 8452, NPC, 18, 708);
		else
			SaveEvent(UID, 2292);
		end
	end
end

if (EVENT == 707) then
	QuestStatus = GetQuestStatus(UID, 473)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 473, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 473, 8452, NPC, 18, 708);
		else
			SelectMsg(UID, 4, 473, 8452, NPC, 41, 709, 23, -1);
		end
	end
end


if (EVENT == 709) then
	QuestStatus = GetQuestStatus(UID, 473)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 473, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 473, 8452, NPC, 18, 708);
		else
			RunQuestExchange(UID,1950);
			SaveEvent(UID, 2291);
		end
	end
end