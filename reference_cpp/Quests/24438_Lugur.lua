-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24438;

if (EVENT == 3000) then
	NpcMsg(UID, 810, NPC);
end

if (EVENT == 3010) then
    SelectMsg(UID, 2, -1, 803, NPC, 67, 3011, 68, -1);
end

if (EVENT == 3011) then
	Level = CheckLevel(UID);
	if (Level > 69) then
		SelectMsg(UID, 2, -1, 804, NPC, 2002, 3012);
	else
		SelectMsg(UID, 2, -1, 810, NPC, 10, -1);
	end
end

if (EVENT == 3012) then
	SelectMsg(UID, 2, -1, 805, NPC, 65, 3013);
end

if (EVENT == 3013) then
JURADTIME = CheckJuraidMountainTime(UID);
if (JURADTIME == true) then
	JoinEvent(UID);
	SaveEvent(UID, 695);
else
	SelectMsg(UID, 2, -1, 804, NPC, 10, -1);
	end
end