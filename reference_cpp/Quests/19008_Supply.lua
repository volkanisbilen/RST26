local Ret = 0;
local NPC = 19008;

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 8903, NPC, 4519, 101);
end   

if(EVENT == 101) then
	SelectMsg(UID, 2, -1, 9208, NPC, 40206, 102)
end

if(EVENT == 102) then
	Class = CheckClass(UID);
    if (Class == 1 or Class == 5 or Class == 6) then
		EVENT = 103
    elseif (Class == 2 or Class == 7 or Class == 8) then
		EVENT = 113
    elseif (Class == 3 or Class == 9 or Class == 10) then
		EVENT = 123
    elseif (Class == 4 or Class == 11 or Class == 12) then
		EVENT = 133
	end
end

if(EVENT == 103) then
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	if( LEVEL > 59 and COIN > 29999999 ) then
		SelectMsg(UID, 2, -1, 8889, NPC, 4466, 104, 3019, 193 )
	elseif(LEVEL > 35 and LEVEL < 60 and COIN > 599999) then
		SelectMsg(UID, 2, -1, 8885, NPC, 4466, 105, 3019, 193 )
	else
		EVENT = 193
	end
end

if(EVENT == 113) then
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	if( LEVEL > 59 and COIN > 29999999 ) then
		SelectMsg(UID, 2, -1, 8890, NPC, 4466, 114, 3019, 193 )
	elseif(LEVEL > 35 and LEVEL < 60 and COIN > 599999) then
		SelectMsg(UID, 2, -1, 8886, NPC, 4466, 115, 3019, 193 )
	else
		EVENT = 193
	end
end

if(EVENT == 123) then
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	if( LEVEL > 59 and COIN > 29999999 ) then
		SelectMsg(UID, 2, -1, 8891, NPC, 4466, 124, 3019, 193 )
	elseif(LEVEL > 35 and LEVEL < 60 and COIN > 599999) then
		SelectMsg(UID, 2, -1, 8887, NPC, 4466, 125, 3019, 193 )
	else
		EVENT = 193
	end
end

if(EVENT == 133) then
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	if( LEVEL > 59 and COIN > 29999999 ) then
		SelectMsg(UID, 2, -1, 8892, NPC, 4466, 134, 3019, 193 )
	elseif(LEVEL > 35 and LEVEL < 60 and COIN > 599999) then
		SelectMsg(UID, 2, -1, 8888, NPC, 4466, 135, 3019, 193 )
	else
		EVENT = 193
	end
end

if(EVENT == 193 ) then
	SelectMsg(UID, 2, -1, 9503, NPC, 10, -1 )
end

if(EVENT == 104) then --Warrior
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000); 
	WarriorSlot = CheckGiveSlot(UID, 6)
	if WarriorSlot == false then
		SelectMsg(UID,2,-1,8900,NPC,10,1000)
	else
	if( LEVEL > 59 and COIN > 29999999 ) then
		GiveItem(UID, 906001548, 1, 1)
		GiveItem(UID, 906002549, 1, 1)
		GiveItem(UID, 906003550, 1, 1)
		GiveItem(UID, 906004551, 1, 1)
		GiveItem(UID, 906005552, 1, 1)
		GiveItem(UID, 956210544, 1, 1)
		GoldLose(UID, 30000000)
	else
		SelectMsg(UID, 2, -1, 8893, NPC, 4466, 106, 3019, 193 )
		end
	end
end

if(EVENT == 105) then
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	WarriorSlot = CheckGiveSlot(UID, 6)
	if WarriorSlot == false then
		SelectMsg(UID,2,-1,8900,NPC,10,1000)
	else
	if( LEVEL > 35 and LEVEL < 60 and COIN > 29999999 ) then
		GiveItem(UID, 956112518, 1, 1)
		GiveItem(UID, 905001524, 1, 1)
		GiveItem(UID, 905002525, 1, 1)
		GiveItem(UID, 905003526, 1, 1)
		GiveItem(UID, 905004527, 1, 1)
		GiveItem(UID, 905005528, 1, 1)
		GoldLose(UID, 30000000)
	else
		SelectMsg(UID, 2, -1, 8893, NPC, 4466, 106, 3019, 193 )
		end
	end
end

if(EVENT == 114) then -- Rogue
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000); 
	RogueSlot = CheckGiveSlot(UID, 8)
	if RogueSlot == false then
		SelectMsg(UID,2,-1,8900,NPC,10,1000)
	else
	if( LEVEL > 59 and COIN > 29999999 ) then
		GiveItem(UID, 911210545, 1, 1);
		GiveItem(UID, 911210545, 1, 1);
		GiveItem(UID, 968410546, 1, 1);
		GiveItem(UID, 946001553, 1, 1);
		GiveItem(UID, 946002554, 1, 1);
		GiveItem(UID, 946003555, 1, 1);
		GiveItem(UID, 946004556, 1, 1);
		GiveItem(UID, 946005557, 1, 1);	
		GoldLose(UID, 30000000)
	else
		SelectMsg(UID, 2, -1, 8893, NPC, 4466, 106, 3019, 193 )
		end
	end
