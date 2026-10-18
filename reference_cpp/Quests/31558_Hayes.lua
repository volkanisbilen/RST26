-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31558;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then 
		SelectMsg(UID, 2, -1, 4703, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then 
		NpcMsg(UID, 8060, NPC)
	else
		EVENT = QuestNum
	end
end


if (EVENT == 1001) then
SelectMsg(UID, 4, 643, 21267, NPC, 22, 1002, 23, -1);
end

if (EVENT == 1002) then
	QuestStatus = GetQuestStatus(UID, 643)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12516);
	end
end

if (EVENT == 1006) then
	QuestStatus = GetQuestStatus(UID, 643)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389500000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 643, 21267, NPC, 18,1004);
		else
			SaveEvent(UID, 12518);
		end
	end
end

if (EVENT == 1005) then
	QuestStatus = GetQuestStatus(UID, 643)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389500000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 643, 21267, NPC, 18,1004);
		else
			SelectMsg(UID, 4, 643, 21267, NPC, 22, 1007, 27, -1);
		end
	end
end	

if (EVENT == 1004) then
	ShowMap(UID, 545);
end

if (EVENT == 1007)then
	QuestStatus = GetQuestStatus(UID, 643)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389500000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 643, 21267, NPC, 18,1004);
		else
			RunQuestExchange(UID,3128);
			SaveEvent(UID,12517);
			SaveEvent(UID,12528);
			SelectMsg(UID, 2, 643, 21632, NPC, 10,-1);
		end
	end
end

if (EVENT == 1101) then
SelectMsg(UID, 4, 645, 21271, NPC, 22, 1102, 23, -1);
end

if (EVENT == 1102) then
	QuestStatus = GetQuestStatus(UID, 645)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12540);
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 645)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389760000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 645, 21271, NPC, 18,1104);
		else
			SaveEvent(UID, 12542);
		end
	end
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 645)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389760000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 645, 21271, NPC, 18,1104);
		else
			SelectMsg(UID, 4, 645, 21271, NPC, 22, 1107, 27, -1);
		end
	end
end	

if (EVENT == 1104) then
	ShowMap(UID, 344);
end

if (EVENT == 1107)then
	QuestStatus = GetQuestStatus(UID, 645)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389760000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 645, 21271, NPC, 18,1104);
		else
			RunQuestExchange(UID,3130);
			SaveEvent(UID,12541);
			SaveEvent(UID,12552);
			SelectMsg(UID, 2, 645, 21658, NPC, 10,-1);
		end
	end
end

if (EVENT == 1201) then
SelectMsg(UID, 4, 647, 21275, NPC, 22, 1202, 23, -1);
end

if (EVENT == 1202) then
	QuestStatus = GetQuestStatus(UID, 647)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12564);
	end
end

if (EVENT == 1206) then
	QuestStatus = GetQuestStatus(UID, 647)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389510000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 647, 21275, NPC, 18,1204);
		else
			SaveEvent(UID, 12566);
		end
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 647)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389510000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 647, 21275, NPC, 18,1204);
		else
			SelectMsg(UID, 4, 647, 21275, NPC, 22, 1207, 27, -1);
		end
	end
end	

if (EVENT == 1204) then
	ShowMap(UID, 827);
end

if (EVENT == 1207)then
	QuestStatus = GetQuestStatus(UID, 647)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389510000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 647, 21275, NPC, 18,1204);
		else
			RunQuestExchange(UID,3132);
			SaveEvent(UID,12565);
			SaveEvent(UID,12576);
		end
	end
end

if (EVENT == 1301) then
SelectMsg(UID, 4, 649, 21279, NPC, 22, 1302, 23, -1);
end

if (EVENT == 1302) then
	QuestStatus = GetQuestStatus(UID, 649)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12588);
	end
end

if (EVENT == 1306) then
	QuestStatus = GetQuestStatus(UID, 649)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389440000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 649, 21279, NPC, 18,1304);
		else
			SaveEvent(UID, 12590);
		end
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 649)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389440000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 649, 21279, NPC, 18,1304);
		else
		SelectMsg(UID, 4, 649, 21279, NPC, 22, 1307, 27, -1);
		end
	end
end	

if (EVENT == 1304) then
	ShowMap(UID, 514);
end

if (EVENT == 1307)then
	QuestStatus = GetQuestStatus(UID, 649)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389440000);   
		if (ITEM1_COUNT < 3) then
			SelectMsg(UID, 2, 649, 21279, NPC, 18,1304);
		else
			RunQuestExchange(UID,3134);
			SaveEvent(UID,12589);
			SaveEvent(UID,12600);
		end
	end
end