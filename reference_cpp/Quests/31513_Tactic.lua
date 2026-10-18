-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31513;

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 9205, NPC, 7255, 101);
end

if (EVENT == 101) then
	SelectMsg(UID, 2, -1, 9101, NPC, 3000, 102,3005,-1);
end

if (EVENT == 102) then
	SelectMsg(UID, 19, -1, 9116, NPC, 10,103);
end

if (EVENT == 103) then
	SelectMsg(UID, 2, -1, 9123, NPC, 7099,104,7098,120);
end

if (EVENT == 104) then
	SelectMsg(UID, 2, -1, 9132, NPC, 7091,105,7092,106,7093,107);
end

if (EVENT == 105) then 
	NOAH = HowmuchItem(UID, 900000000);
	if (NOAH < 50000) then
		SelectMsg(UID, 2, -1, 9135, NPC, 18, 1000);
	else
	Cast = CastSkill(UID, 492062);
	if (Cast) then
		CastSkill(UID, 492062);
		end
	end
end

if (EVENT == 106) then 
	NOAH = HowmuchItem(UID, 900000000);
	if (NOAH < 50000) then
		SelectMsg(UID, 2, -1, 9135, NPC, 18, 1000);
	else
	Cast = CastSkill(UID, 301028);
	if (Cast) then
		CastSkill(UID, 301028);
		end
	end
end

if (EVENT == 107) then 
	NOAH = HowmuchItem(UID, 900000000);
	if (NOAH < 50000) then
		SelectMsg(UID, 2, -1, 9135, NPC, 18, 1000);
	else
	Cast = CastSkill(UID, 302328);
	if (Cast) then
		CastSkill(UID, 302328);
		end
	end
end