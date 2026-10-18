-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29079;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 9612, NPC, 7210, 102, 7213, 101, 7220, 200);
end

if (EVENT == 102) then
	SelectMsg(UID, 2, -1, 9613, NPC, 7212, 103, 7211, 104);
end

if (EVENT == 103) then
	EventSoccerMember(UID, 1, 672, 166)
	-- Enter the blue Team -- Mavi Takım İçin Giriş
end

if (EVENT == 104) then
	EventSoccerMember(UID, 2, 672, 154)
	-- Enter the red Team -- Kırmızı Takım İçin Giriş
end

if (EVENT == 200) then
	EventSoccerStard(UID)
	--Soccer Start -- Maç Başlasın
end

if (EVENT == 101) then
	GOAL = HowmuchItem(UID, 900349000);
	if (GOAL > 9) then
		SelectMsg(UID, 3, -1, 9617, NPC, 7224, 105, 7225, 107, 7226, 109);
	else
		SelectMsg(UID, 2, -1, 9616, NPC, 27, -1);
	end
end

if (EVENT == 105) then
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		 EVENT =106
end
end

if (EVENT == 106) then
	NATION = CheckNation(UID);
	if (NATION == 1) then
        RobItem(UID, 900349000, 10);
		GiveItem(UID, 900362641, 1,1);
		SelectMsg(UID, 2, -1, 9618, NPC, 10, -1);
	else
		RobItem(UID, 900349000, 10);
		GiveItem(UID, 900357640, 1,1);
		SelectMsg(UID, 2, -1, 9618, NPC, 10, -1);
	end
end

if (EVENT == 107) then
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		 EVENT =108
end
end

if (EVENT == 108) then
	NATION = CheckNation(UID);
	if (NATION == 1) then
        RobItem(UID, 900349000, 10);
		GiveItem(UID, 929001609, 1,1);
        SelectMsg(UID, 2, -1, 9618, NPC, 10, -1);		
	else
		RobItem(UID, 900349000, 10);
		GiveItem(UID, 927001607, 1,1);
		SelectMsg(UID, 2, -1, 9618, NPC, 10, -1);
	end
end

if (EVENT == 109) then
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		 EVENT =110
end
end

if (EVENT == 110) then
	NATION = CheckNation(UID);
	if (NATION == 1) then
        RobItem(UID, 900349000, 10);
		GiveItem(UID, 933001634, 1,1);	
        SelectMsg(UID, 2, -1, 9618, NPC, 10, -1);		
	else
		RobItem(UID, 900349000, 10);
		GiveItem(UID, 928001608, 1,1);
		SelectMsg(UID, 2, -1, 9618, NPC, 10, -1);
	end
end