-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local EventData = 500;
local NPC = 0;

if (EVENT == 500) then
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 6, EventData, 5000, NPC, 5000, 501);
		else
			SelectMsg(UID, 6, EventData, 5001, NPC, 5000, 501);
		end
end

if (EVENT == 501) then
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SaveEvent(UID, 5001);
			ShowMap(UID, 1);
		else
			SaveEvent(UID, 5004);
			ShowMap(UID, 1);
		end
end

if (EVENT == 502) then
NATION = CheckNation(UID);
	if (NATION == 1) then
		SaveEvent(UID, 5002);
		SelectMsg(UID, 6, EventData, 5002, NPC, 5001, 505);
	else
		SaveEvent(UID, 5005);
		SelectMsg(UID, 6, EventData, 5003, NPC, 5004, 505);
	end
end

if (EVENT == 505) then
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 1, EventData, 5004, NPC, 5002, 506);
		else
			SelectMsg(UID, 1, EventData, 5005, NPC, 5005, 506);
		end
end
	
if (EVENT == 506) then
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 1, EventData, 5006, NPC, 5003, 507);
		else
			SelectMsg(UID, 1, EventData, 5007, NPC, 5003, 507);
		end
end

if (EVENT == 507) then
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 1, EventData, 5008, NPC, 5004, 508);
		else
			SelectMsg(UID, 1, EventData, 5009, NPC, 5004, 508);
		end
end

if (EVENT == 508) then
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 1, EventData, 5010, NPC, 5005, 509);
		else
			SelectMsg(UID, 1, EventData, 5011, NPC, 5005, 509);
		end
end

if (EVENT == 509) then
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 1, EventData, 5012, NPC, 5006, 510);
		else
			SelectMsg(UID, 1, EventData, 5013, NPC, 5006, 510);
		end
end

if (EVENT == 510) then
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 6, EventData, 5014, NPC, 6002, -1);
		else
			SelectMsg(UID, 6, EventData, 5015, NPC, 6002, -1);
		end
end