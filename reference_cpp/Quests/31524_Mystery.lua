-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31524;
local Ret = 0;

if (EVENT == 100) then
	QuestStatusCheck = GetQuestStatus(UID, 772)	
	ITEM1_COUNT = HowmuchItem(UID, 900291000);  
	if(QuestStatusCheck == 1 and ITEM1_COUNT < 1) then
		EVENT = 2000
	else
	SelectMsg(UID, 3, -1, 21215, NPC, 7494, 102, 8351, 103,8788,104);
end
end

if (EVENT == 102) then
	SelectMsg(UID, 27, -1, -1, NPC); --ilk baştaki -1 olan yer normal hali 27
end

if (EVENT == 103) then
	SelectMsg(UID, 2, 616, 21215, NPC, 10, 101);
end

if (EVENT == 2000)then
SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
         else
SelectMsg(UID, 2, -1, 22274, NPC, 22,-1);
GiveItem(UID, 900291000,1)
end
end

if (EVENT == 104) then
	SelectMsg(UID, 2, -1, 12247, NPC, 8785, 105,8786,108);
end

if (EVENT == 105) then 
	SPELL = HowmuchItem(UID, 810369000);
	if (SPELL < 20) then
		SelectMsg(UID, 2, -1, 12248, NPC, 10, -1);
	else
	    EVENT = 106
	end
end

if (EVENT == 106) then
RobItem(UID, 810369000,20);
GiveItem(UID, 900823000,1);
	SelectMsg(UID, 2, -1, 12249, NPC,8787,-1);
end
--27  item parçalama
--15 maradon scene


if (EVENT == 200) then
	SelectMsg(UID, 53, -1, NPC);
end