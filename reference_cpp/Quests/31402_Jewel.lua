-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local UserClass;
local QuestNum;
local Ret = 0;
local NPC =31402;

if (EVENT == 100) then
	QuestStatus = GetQuestStatus(UID, 698)	
	if(QuestStatus == 1) then
		EVENT = 2000
		else
	SelectMsg(UID, 20, -1, 845, NPC, 4520, 101, 7056, 102, 7060, 103, 7057, 104, 7058, 105);
end
end

if EVENT == 101 then -- Chating Menüsü
  SelectMsg(UID,19, -1, 1660, NPC,2003, 100,40197,150);
end

if EVENT == 150 then -- Chating Menüsü
  SelectMsg(UID,19, -1, 43761, NPC,40198, 151);
end

if EVENT == 151 then -- Chating Menüsü
  SelectMsg(UID,19, -1, 43762, NPC,40199, 152,40200,-1);
end

if EVENT == 152 then -- Chating Menüsü
  SelectMsg(UID,19, -1, 43763, NPC,10,153);
end

if EVENT == 153 then -- Chating Menüsü
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
GiveItem(UID, 900608000, 1);
end
end

if EVENT == 102 then -- accesories ask menü
  SelectMsg(UID,19, -1, 1661, NPC,2003, 100);
end

if EVENT == 103 then
  SelectMsg(UID ,19, -1, 1663, NPC,4527, 110, 4528, 112);
end

if (EVENT == 110) then --  -- Soft Stone All
	MONEY = HowmuchItem(UID, 900000000);
	if (MONEY >= 1000000) then
		GoldLose(UID, 1000000);
		GiveItem(UID, 810101000, 1);
		SelectMsg(UID, 19, -1, 1664, NPC,6002,570);		
		else
		SelectMsg(UID, 3, 974, 1665, NPC, 18, 204);
    end
end


if EVENT == 104 then
   SelectMsg(UID, 19, -1, 848, NPC, 7059, 106, 4525, 168);	
end



if EVENT == 106 then
   SelectMsg(UID, 18, -1, -1, NPC);	
end

if EVENT == 168 then
   Ret = 1;
end

if EVENT == 105 then
   SelectMsg(UID, 21, -1, -1, NPC, -1, -1 );	
end


if (EVENT == 2000)then
SelectMsg(UID, 2, -1, 22271, NPC, 10,2001);
end

if (EVENT == 2001)then
SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
         else
SelectMsg(UID, 2, -1, 22273, NPC, 22,-1);
GiveItem(UID, 900218000,1);
end
end