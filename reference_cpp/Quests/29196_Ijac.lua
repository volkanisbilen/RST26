-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29196;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 10398, NPC, 7573, 110, 7574, 120, 7575, 130,7581,140);
end

if (EVENT == 110) then
QuestStatus = GetQuestStatus(UID, 942);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 111
	else
		SelectMsg(UID, 2, -1, 10395, NPC, 56,112);
	end
end

if (EVENT == 111) then
QuestStatus = GetQuestStatus(UID, 942);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10416, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10398, NPC, 10,113);
		else
			SaveEvent(UID, 6796);
	end
end

if (EVENT == 112) then
	SaveEvent(UID, 6796);
end

if (EVENT == 1005) then
	SaveEvent(UID, 6798);
end

if (EVENT == 113) then
COUNTA = HowmuchItem(UID, 508139000);
	if (COUNTA < 3) then
		SelectMsg(UID, 2, -1, 10416, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2533);
		SaveEvent(UID, 6799);
	end
end

if (EVENT == 120) then
QuestStatus = GetQuestStatus(UID, 943);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 121
	else
		SelectMsg(UID, 2, -1, 10422, NPC, 56,122);
	end
end

if (EVENT == 121) then
QuestStatus = GetQuestStatus(UID, 943);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10424, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10396, NPC, 10,123);
		else
			SaveEvent(UID, 6801);
	end
end

if (EVENT == 122) then
	SaveEvent(UID, 6801);
end

if (EVENT == 2005) then
	SaveEvent(UID, 6803);
end

if (EVENT == 123) then
COUNTA = HowmuchItem(UID, 508141000);
	if (COUNTA < 3) then
		SelectMsg(UID, 2, -1, 10424, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2534);
		SaveEvent(UID, 6804);
	end
end

if (EVENT == 130) then
QuestStatus = GetQuestStatus(UID, 944);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 131
	else
		SelectMsg(UID, 2, -1, 10397, NPC, 56,132);
	end
end

if (EVENT == 131) then
QuestStatus = GetQuestStatus(UID, 944);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10430, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10429, NPC, 10,133);
		else
			SaveEvent(UID, 6806);
	end
end

if (EVENT == 132) then
	SaveEvent(UID, 6806);
end

if (EVENT == 3005) then
	SaveEvent(UID, 6808);
end

if (EVENT == 133) then
COUNTA = HowmuchItem(UID, 508142000);
	if (COUNTA < 5) then
		SelectMsg(UID, 2, -1, 10430, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2535);
		SaveEvent(UID, 6809);
	end
end

if (EVENT == 140) then
QuestStatus = GetQuestStatus(UID, 945);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 141
	else
		SelectMsg(UID, 2, -1, 10435, NPC, 56,142);
	end
end

if (EVENT == 141) then
QuestStatus = GetQuestStatus(UID, 945);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10436, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10435, NPC, 10,143);
		else
			SaveEvent(UID, 6811);
	end
end

if (EVENT == 142) then
	SaveEvent(UID, 6811);
end

if (EVENT == 4005) then
	SaveEvent(UID, 6813);
end

if (EVENT == 133) then
COUNTA = HowmuchItem(UID, 508142000);
COUNTB = HowmuchItem(UID, 508142000);
COUNTC = HowmuchItem(UID, 508142000);
COUNTD = HowmuchItem(UID, 508142000);
	if (COUNTA < 1 or COUNTB < 1 or COUNTC < 1 or COUNTD < 1) then
		SelectMsg(UID, 2, -1, 10436, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2536);
		SaveEvent(UID, 6814);
	end
end