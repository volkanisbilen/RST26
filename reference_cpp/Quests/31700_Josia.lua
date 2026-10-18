-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31700;
	SaveEvent(UID, 1206)
if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4174, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 4174, NPC)
	else
		EVENT = QuestNum
	end
end
--if EVENT == 100 then
	--QuestStatusCheck = GetQuestStatus(UID, 182)	
	--if(QuestStatusCheck == 3) then
	--	EVENT = 107
	--	else
 --  SelectMsg(UID, 2, -1, 4174, NPC, 7015, 101, 7017, 300);
--end
--end

if EVENT == 201 then 
   SelectMsg(UID, 19, -1, 9185, NPC, 10,102);
end

if EVENT == 202 then 
   SelectMsg(UID, 19, -1, 1642, NPC, 10,103);
end

if EVENT == 203 then 
   SelectMsg(UID, 2, -1, 1564, NPC, 3000,104,3005,-1);
end

if EVENT == 204 then 
   SelectMsg(UID, 4, 182, 1565, NPC, 3000,105,3005,-1);
end

if(EVENT == 205) then
IsTakeToday = GetUserDailyOp(UID,10);
if (IsTakeToday == 1) then
SelectMsg(UID, 19, -1, 1566, NPC, 10,106);
SaveEvent(UID, 1206)
	else
	SelectMsg(UID, 2, -1, 11584, NPC, 10, -1);
	end
end

if(EVENT == 106) then
	SelectMsg(UID, 2, -1, 1571, NPC, 10,-1);
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		SaveEvent(UID, 1209)
		GiveItem(UID, 900074000, 1);
		GiveItem(UID, 900075000, 1);
	end
end

if EVENT == 107 then 
   SelectMsg(UID, 4, 182, 1565, NPC, 3000,108,3005,-1);
end

if EVENT == 108 then
	SaveEvent(UID, 1209);
	RunQuestExchange(UID, 188);
end

if(EVENT == 216) then
SaveEvent(UID, 1208)
end

if (EVENT == 125) then
	SelectMsg(UID, 4, 216, 6333, NPC, 22, 121, 27, -1);
end

if (EVENT == 121) then
	SaveEvent(UID,4167);
end

if (EVENT == 127) then
	SaveEvent(UID,4169);
end

if (EVENT == 130) then
	ITEM1_COUNT = HowmuchItem(UID, 910085000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 216, 4186, NPC, 18,123);
	else
		SelectMsg(UID, 4, 216, 6377, NPC, 4006, 138,4005, -1);
end
end	

if (EVENT == 123 ) then
	ShowMap(UID, 425);
end

if (EVENT == 138)then
	QuestStatusCheck = GetQuestStatus(UID, 216) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 216, NPC, 10, -1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 910085000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 216, 4186, NPC, 18,123);
	else
	RunQuestExchange(UID,469);
	SaveEvent(UID,4168);
		end
	end
end
	  