-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24441;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 3003, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 3200, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1002) then
	SelectMsg(UID, 4, 403, 799, NPC, 22, 1003, 23, -1);
end

if (EVENT == 1003) then
	QuestStatus = GetQuestStatus(UID, 403)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2014);
	end
end

if (EVENT == 1010) then
	QuestStatus = GetQuestStatus(UID, 403)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 403, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 403, 732, NPC, 18, 1007);
		else
			SaveEvent(UID, 2016);
		end
	end
end

if (EVENT == 1006) then
	QuestStatus = GetQuestStatus(UID, 403)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 403, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 403, 732, NPC, 18, 1007);
		else
			SelectMsg(UID, 4, 403, 8461, NPC, 41, 1008, 23, 193);
		end
	end
end

if (EVENT == 1007) then
	ShowMap(UID, 97);
end

if (EVENT == 1008) then
	QuestStatus = GetQuestStatus(UID, 403)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 403, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 403, 732, NPC, 18, 1007);
		else
			RunQuestExchange(UID, 1190);
			SaveEvent(UID, 2015);
		end
	end
end

if (EVENT == 8402) then
	SelectMsg(UID, 4, 128, 8363, NPC, 22, 8403, 23, -1);
end   

if (EVENT == 8403) then
	QuestStatus = GetQuestStatus(UID, 128)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8470);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8475);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8480);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8485);
		end
	end
end

if (EVENT == 8410) then
	QuestStatus = GetQuestStatus(UID, 128)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 128, 1);
		if (MonsterCount < 15) then
			SelectMsg(UID, 2, 128, 8363, NPC, 18, 8407);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8472);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8477);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8482);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8487);
			end
		end
	end
end

if (EVENT == 8406) then
	QuestStatus = GetQuestStatus(UID, 128)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 128, 1);
		if (MonsterCount < 15) then
			SelectMsg(UID, 2, 128, 8363, NPC, 18, 8407);
		else
			SelectMsg(UID, 4, 128, 8363, NPC, 41, 8408, 23, -1);
		end
	end
end

if (EVENT == 8407) then
	ShowMap(UID, 97);
end

if (EVENT == 8408) then
	QuestStatus = GetQuestStatus(UID, 128)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 128, 1);
		if (MonsterCount < 15) then
			SelectMsg(UID, 2, 128, 8363, NPC, 18, 8407);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID, 904)
			SaveEvent(UID, 8471);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID, 905)
			SaveEvent(UID, 8476);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID, 906)
			SaveEvent(UID, 8481);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID, 907)
			SaveEvent(UID, 8486);
			end
		end
	end
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 415, 799, NPC, 22, 1103, 23, -1);
end

if (EVENT == 1103) then
	QuestStatus = GetQuestStatus(UID, 415)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2086);
	end
end

if (EVENT == 1110) then
	QuestStatus = GetQuestStatus(UID, 415)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 415, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 415, 732, NPC, 18, 1107);
		else
			SaveEvent(UID, 2088);
		end
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 415)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 415, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 415, 732, NPC, 18, 1107);
		else
			SelectMsg(UID, 4, 415, 8461, NPC, 41, 1108, 23, -1);
		end
	end
end

if (EVENT == 1107) then
	ShowMap(UID, 109);
end

if (EVENT == 1108) then
	QuestStatus = GetQuestStatus(UID, 415)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 415, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 415, 732, NPC, 18, 1107);
		else
			RunQuestExchange(UID, 1196);
			SaveEvent(UID, 2087); 
		end
	end
end

if (EVENT == 8852) then
	SelectMsg(UID, 4, 146, 799, NPC, 22, 8853, 23, -1);
end   

if (EVENT == 8853) then
	QuestStatus = GetQuestStatus(UID, 146)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8722);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8727);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8732);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8737);
		end
	end
end

if (EVENT == 8860) then
	QuestStatus = GetQuestStatus(UID, 146)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 146, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 146, 732, NPC, 18, 8857);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 8724);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 8729);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 8734);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 8739);
			end
		end
	end
end

if (EVENT == 8856) then
	QuestStatus = GetQuestStatus(UID, 146)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 146, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 146, 732, NPC, 18, 8857);
		else
			SelectMsg(UID, 4, 146, 8461, NPC, 41, 8858, 23, -1);
		end
	end
end

if (EVENT == 8857) then
	ShowMap(UID, 109);
end

if (EVENT == 8858) then
	QuestStatus = GetQuestStatus(UID, 146)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 146, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 146, 732, NPC, 18, 8857);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID, 961);
			SaveEvent(UID, 8723);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID, 962);
			SaveEvent(UID, 8728);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID, 963);
			SaveEvent(UID, 8733);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID, 964);
			SaveEvent(UID, 8738);
			end
		end
	end
end