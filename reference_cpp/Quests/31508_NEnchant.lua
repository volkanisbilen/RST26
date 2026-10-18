-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31508;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 9138, NPC, 7255, 200, 7316, 400, 8430, 600,8915,800);
end

if (EVENT == 1200) then
	SelectMsg(UID, 54, -1, -1, NPC);
end

if (EVENT == 200) then
	Level = CheckLevel(UID);
	if (Level < 35) then
		SelectMsg(UID, 2, -1, 9126, NPC, 4161, 201, 4162, -1);
	elseif (Level > 34 and Level < 61) then
		SelectMsg(UID, 2, -1, 9127, NPC, 4161, 202, 4162, -1);
	elseif (Level > 60) then
		SelectMsg(UID, 2, -1, 9128, NPC, 4161, 203, 4162, -1);
	end
end

if (EVENT == 201) then -- 35 Level'den düşük
	SelectMsg(UID, 19, -1, 9129, NPC, 10, 204);
end

if (EVENT == 202) then -- 35 - 60 Level arası
	VCITORY = HowmuchItem(UID, 900017000);
	if (VCITORY > 0) then
	SelectMsg(UID, 2, -1, 9129, NPC, 10, 219);
	else
	SelectMsg(UID, 19, -1, 9129, NPC, 10, 205);
end
end

if (EVENT == 203) then -- +60 Level'den büyük
	VCITORY = HowmuchItem(UID, 900017000);
	if (VCITORY > 0) then
	SelectMsg(UID, 2, -1, 9129, NPC, 10, 225);
	else
	SelectMsg(UID, 19, -1, 9129, NPC, 10, 206);
end
end


if (EVENT == 204) then
	SelectMsg(UID, 3, -1, 9132, NPC, 7091, 207, 7092, 208, 7093, 209, 8891, 210);
end

if (EVENT == 207) then -- 1 - 34 Attack
	Cast = CastSkill(UID, 302344);
	if (Cast) then
		CastSkill(UID, 302344)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 208) then -- 1 - 34 Defans
	Cast = CastSkill(UID, 302331);
	if (Cast) then
		CastSkill(UID, 302331)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 209) then -- 1 - 34 HP
	Cast = CastSkill(UID, 302328);
	if (Cast) then
		CastSkill(UID, 302328)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 210) then -- 1 - 34 sprint
	Cast = CastSkill(UID, 490223);
	if (Cast) then
		CastSkill(UID, 490223)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 205) then
	NOAH = HowmuchItem(UID, 900000000);
	if (NOAH < 30000) then
		SelectMsg(UID, 2, -1, 9135, NPC, 18, 1000);
	else
		SelectMsg(UID, 3, -1, 9133, NPC, 7091, 211, 7094, 212, 7095, 213, 8891, 214);
	end
end

if (EVENT == 211) then -- 35 - 60 Attack
	Cast = CastSkill(UID, 302344);
	if (Cast) then
		CastSkill(UID, 302344)
		GoldLose(UID, 30000)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 212) then -- 35 - 60 Defans
	Cast = CastSkill(UID, 302332);
	if (Cast) then
		CastSkill(UID, 302332)
		GoldLose(UID, 30000)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 213) then -- 35 - 60 HP
	Cast = CastSkill(UID, 302329);
	if (Cast) then
		CastSkill(UID, 302329)
		GoldLose(UID, 30000)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 214) then -- 35 - 60 sprint
	Cast = CastSkill(UID, 490223);
	if (Cast) then
		CastSkill(UID, 490223)
		GoldLose(UID, 30000)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 206) then
	NOAH = HowmuchItem(UID, 900000000);
	if (NOAH < 50000) then
		SelectMsg(UID, 2, -1, 9135, NPC, 18, 1000);
	else
		SelectMsg(UID, 3, -1, 9134, NPC, 7091, 215, 7096, 216, 7097, 217, 8891, 218);
	end
end

if (EVENT == 215) then -- +60 Attack
	Cast = CastSkill(UID, 302344);
	if (Cast) then
		CastSkill(UID, 302344)
		GoldLose(UID, 50000)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 216) then -- +60 Defans
	Cast = CastSkill(UID, 302333);
	if (Cast) then
		CastSkill(UID, 302333)
		GoldLose(UID, 50000)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 217) then -- +60 HP
	Cast = CastSkill(UID, 302330);
	if (Cast) then
		CastSkill(UID, 302330)
		GoldLose(UID, 50000)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 218) then -- +60 Swift
	Cast = CastSkill(UID, 490223);
	if (Cast) then
		CastSkill(UID, 490223)
		GoldLose(UID, 50000)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 219) then
		SelectMsg(UID, 3, -1, 9133, NPC, 7091, 220, 7094, 221, 7095, 222, 8891, 223);
	end

