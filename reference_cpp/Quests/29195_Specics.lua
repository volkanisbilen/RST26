-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29195;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 10392, NPC,7570,110,7571,120,7581,130);
end

if (EVENT == 110) then
QuestStatus = GetQuestStatus(UID, 939);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 111
	else
		SelectMsg(UID, 2, -1, 10380, NPC, 56,112);
	end
end

if (EVENT == 111) then
QuestStatus = GetQuestStatus(UID, 939);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10382, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10392, NPC, 10,113);
		else
			SaveEvent(UID, 6781);
	end
end

if (EVENT == 112) then
	SaveEvent(UID, 6781);
end

if (EVENT == 1005) then
	SaveEvent(UID, 6783);
end

if (EVENT == 113) then
COUNTA = HowmuchItem(UID, 508137000);
	if (COUNTA < 3) then
		SelectMsg(UID, 2, -1, 10382, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2530);
		SaveEvent(UID, 6784);
	end
end

if (EVENT == 120) then
QuestStatus = GetQuestStatus(UID, 940);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 121
	else
		SelectMsg(UID, 2, -1, 10388, NPC, 56,122);
	end
end

if (EVENT == 121) then
QuestStatus = GetQuestStatus(UID, 940);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10386, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10393, NPC, 10,123);
		else
			SaveEvent(UID, 6786);
	end
end

if (EVENT == 122) then
	SaveEvent(UID, 6786);
end

if (EVENT == 2005) then
	SaveEvent(UID, 6788);
end

if (EVENT == 123) then
COUNTA = HowmuchItem(UID, 508138000);
	if (COUNTA < 3) then
		SelectMsg(UID, 2, -1, 10386, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2531);
		SaveEvent(UID, 6789);
	end
end

if (EVENT == 130) then
QuestStatus = GetQuestStatus(UID, 941);
	if(QuestStatus == 1 or QuestStatus == 3) then
		EVENT = 131
	else
		SelectMsg(UID, 2, -1, 10406, NPC, 56,132);
	end
end

if (EVENT == 131) then
QuestStatus = GetQuestStatus(UID, 941);
		if(QuestStatus == 1) then
			SelectMsg(UID, 2, -1, 10406, NPC, 10,-1);
		elseif (QuestStatus == 3) then
			SelectMsg(UID, 2, -1, 10482, NPC, 10,133);
		else
			SaveEvent(UID, 6791);
	end
end

if (EVENT == 132) then
	SaveEvent(UID, 6791);
end

if (EVENT == 4005) then
	SaveEvent(UID, 6793);
end

if (EVENT == 133) then
COUNTA = HowmuchItem(UID, 508142000);
COUNTB = HowmuchItem(UID, 508142000);
COUNTC = HowmuchItem(UID, 508142000);
COUNTD = HowmuchItem(UID, 508142000);
	if (COUNTA < 1 or COUNTB < 1 or COUNTC < 1 or COUNTD < 1) then
		SelectMsg(UID, 2, -1, 10482, NPC, 10,-1);
	else
		RunQuestExchange(UID, 2532);
		SaveEvent(UID, 6794);
	end
end