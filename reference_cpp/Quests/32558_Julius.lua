-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32558;

if (EVENT == 100) then -- Eventleri Boş vaktimde yazıcam -Yasin(TriLogy)...
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 1034, NPC,10,-1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 1034, NPC)
	else
		EVENT = QuestNum
	end
end
--if EVENT == 100 then
	--SelectMsg(UID, 2, -1, 1034, NPC,10,-1);
--end
