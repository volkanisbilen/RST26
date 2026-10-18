-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 30335;

if (EVENT == 100) then
	Cast = CastSkill(UID, 610096);
		if (Cast) then
			Cast = CastSkill(UID, 610096);
		else
			SelectMsg(UID, 2, -1, 8970, NPC, 10, -1);
	end	
end