end

if(EVENT == 115) then
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	RogueaSlot = CheckGiveSlot(UID, 7)
	if RogueaSlot == false then
		SelectMsg(UID,2,-1,8900,NPC,10,1000)
	else
	if( LEVEL > 35 and LEVEL < 60 and COIN > 29999999 ) then
		GiveItem(UID, 911110519, 1, 1)
		GiveItem(UID, 911110519, 1, 1)
		GiveItem(UID, 960450520, 1, 1)		
		GiveItem(UID, 945001529, 1, 1)
		GiveItem(UID, 945002530, 1, 1)
		GiveItem(UID, 945003531, 1, 1)
		GiveItem(UID, 945004532, 1, 1)
		GiveItem(UID, 945005533, 1, 1)
		GoldLose(UID, 30000000);
	else
		SelectMsg(UID, 2, -1, 8893, NPC, 4466, 106, 3019, 193 )
		end
	end
end

if(EVENT == 124) then -- Mage
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	MageSlot = CheckGiveSlot(UID, 8)
	if MageSlot == false then
		SelectMsg(UID,2,-1,8900,NPC,10,1000)
	else
	if( LEVEL > 59 and COIN > 29999999 ) then
		GiveItem(UID, 981110570, 1, 3);
		GiveItem(UID, 981110571, 1, 3);
		GiveItem(UID, 981110547, 1, 3);
		GiveItem(UID, 966001558, 1, 3);
		GiveItem(UID, 966002559, 1, 3);
		GiveItem(UID, 966003560, 1, 3);
		GiveItem(UID, 966004561, 1, 3);
		GiveItem(UID, 966005562, 1, 3);
		GoldLose(UID, 30000000);
	else              
		SelectMsg(UID, 2, -1, 8893, NPC, 4466, 106, 3019, 193 )
		end
	end
end

if(EVENT == 125) then
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	MageSlot = CheckGiveSlot(UID, 8)
	if MageSlot == false then
		SelectMsg(UID,2,-1,8900,NPC,10,1000)
	else
	if( LEVEL > 35 and LEVEL < 60 and COIN > 29999999 ) then
		GiveItem(UID, 980812521, 1, 1)
		GiveItem(UID, 980812568, 1, 1)
		GiveItem(UID, 980812569, 1, 1)
		GiveItem(UID, 965001534, 1, 1)
		GiveItem(UID, 965002535, 1, 1)
		GiveItem(UID, 965003536, 1, 1)
		GiveItem(UID, 965004537, 1, 1)
		GiveItem(UID, 965005538, 1, 1)
		GoldLose(UID, 30000000)
	else
		SelectMsg(UID, 2, -1, 8893, NPC, 4466, 106, 3019, 193 )
		end
	end
end

if(EVENT == 134) then -- Priest
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000);  
	PriestSlot = CheckGiveSlot(UID, 7)
	if PriestSlot == false then
		SelectMsg(UID,2,-1,8900,NPC,10,1000)
	else
	if( LEVEL > 59 and COIN > 29999999 ) then
		GiveItem(UID, 990250522, 1, 1);
		GiveItem(UID, 979101523, 1, 1);
		GiveItem(UID, 986001563, 1, 1);
		GiveItem(UID, 986002564, 1, 1);
		GiveItem(UID, 986003565, 1, 1);
		GiveItem(UID, 986004566, 1, 1);
		GiveItem(UID, 986005567, 1, 1);
		GoldLose(UID, 30000000)
	else
		SelectMsg(UID, 2, -1, 8893, NPC, 4466, 106, 3019, 193 )
		end
	end
end

if(EVENT == 135) then
	LEVEL = GetLevel(UID)
	COIN = HowmuchItem(UID, 900000000); 
	PriestSlot = CheckGiveSlot(UID, 7)
	if PriestSlot == false then
		SelectMsg(UID,2,-1,8900,NPC,10,1000)
	else
	if( LEVEL > 35 and LEVEL < 60 and COIN > 29999999 ) then
		GiveItem(UID, 990250522, 1, 1);
		GiveItem(UID, 979101523, 1, 1);
		GiveItem(UID, 985001539, 1, 1);
		GiveItem(UID, 985002540, 1, 1);
		GiveItem(UID, 985003541, 1, 1);
		GiveItem(UID, 985004542, 1, 1);
		GiveItem(UID, 985005543, 1, 1);
		GoldLose(UID, 30000000);
	else
		SelectMsg(UID, 2, -1, 8893, NPC, 4466, 106, 3019, 193 )
		end
	end
end

if (EVENT == 193) then
	Ret = 1;
end

if (EVENT == 3001) then
	SelectMsg(UID, 2, -1, 8947, NPC, 4446, 193);
end

