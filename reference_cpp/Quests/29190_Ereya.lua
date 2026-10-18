-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29190;

if (EVENT == 100) then -- normal hali 100
	SelectMsg(UID, 3, -1, 10502, NPC,7586,101,7636,110,7587,120);
end

if (EVENT == 101) then
	Check = CheckUnderTheCastleOpen(UID);
	if (Check == true) then
		EVENT = 102
		else
		SelectMsg(UID, 2, -1, 11792, NPC, 10, -1);
	end
end

if (EVENT == 102) then
	Count = CheckUnderTheCastleUserCount(UID);
	if (Count < 300) then
		ZoneChange(UID, 86, 69, 64);
		else
		SelectMsg(UID, 2, -1, 10542, NPC, 10, -1);
	end
end