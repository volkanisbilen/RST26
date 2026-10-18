-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29197;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 10486, NPC, 7572, 101, 7582, 110,7589,120,7590,130,7636,140);
end

if (EVENT == 110) then
	TROPHYFLAME = HowmuchItem(UID, 800149000);
		if (TROPHYFLAME < 1 or TROPHYFLAME == 0) then
			SelectMsg(UID, 2, -1, 10370, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
	Check = CheckExchange(UID, 2626)
		if  Check == true then   
			Roll = RollDice(UID, 25) 
			found = Roll + 2601
			RunQuestExchange(UID, found);
			end
		end
	end
end

if (EVENT == 130) then
	TROPHYBLOODY = HowmuchItem(UID, 508165000);
		if (TROPHYBLOODY < 1 or TROPHYFLAME == 0) then
			SelectMsg(UID, 2, -1, 10370, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
	Check = CheckExchange(UID, 2734)
		if  Check == true then   
			Roll = RollDice(UID, 33) 
			found = Roll + 2701
			RunQuestExchange(UID, found);
			end
		end
	end
end