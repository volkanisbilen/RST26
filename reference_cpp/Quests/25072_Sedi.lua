-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25072;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43812, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43812, NPC)
	else
		EVENT = QuestNum
	end
end

if(EVENT == 112) then 
	SelectMsg(UID, 4, 1240, 43812, NPC, 22, 113, 23, -1);
end

if(EVENT == 113) then
	QuestStatus = GetQuestStatus(UID, 1240)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7548);
	end
end

if(EVENT == 117) then
	QuestStatus = GetQuestStatus(UID, 1240)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	WOLFDOG = HowmuchItem(UID, 900652000)
		if( WOLFDOG < 20) then
			SelectMsg(UID, 2, 1240, 43812, NPC, 18, 116);
		else
			SaveEvent(UID, 7550);
		end
	end
end

if(EVENT == 115) then
	QuestStatus = GetQuestStatus(UID, 1240)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	WOLFDOG = HowmuchItem(UID, 900652000)
		if( WOLFDOG < 20) then
			SelectMsg(UID, 2, 1240, 43812, NPC, 18, 116);
		else
			SelectMsg(UID, 4, 1240, 43812, NPC, 10, 118, 27, -1);
		end
	end
end

if(EVENT == 116) then
ShowMap(UID, 1318);
end

if(EVENT == 118) then
	QuestStatus = GetQuestStatus(UID, 1240)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	WOLFDOG = HowmuchItem(UID, 900652000)
		if( WOLFDOG < 20) then
			SelectMsg(UID, 2, 1240, 43812, NPC, 18, 116);
		else
			SaveEvent(UID, 7549);
			RunQuestExchange(UID,6036);
		end
	end
end


if(EVENT == 122) then 
	SelectMsg(UID, 4, 1241, 43815, NPC, 10, 123, 23, -1);
end

if(EVENT == 123) then
	QuestStatus = GetQuestStatus(UID, 1241)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7554);
			GiveItem(UID, 900670000,1);
	end
end

if(EVENT == 127) then
	QuestStatus = GetQuestStatus(UID, 1241)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SADDY = HowmuchItem(UID, 900659000)
		if( SADDY < 1) then
			SelectMsg(UID, 2, 1241, 43815, NPC, 18, 126);
		else
			SaveEvent(UID, 7556);
		end
	end
end
	
if(EVENT == 125) then
	QuestStatus = GetQuestStatus(UID, 1241)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SADDY = HowmuchItem(UID, 900659000)
		if( SADDY < 1) then
			SelectMsg(UID, 2, 1241, 43815, NPC, 18, 126);
		else
			SelectMsg(UID, 4, 1241, 43815, NPC, 10, 128, 27, -1);
		end
	end
end

if(EVENT == 126) then
	ShowMap(UID, 1334);
end

if(EVENT == 128) then
	QuestStatus = GetQuestStatus(UID, 1241)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SADDY = HowmuchItem(UID, 900659000)
		if( SADDY < 1) then
			SelectMsg(UID, 2, 1241, 43815, NPC, 18, 126);
		else
			SaveEvent(UID, 7555);
			RunQuestExchange(UID,6037);
		end
	end
end