if (EVENT == 221) then -- 35 - 60 Attack
	Cast = CastSkill(UID, 302344);
	if (Cast) then
		CastSkill(UID, 302344)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 222) then -- 35 - 60 Defans
	Cast = CastSkill(UID, 302332);
	if (Cast) then
		CastSkill(UID, 302332)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 223) then -- 35 - 60 HP
	Cast = CastSkill(UID, 302329);
	if (Cast) then
		CastSkill(UID, 302329)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 224) then -- 35 - 60 sprint
	Cast = CastSkill(UID, 490223);
	if (Cast) then
		CastSkill(UID, 490223)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 225) then
		SelectMsg(UID, 3, -1, 9134, NPC, 7091, 226, 7096, 227, 7097, 228, 8891, 229);
end

if (EVENT == 226) then -- +60 Attack
	Cast = CastSkill(UID, 302344);
	if (Cast) then
		CastSkill(UID, 302344)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 227) then -- +60 Defans
	Cast = CastSkill(UID, 302333);
	if (Cast) then
		CastSkill(UID, 302333)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 228) then -- +60 HP
	Cast = CastSkill(UID, 302330);
	if (Cast) then
		CastSkill(UID, 302330)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 229) then -- +60 Swift
	Cast = CastSkill(UID, 490223);
	if (Cast) then
		CastSkill(UID, 490223)
		NpcMsg(UID, 9137);
	else
		NpcMsg(UID, 9137);
	end
end

if (EVENT == 400) then 
	COUPON = HowmuchItem(UID, 900667000);
	if (COUPON < 1) then
		SelectMsg(UID, 2, -1, 11849, NPC, 10, 241);
	else
		Ret = 1;
	end
end

if (EVENT == 600) then 
	COUPON = HowmuchItem(UID, 900667000);
	if (COUPON < 1) then
		SelectMsg(UID, 2, -1, 11849, NPC, 10, 241);
	else
		Ret = 1;
	end
end

if (EVENT == 800) then
	SelectMsg(UID, 3, -1, 9138, NPC, 8916,801,8917,802,8918,803);
end

if (EVENT == 801) then
	Level = CheckLevel(UID);
	if (Level < 35) then
		EVENT = 804
		else
		SelectMsg(UID, 2, -1, 9129, NPC, 10, -1);
end
end

if (EVENT == 804) then
	Cast1 = CastSkill(UID, 302344);
	Cast2 = CastSkill(UID, 302331);
	Cast3 = CastSkill(UID, 302328);
	Cast4 = CastSkill(UID, 490223);
	if (Cast1) then
    Cast1 = CastSkill(UID, 302344);
    elseif (cast2) then
    Cast2 = CastSkill(UID, 302331);
    elseif (cast3) then
	Cast3 = CastSkill(UID, 302328);
	elseif (cast4) then
	Cast4 = CastSkill(UID, 490223);	
	else
    NpcMsg(UID, 9137);
	end
	end
	
if (EVENT == 802) then
	Level = CheckLevel(UID);
	NOAH = HowmuchItem(UID, 900000000);
	if (Level > 34 and Level < 61 and NOAH > 150000) then
	EVENT = 805
	elseif (Level > 60 and NOAH > 200000) then
	EVENT = 806
	else
	SelectMsg(UID, 2, -1, 9135, NPC, 10, -1);
end
end

if (EVENT == 805) then
	Cast1 = CastSkill(UID, 302344);
	Cast2 = CastSkill(UID, 302332);
	Cast3 = CastSkill(UID, 302329);
	Cast4 = CastSkill(UID, 490223);
	GoldLose(UID, 150000);
	if (Cast1) then
    Cast1 = CastSkill(UID, 302344);
    elseif (cast2) then
    Cast2 = CastSkill(UID, 302332);
    elseif (cast3) then
	Cast3 = CastSkill(UID, 302329);
	elseif (cast4) then
	Cast4 = CastSkill(UID, 490223);	
	else
    NpcMsg(UID, 9137);
	end
	end
	
if (EVENT == 806) then
	Cast1 = CastSkill(UID, 302344);
	Cast2 = CastSkill(UID, 302332);
	Cast3 = CastSkill(UID, 302329);
	Cast4 = CastSkill(UID, 490223);
	GoldLose(UID, 200000);
	if (Cast1) then
    Cast1 = CastSkill(UID, 302344);
    elseif (cast2) then
    Cast2 = CastSkill(UID, 302332);
    elseif (cast3) then
	Cast3 = CastSkill(UID, 302329);
	elseif (cast4) then
	Cast4 = CastSkill(UID, 490223);	
	else
    NpcMsg(UID, 9137);
	end
	end