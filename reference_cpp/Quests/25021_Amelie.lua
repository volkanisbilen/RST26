-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25021;

if (EVENT == 100) then
	QuestStatus = GetQuestStatus(UID, 1223)	--1228
		if(QuestStatus == 1) then
			EVENT = 112
		else
	QuestStatus = GetQuestStatus(UID, 1226)	--1228
		if(QuestStatus == 1) then
			EVENT = 122
		else
	QuestStatus = GetQuestStatus(UID, 1228)	--1228
		if(QuestStatus == 1) then
			EVENT = 132
		else
	QuestNum = SearchQuest(UID, NPC);
		if (QuestNum == 0) then
			SelectMsg(UID, 2, -1, 43664, NPC, 10, -1);
		elseif (QuestNum > 1 and  QuestNum < 100) then
			NpcMsg(UID, 43664, NPC)
		else
			EVENT = QuestNum
		end
	end
end
end
end

if(EVENT == 112) then
	SelectMsg(UID, 2, 1224, 43664, NPC, 40201, 113);
end

if(EVENT == 113) then
	SelectMsg(UID, 2, 1224, 43770, NPC, 40204, 114);
end

if(EVENT == 114) then
	SelectMsg(UID, 2, 1224, 43771, NPC, 40205, 115);
end

if(EVENT == 115) then
	SelectMsg(UID, 2, 1224, 43772, NPC, 4160, 116);
end

if(EVENT == 116) then
	SelectMsg(UID, 2, 1224, 43773, NPC, 4160, 117);
end

if(EVENT == 117) then
	SelectMsg(UID, 2, 1224, 43773, NPC, 4160, 118);
end

if(EVENT == 118) then
	SelectMsg(UID, 2, 1224, 43774, NPC, 4160, 119);
end

if(EVENT == 119) then
	SelectMsg(UID, 2, 1224, 43774, NPC, 4160, 120);
end

if(EVENT == 120) then
	SelectMsg(UID, 2, 1224, 43775, NPC, 4160, 121);
end

if(EVENT == 121) then
	SaveEvent(UID, 7453);
	SaveEvent(UID, 7448);
end


if(EVENT == 122) then
	SelectMsg(UID, 2, 1225, 43664, NPC, 40202, 123);
end

if(EVENT == 123) then
	SelectMsg(UID, 2, 1225, 43780, NPC, 40209, 124,40210,-1);
end

if(EVENT == 124) then
	SlotCheck = CheckGiveSlot(UID, 2)
		if SlotCheck == false then
			
        else
			SaveEvent(UID, 7459);
            GiveItem(UID, 900609000, 1);
	end
end

if(EVENT == 132) then
	SelectMsg(UID, 2, -1, 43664, NPC, 40211, 133);
end

if(EVENT == 133) then
	SelectMsg(UID, 2, -1, 43783, NPC, 40212, 134);
end

if(EVENT == 134) then
	SelectMsg(UID, 2, -1, 43784, NPC, 40212, 135);
end

if(EVENT == 135) then
	SelectMsg(UID, 2, -1, 43785, NPC, 40214, 136);
end

if(EVENT == 136) then
	SelectMsg(UID, 2, -1, 43786, NPC, 40215, 137);
end

if(EVENT == 137) then
	SelectMsg(UID, 2, -1, 43787, NPC, 10, 138);
end

if(EVENT == 138) then
	SaveEvent(UID, 7478);
end