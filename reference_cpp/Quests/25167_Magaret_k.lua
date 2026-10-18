-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 25167;

if (EVENT == 100) then
	QuestStatusCheck = GetQuestStatus(UID, 1311)	
		if(QuestStatusCheck == 1) then
			EVENT = 101
		else
			SelectMsg(UID, 2, -1, 43821, NPC, 10, -1);
	end
end

if(EVENT == 101) then
SelectMsg(UID, 2, -1, 43821, NPC, 40250, 102);
end

if(EVENT == 102) then
SelectMsg(UID, 2, -1, 43822, NPC, 40251, 103);
end

if(EVENT == 103) then
	SADI = HowmuchItem(UID, 900670000)
		if( SADI > 0) then
			SelectMsg(UID, 2, -1, 43823, NPC, 40252, 104);
		else
			ShowMap(UID, 1334);
	end
end

if(EVENT == 104) then
	SADI = HowmuchItem(UID, 900670000)
		if( SADI > 0) then
			RobItem(UID, 900670000);
			GiveItem(UID, 900659000,1);
		else
			ShowMap(UID, 1334);
	end
end