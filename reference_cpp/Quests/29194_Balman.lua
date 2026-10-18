-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29194;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 10444, NPC, 7579, 110, 7580, 120, 7581, 130);
end

if (EVENT == 110) then
QuestStatus = GetQuestStatus(UID, 949);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 111
	else
		SelectMsg(UID, 2, -1, 10468, NPC, 56,112);
	end
end

if (EVENT == 111) then
QuestStatus = GetQuestStatus(UID, 949);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10470, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10402, NPC, 10,113);
		else
			SaveEvent(UID, 6831);
	end
end

if (EVENT == 112) then
	SaveEvent(UID, 6831);
end

if (EVENT == 1005) then
	SaveEvent(UID, 6833);
end

if (EVENT == 113) then
COUNTA = HowmuchItem(UID, 508145000);
	if (COUNTA < 5) then
		SelectMsg(UID, 2, -1, 10470, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2540);
		SaveEvent(UID, 6834);
	end
end

if (EVENT == 120) then
QuestStatus = GetQuestStatus(UID, 950);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 121
	else
		SelectMsg(UID, 2, -1, 10474, NPC, 56,122);
	end
end

if (EVENT == 121) then
QuestStatus = GetQuestStatus(UID, 950);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10474, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10476, NPC, 10,123);
		else
			SaveEvent(UID, 6836);
	end
end

if (EVENT == 122) then
	SaveEvent(UID, 6836);
end

if (EVENT == 2005) then
	SaveEvent(UID, 6838);
end

if (EVENT == 123) then
COUNTA = HowmuchItem(UID, 508146000);
	if (COUNTA < 5) then
		SelectMsg(UID, 2, -1, 10474, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2541);
		SaveEvent(UID, 6839);
	end
end

if (EVENT == 130) then
QuestStatus = GetQuestStatus(UID, 951);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 131
	else
		SelectMsg(UID, 2, -1, 10458, NPC, 56,132);
	end
end

if (EVENT == 131) then
QuestStatus = GetQuestStatus(UID, 951);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10459, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10458, NPC, 10,133);
		else
			SaveEvent(UID, 6841);
	end
end

if (EVENT == 132) then
	SaveEvent(UID, 6841);
end

if (EVENT == 4005) then
	SaveEvent(UID, 6843);
end

if (EVENT == 133) then
COUNTA = HowmuchItem(UID, 508142000);
COUNTB = HowmuchItem(UID, 508142000);
COUNTC = HowmuchItem(UID, 508142000);
COUNTD = HowmuchItem(UID, 508142000);
	if (COUNTA < 1 or COUNTB < 1 or COUNTC < 1 or COUNTD < 1) then
		SelectMsg(UID, 2, -1, 10459, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2542);
		SaveEvent(UID, 6844);
	end
end