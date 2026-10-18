-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29098;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 9754, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 9754, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1802) then
	SelectMsg(UID, 4, 837, 9754, NPC, 22, 1803, 23, -1);
end


if (EVENT == 1803) then
	SelectMsg(UID, 2, -1, 9754, NPC, 10, -1);
end