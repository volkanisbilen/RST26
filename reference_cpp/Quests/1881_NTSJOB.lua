-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 1881;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 45100, NPC, 45425, 101,45426,102,45427,103);
end

------------------------------------------------------------------------------------------------------------------------------
-- NATION TRANSFER EXCHANGE (IRK DEĞİŞTİRME)

if (EVENT == 101) then  ----------- NATION TRANSFER MENU
	SelectMsg(UID, 3, -1, 45056, NPC, 50537, 10000, 45423,410, 45424,699);
end

if (EVENT == 10000) then
	SelectMsg(UID, 3, 73, 1549, NPC, 10,10001);
end

if (EVENT == 10001) then
    SaveEvent(UID, 4070);
end

if (EVENT == 410) then
	SelectMsg(UID, 3, -1, 1522, NPC, 7014, 411,73,-1);
end

if (EVENT == 411) then
	SelectMsg(UID, 3, -1, 1533, NPC, 10, 412);
end


if (EVENT == 412) then
	SelectMsg(UID, 3, -1, 1534, NPC, 3000, 413,3005,-1);
end

if (EVENT == 413) then
	NTS = HowmuchItem(UID, 800360000);
		if NTS == 0 then
			SelectMsg(UID, 3, -1, 1532, NPC,10,-1);
		else
			RobItem(UID, 800360000, 1);
			GiveItem(UID, 810096000, 1,1);
   end
end

if (EVENT == 699) then
	NTS2 = HowmuchItem(UID, 810096000);
		if NTS2 == 0 then
			SelectMsg(UID, 3, -1, 1523, NPC, 18,5000);
		else
			SelectMsg(UID, 3, -1, 1524, NPC, 72, 700,73,-1);
	end
end

if (EVENT == 700) then
	NTS2 = HowmuchItem(UID, 810096000);
		if NTS2 == 0 then
			SelectMsg(UID, 3, -1, 1523, NPC,10,-1);
		else
			SendNationTransfer(UID);
	end
end
------------------------------------------------------------------------------------------------------------------------------

------------------------------------------------------------------------------------------------------------------------------
-- JOB CHANGE (JOB DEĞİŞTİRME)

if (EVENT == 102) then  
    JOBCHANGEITEM = HowmuchItem(UID, 700112000);
    if (JOBCHANGEITEM > 0) then
       -- NewJobChange(UID);
    else
	SelectMsg(UID, 3, 73, 45055, NPC, 10,10001);
    end
end

------------------------------------------------------------------------------------------------------------------------------

------------------------------------------------------------------------------------------------------------------------------
-- GENDER EXCHANGE (TİP DEĞİŞTİRME)

if (EVENT == 103) then  ----------- GENDER CHANGE  MENU
	SelectMsg(UID, 3, -1, 45036, NPC, 8975, 105, 8976, 991, 10,-1);
end

if (EVENT == 500) then
	SendGenderChange(UID);
end

if (EVENT == 1000) then
	ShowMap(UID, 450);
end

if (EVENT == 105) then
	SelectMsg(UID, 24, -1, -1, NPC);
end


if (EVENT == 991) then
	ItemGen = HowmuchItem(UID, 810594000);
	Nation = CheckNation(UID);	
	if (ItemGen >= 1) then
		if (Nation == 1) then
			SelectMsg(UID, 3, savenum, 147, NPC, 2002, 250);
		else
			SelectMsg(UID, 3, savenum, 147, NPC, 2002, 251);
		end
	else
		ShowMap(UID, 450);
	end
end

if (EVENT == 250) then
	WarriorC = isWarrior(UID);
	RogueC = isRogue(UID);
	MageC = isMage(UID);
	PriestC = isPriest(UID);
	if (WarriorC) then
		SelectMsg(UID, 3, savenum, 7108, NPC, 10, 101);
	elseif (RogueC) then
		SelectMsg(UID, 3, savenum, 7108, NPC, 10, 101);
	elseif (MageC) then
		SelectMsg(UID, 3, savenum, 147, NPC, 50534, 211, 50532, 212);
	elseif (PriestC) then
		SelectMsg(UID, 3, savenum, 147, NPC, 50531, 210, 50532, 212);
	end
end


if (EVENT == 251) then
	WarriorC = isWarrior(UID);
	RogueC = isRogue(UID);
	MageC = isMage(UID);
	PriestC = isPriest(UID);
	if (WarriorC) then
		SelectMsg(UID, 3, savenum, 147, NPC, 50533, 213, 50531, 214, 50532, 215);
	elseif (RogueC) then
		SelectMsg(UID, 3, savenum, 147, NPC, 50531, 214, 50532, 215);
	elseif (MageC) then
		SelectMsg(UID, 3, savenum, 147, NPC, 50531, 214, 50532, 215);
	elseif (PriestC) then
		SelectMsg(UID, 3, savenum, 147, NPC, 50531, 214, 50532, 215);
	end
end

--Karus Gender Change Effected
if (EVENT == 210) then
	GenderChange(UID, 2)
end

if (EVENT == 211) then
	GenderChange(UID, 3)
end

if (EVENT == 212) then
	GenderChange(UID, 4)
end

--Elmorad Gender Change Effected
if (EVENT == 213) then
	GenderChange(UID, 11)
end

if (EVENT == 214) then
	GenderChange(UID, 12)
end

if (EVENT == 215) then
	GenderChange(UID, 13)
end

------------------------------------------------------------------------------------------------------------------------------

