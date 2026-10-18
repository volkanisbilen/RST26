-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29193;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 10399, NPC,7577,110,7578,120,7581,130);
end

if (EVENT == 110) then
QuestStatus = GetQuestStatus(UID, 946);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 111
	else
		SelectMsg(UID, 2, -1, 10399, NPC, 56,112);
	end
end

if (EVENT == 111) then
QuestStatus = GetQuestStatus(UID, 946);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10446, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10399, NPC, 10,113);
		else
			SaveEvent(UID, 6816);
	end
end

if (EVENT == 112) then
	SaveEvent(UID, 6816);
end

if (EVENT == 1005) then
	SaveEvent(UID, 6818);
end

if (EVENT == 113) then
COUNTA = HowmuchItem(UID, 508143000);
	if (COUNTA < 3) then
		SelectMsg(UID, 2, -1, 10446, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2537);
		SaveEvent(UID, 6819);
	end
end
if (EVENT == 120) then
QuestStatus = GetQuestStatus(UID, 947);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 121
	else
		SelectMsg(UID, 2, -1, 10451, NPC, 56,122);
	end
end

if (EVENT == 121) then
QuestStatus = GetQuestStatus(UID, 947);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10451, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10454, NPC, 10,123);
		else
			SaveEvent(UID, 6821);
	end
end

if (EVENT == 122) then
	SaveEvent(UID, 6821);
end

if (EVENT == 2005) then
	SaveEvent(UID, 6823);
end

if (EVENT == 123) then
COUNTA = HowmuchItem(UID, 508144000);
	if (COUNTA < 3) then
		SelectMsg(UID, 2, -1, 10451, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2538);
		SaveEvent(UID, 6824);
	end
end

if (EVENT == 130) then
QuestStatus = GetQuestStatus(UID, 948);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 131
	else
		SelectMsg(UID, 2, -1, 10458, NPC, 56,132);
	end
end

if (EVENT == 131) then
QuestStatus = GetQuestStatus(UID, 948);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10459, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10458, NPC, 10,133);
		else
			SaveEvent(UID, 6826);
	end
end

if (EVENT == 132) then
	SaveEvent(UID, 6826);
end

if (EVENT == 4005) then
	SaveEvent(UID, 6828);
end

if (EVENT == 133) then
COUNTA = HowmuchItem(UID, 508142000);
COUNTB = HowmuchItem(UID, 508142000);
COUNTC = HowmuchItem(UID, 508142000);
COUNTD = HowmuchItem(UID, 508142000);
	if (COUNTA < 1 or COUNTB < 1 or COUNTC < 1 or COUNTD < 1) then
		SelectMsg(UID, 2, -1, 10459, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2539);
		SaveEvent(UID, 6829);
	end
end