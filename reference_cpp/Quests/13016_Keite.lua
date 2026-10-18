-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local Ret = 0;
local NPC = 13016;

if (EVENT == 500) then
SelectMsg(UID, 3, -1, 4834, NPC, 4263, 101, 4264, 102, 4265, 103, 7493,3002, 4337, 104, 7175, 5000, 4199, 3001);
QuestStatus = GetQuestStatus(UID, 78)	
if(QuestStatus == 0) then
	EVENT = 602
end

if(QuestStatus == 2) then
	EVENT = 800
end

if(QuestStatus == 3) then
	EVENT = 606
end
end
if (EVENT == 800) then--800
	SelectMsg(UID, 3, -1, 4834, NPC, 4263, 101, 4264, 102, 4265, 103, 7493,3002, 4337, 104, 7175, 5000, 4199, 3001);
end

if (EVENT == 104) then
	SelectMsg(UID, 3, -1, 820, NPC, 4344, 105, 4345, 106, 7551, 107, 4199, 3001);
end

if (EVENT == 3001) then
	Ret = 1;
end

if (EVENT == 101) then
	SelectMsg(UID, 9, -1, -1, NPC);
end

if (EVENT == 102) then
	ITEMA = HowmuchItem(UID, 800090000);
	if (ITEMA > 0) then
		-- Familiar Name Change Fonksiyonu yok
	else
		SelectMsg(UID, 2, -1, 4833, NPC, 27, 3001);
	end
end	      

if (EVENT == 103) then
	SelectMsg(UID, 14, -1, NPC);
end    

if (EVENT == 105) then
	MagicBag = HowmuchItem(UID, 800440000); 
	if (MagicBag < 1 or MagicBag == 0) then
		SelectMsg(UID, 2, -1, 823, NPC, 10, 3001);
	else
		SelectMsg(UID, 4, 6935, 4834, NPC, 4006, 150, 4005, -1);
	end
end


if (EVENT == 150) then
	MagicBag = HowmuchItem(UID, 800440000);
	if (MagicBag < 1 or MagicBag == 0) then
		SelectMsg(UID, 2, -1, 823, NPC, 10, 3001);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,653, 1);
		end
	end
end


if (EVENT == 106) then
	AUTOPET = HowmuchItem(UID, 800450000); 
	if (AUTOPET < 1 or AUTOPET == 0) then
		SelectMsg(UID, 2, -1, 824, NPC, 10, 3001);
	else
		SelectMsg(UID, 4, 6936, 4834, NPC, 4006, 151, 4005, -1);
	end
end


if (EVENT == 151) then
	AUTOPET = HowmuchItem(UID, 800450000);
	if (AUTOPET < 1 or AUTOPET == 0) then
		SelectMsg(UID, 2, -1, 824, NPC, 10, 3001);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,654, 1);
		end
	end
end


if EVENT == 107 then 
HPMAESTRO   = HowmuchItem(UID, 810115000);
MPMAESTRO   = HowmuchItem(UID, 810116000);
if(HPMAESTRO > 0 and MPMAESTRO > 0) then
          SelectMsg(UID, 3, -1, 4834, NPC, 7554, 108, 7555, 110);
 elseif(HPMAESTRO  > 0) then
          EVENT = 108
 elseif(MPMAESTRO > 0) then
          EVENT = 110
 else
        SelectMsg(UID, 2, -1, 44826, NPC, 10, 3001);
	end
end

if EVENT == 108 then 
HPMAESTRO  = HowmuchItem(UID, 810115000);
   if(HPMAESTRO  > 0) then
   SelectMsg(UID, 4, 6937, 44412, NPC, 4006, 109, 4005, -1);
   else
   SelectMsg(UID, 2, -1, 44826, NPC, 10, 3001);
	end
end
 

if EVENT == 109 then 
HPMAESTRO  = HowmuchItem(UID, 810115000);
	if(HPMAESTRO  < 1) then
		SelectMsg(UID, 2, -1, 44412, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
    if SlotCheck == false then
    else
		RunGiveItemExchange(UID,655, 1);
		end
	end
end


if EVENT == 110 then 
MPMAESTRO  = HowmuchItem(UID, 810116000);
   if(MPMAESTRO  > 0) then
   SelectMsg(UID, 4, 6938, 44412, NPC, 4006, 111, 4005, -1);
   else
   SelectMsg(UID, 2, -1, 44826, NPC, 10, 3001);
	end
end
 

if EVENT == 111 then 
MPMAESTRO  = HowmuchItem(UID, 810116000);
	if(MPMAESTRO  < 1) then
		SelectMsg(UID, 2, -1, 44826, NPC, 10, 3001);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
    if SlotCheck == false then
    else
	RunGiveItemExchange(UID,656, 1);
		end
	end
end

local savenum = 78;

if (EVENT == 602) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, savenum, 4704, NPC, 3000, 609, 3005, 3001);
	else
		SelectMsg(UID, 2, savenum, 4705, NPC, 10, 3001);
	end
end

if (EVENT == 609) then
	SelectMsg(UID, 4, savenum, 4706, NPC, 22, 603, 23, 604);
end

if (EVENT == 603) then
	SaveEvent(UID, 5402);
end

if (EVENT == 604) then
	SaveEvent(UID, 5405);
end

if (EVENT == 605) then
	SaveEvent(UID, 5404);
end

if (EVENT == 503) then
	SaveEvent(UID, 5404);
end

if (EVENT == 606) then
	MonsterCount = CountMonsterQuestSub(UID, 525, 1);
	if (MonsterSub > 10) then
		SelectMsg(UID, 2, savenum, 4721, NPC, 10, 607);   
	else
		SelectMsg(UID, 4, savenum, 4706, NPC, 22, 608, 23, 3001);
	end
end

if (EVENT == 607) then
	ShowMap(UID, 1);
end

if (EVENT == 608) then
	GiveItem(UID, 600001000, 1)
	GiveItem(UID, 389191000, 5)
	SaveEvent(UID, 5403);
end

if (EVENT == 3002) then
  SelectMsg(UID, 28, savenum, 4706, NPC, 22, 608, 23, 3001);
end

if (EVENT == 5000) then 
	--InsertRepurchase(UID)
	CHAMPIONSHIP = HowmuchItem(UID, 0000000000); --item kodu gerekli..
	if (CHAMPIONSHIP < 1 or CHAMPIONSHIP == 0) then
		SelectMsg(UID, 2, -1, 44826, NPC, 10, -1);
	else
		SelectMsg(UID, 2, -1, 12458, NPC, -1, -1,-1,-1);--- görevin ne verdiğini öğrenince yazılacak..
	end
end

