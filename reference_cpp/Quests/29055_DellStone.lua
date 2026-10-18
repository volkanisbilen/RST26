-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29055;

if (EVENT == 100) then
SelectMsg(UID, 2, -1, 9496, NPC, 3000, 101,3005,-1);
end

if (EVENT == 101) then
	Check = isCswWinnerNembers(UID)
	if (Check) then
		DelosCasttellanZoneOut(UID);
	else
		SelectMsg(UID, 2, -1, 9497, NPC, 10, -1);
	end
end
