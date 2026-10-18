-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32289;

if (EVENT == 100) then
	Level = CheckLevel(UID);
	if (Level > 69) then
		SelectMsg(UID, 2, -1, 906, NPC, 4076, 102, 4154, -1);
	else
		SelectMsg(UID, 2, -1, 910, NPC, 10, -1);
	end	
end

if (EVENT == 102) then
	CheckLider = isPartyLeader(UID);
	if (CheckLider) then
		ZoneChangeParty(UID, 75, 62, 1859);
	else
		ZoneChange(UID, 75, 62, 1859);
	end	
end