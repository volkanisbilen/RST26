-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32585;

if (EVENT == 100) then
	QuestStatusCheck = GetQuestStatus(UID, 772);	
	Benshars_spell_water = HowmuchItem(UID, 900293000);  
		if(QuestStatusCheck == 1 and Benshars_spell_water < 1) then
			EVENT = 101
		else
			SelectMsg(UID,2,-1,44479,NPC,10,-1);
		end
end

if (EVENT == 101)then
	SlotCheck = CheckGiveSlot(UID, 1);
		if SlotCheck == false then
			;
        else
			SelectMsg(UID, 2, -1, 20816, NPC, 22,-1);
			GiveItem(UID, 900293000,1);
		end
end