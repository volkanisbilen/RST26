-- =============================================
-- RİMA ACS  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
----------------------------------------------------------------------------------------------------------------------
local NPC = 29056;
----------------------------------------------------------------------------------------------------------------------

if (EVENT == 100) then 
	SelectMsg(UID, 3, -1, 3018, NPC,7496,550,40102,110,7202,400,7214,420,7219,430,7228,460,45237,556); 
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 101) then -- Fortune Pocket Event
	FORTUNEPOCKET = HowmuchItem(UID, 810449000);
	if (FORTUNEPOCKET < 1) then
		SelectMsg(UID, 2, -1, 44415, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 12367, NPC, 4302, 102, 4303, -1);
	end
end

if EVENT == 102 then 
SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
        else
Check = CheckExchange(UID, 000)
   if  Check == true then   
   Roll = RollDice(UID, 00) 
   found = Roll + 000
   RunQuestExchange(UID, found);
		end
    end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 110) then 
	SelectMsg(UID, 3, -1, 3018, NPC,7685,116,7247,180,7248,185,7258,500,4296,100);
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 116) then -- Official List
	LIST = HowmuchItem(UID, 810163000);
	if (LIST < 1 or LIST == 0) then
		SelectMsg(UID, 2, -1, 10596, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6897, 10597, NPC, 4006, 117, 4005, -1);
	end
end

if (EVENT == 117) then
	LIST = HowmuchItem(UID, 810163000);
	if (LIST < 1 or LIST == 0) then
		SelectMsg(UID, 2, -1, 10596, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,628, 1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 129) then -- Tour Event
	TOUR = HowmuchItem(UID, 810391000);
	TOUR2 = HowmuchItem(UID, 810392000);
	if (TOUR < 1) then
		SelectMsg(UID, 2, -1, 12168, NPC, 10,-1);
	elseif (TOUR2 < 1) then
	    SelectMsg(UID, 2, -1, 12168, NPC, 10,-1);
	else
		SelectMsg(UID, 2, -1, 12167, NPC,4297,130,27,-1);
	end
end

if (EVENT == 130) then
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RobItem(UID, 00000, 1);
	GiveItem(UID, 0000, 1);
    end
end

----------------------------------------------------------------------------------------------------------------------

--if (EVENT == 180) then -- DC Premium Kontrol
--	DCPRE = GetPremium(UID);	
--	if (DCPRE == 10) then
--	    EVENT = 181
--	else
--		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
--	end
--end


if (EVENT == 180) then -- Golden Pickaxe 
	PICKAX = HowmuchItem(UID, 508122000);
	if (PICKAX < 1 or PICKAX == 0) then
		SelectMsg(UID, 2, -1, 9943, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6895, 9944, NPC, 4297, 182, 4005, -1);
	end
end

if (EVENT == 182) then
	PICKAX = HowmuchItem(UID, 508122000);
	if (PICKAX < 1 or PICKAX == 0) then
		SelectMsg(UID, 2, -1, 9943, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,620, 1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

--if (EVENT == 185) then  -- Exp Premium Kontrol
--	EXP = GetPremium(UID);
--	if (EXP == 11) then
--	    EVENT = 186
--	else
--		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
--	end
--end

if (EVENT == 185) then -- Golden Fishing Rod
	FISHING = HowmuchItem(UID, 508121000);
	if (FISHING < 1) then
		SelectMsg(UID, 2, -1, 9945, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6896, 9946, NPC, 4297, 187, 4005, -1);

	end
end

if (EVENT == 187) then
	FISHING = HowmuchItem(UID, 508121000);
	if (FISHING < 1) then
		SelectMsg(UID, 2, -1, 9945, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange (UID,630,1)
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 400) then--7803,404,10005,405,8574,406, -- Premium Menu
	SelectMsg(UID, 3, -1, 9527, NPC, 7197, 401, 7198, 402, 7252, 403,8390,600, 45241,750,4296,100);
end

if (EVENT == 401) then -- DISC Premium
	SelectMsg(UID, 2, -1, 9529, NPC, 3000, 407, 3005, -1);
end

if (EVENT == 402) then -- EXP Premium
	SelectMsg(UID, 2, -1, 9544, NPC, 3000, 409, 3005, -1);
end

if (EVENT == 403) then -- WAR Premium
	SelectMsg(UID, 2, -1, 9954, NPC, 3000, 411, 3005, -1);
end

if (EVENT == 404) then -- Gold Premium
	SelectMsg(UID, 2, -1, 10881, NPC, 3000, 413, 3005, -1);
end

if (EVENT == 405) then -- Bronze Premium
	SelectMsg(UID, 2, -1, 12068, NPC, 3000, 415, 3005, -1);
end

if (EVENT == 406) then -- Platinum Premium
	SelectMsg(UID, 2, -1, 9532, NPC, 3000, 417, 3005, -1);
end

if (EVENT == 600) then -- Switch Premium
	SelectMsg(UID, 2, -1, 9532, NPC, 3000, 601, 3005, -1);
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 407) then -- DISC Premium
	DCPREM = HowmuchItem(UID, 399281685);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
	else
		EVENT = 408
	end
end

if (EVENT == 408) then
	DCPREM = HowmuchItem(UID, 399281685);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
		else
		RobItem(UID, 399281685, 1);
		GivePremium(UID, 10, 30);
		GivePremiumItem(UID,10)
	end	
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 409) then -- EXP Premium
	EXPPREM = HowmuchItem(UID, 399282686);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
    EVENT = 410
	end
end

if (EVENT == 410) then
	EXPPREM = HowmuchItem(UID, 399282686);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
	RobItem(UID, 399282686, 1);
	GivePremium(UID, 11, 30);
	GivePremiumItem(UID,11);
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 411) then -- WAR Premium
	WARPREM = HowmuchItem(UID, 399292764);
	if (WARPREM < 1 or WARPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
    EVENT = 412
	end
end

if (EVENT == 412) then
	WARPREM = HowmuchItem(UID, 399292764);
	if (WARPREM < 1 or WARPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
	RobItem(UID, 399292764, 1);
	GivePremium(UID, 12, 30);
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6) then
			GivePremiumItem(UID,14);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			GivePremiumItem(UID,14);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			GivePremiumItem(UID,12);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			GivePremiumItem(UID,12);			
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 413) then -- Gold Premium
	GOLDPRE = HowmuchItem(UID, 0);
	if (GOLDPRE < 1 or GOLDPRE == 0) then
		SelectMsg(UID, 2, -1, 10882, NPC, 18, 5000);
	else
    EVENT = 414
	end
end
	
if (EVENT == 414) then
	GOLDPRE = HowmuchItem(UID, 0);
	if (GOLDPRE < 1 or GOLDPRE == 0) then
		SelectMsg(UID, 2, -1, 10882, NPC, 18, 5000);
	else
	RobItem(UID, 0, 1);
	GivePremium(UID, 5, 30);
	GivePremiumItem(UID,5);
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 415) then -- Bronze Premium
	BRONZEPRE = HowmuchItem(UID, 814042000);
	if (BRONZEPRE < 1 or BRONZEPRE == 0) then
		SelectMsg(UID, 2, -1, 12069, NPC, 18, 5000);
	else
    EVENT = 416
	end
end

if (EVENT == 416) then
	BRONZEPRE = HowmuchItem(UID, 814042000);
	if (BRONZEPRE < 1 or BRONZEPRE == 0) then
		SelectMsg(UID, 2, -1, 12069, NPC, 18, 5000);
	else	
	RobItem(UID, 814042000, 1);
	GivePremium(UID, 3, 30);
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 417) then -- Platinum Premium
	PLATPRE = HowmuchItem(UID, 800880000);
	if (PLATPRE < 1 or PLATPRE == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
    EVENT = 418
	end
end

if (EVENT == 418) then
	PLATPRE = HowmuchItem(UID, 800880000);
	if (PLATPRE < 1 or PLATPRE == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
		RobItem(UID, 800880000, 1);
		GivePremium(UID, 7, 30);
		GivePremiumItem(UID,7);
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 601) then -- Switch Premium
	SWITCH = HowmuchItem(UID, 399295859);
	if (SWITCH < 1 or SWITCH == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
    EVENT = 602
	end
end

if (EVENT == 602) then
	SWITCH = HowmuchItem(UID, 399295859);
	if (SWITCH < 1 or SWITCH == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
		RobItem(UID, 399295859, 1);
		GiveSwitchPremium(UID, 10, 30);
		GiveSwitchPremium(UID, 11, 30);
		GiveSwitchPremium(UID, 12, 30);
        Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6) then
			GivePremiumItem(UID,13);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			GivePremiumItem(UID,13);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			GivePremiumItem(UID,13);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			GivePremiumItem(UID,13);		
		end
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 750) then -- Clan Premium
    CheckLeader = isClanLeader(UID) 
    if (CheckLeader == false) then
        SelectMsg(UID, 2, -1, 45035, NPC, 10, -1);
    else
    CLANPREM = HowmuchItem(UID, 399300914);
    if (CLANPREM < 1) then
        SelectMsg(UID, 2, -1, 45033, NPC, 18, -1);
    else
        RobItem(UID, 399300914, 1);
        GiveClanPremium(UID,2,30)
		SelectMsg(UID, 2, -1, 45034, NPC, 10, -1);
		end
    end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 420) then -- Minerva Pack
	MINEVRAPACK = HowmuchItem(UID, 508112000);
	if (MINEVRAPACK < 1 or MINEVRAPACK == 0) then
		SelectMsg(UID, 2, -1, 9622, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6898, 9621, NPC, 4006, 421, 4005, -1);
    end
end

if (EVENT == 421) then
	MINEVRAPACK = HowmuchItem(UID, 508112000);
	if (MINEVRAPACK < 1 or MINEVRAPACK == 0) then
		SelectMsg(UID, 2, -1, 9622, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
         else
		RunGiveItemExchange (UID,629, 1)
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 430) then  -- Pathos Pack
	PATHOSPACK = HowmuchItem(UID, 508074000);
	if (PATHOSPACK < 1 or PATHOSPACK == 0) then
		SelectMsg(UID, 2, -1, 9629, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6899, 9628, NPC, 4006, 431, 4005, -1);
    end
end

if (EVENT == 431) then
	PATHOSPACK = HowmuchItem(UID, 508074000);
	if (PATHOSPACK < 1 or PATHOSPACK == 0) then
		SelectMsg(UID, 2, -1, 9629, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
         else
		RunGiveItemExchange (UID,626, 1)
		end
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 460) then -- Seal Menu
SelectMsg(UID,2,-1,9706,NPC,7229,461,7230,463,4296,100)
end

if (EVENT == 461) then -- Seal 10
	SEAL10 = HowmuchItem(UID, 810520000);
	if (SEAL10 < 1 or SEAL10 == 0) then
		SelectMsg(UID, 2, -1, 9706, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6889, 9706, NPC, 4006, 462, 4005, -1);
    end
end

if (EVENT == 462) then
	SEAL10 = HowmuchItem(UID, 810520000);
	if (SEAL10 < 1 or SEAL10 == 0) then
		SelectMsg(UID, 2, -1, 9706, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RunGiveItemExchange (UID,634, 1)
		end
	end
end

if (EVENT == 463) then -- Seal 50
	SEAL50 = HowmuchItem(UID, 810700000);
	if (SEAL50 < 1 or SEAL50 == 0) then
		SelectMsg(UID, 2, -1, 9707, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6890, 9707, NPC, 4006, 464, 4005, -1);
    end
end

if (EVENT == 464) then
	SEAL50 = HowmuchItem(UID, 810700000);
	if (SEAL50 < 1 or SEAL50 == 0) then
		SelectMsg(UID, 2, -1, 9707, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RunGiveItemExchange (UID,635, 1)
		end
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 500) then -- Peri Menu
	SelectMsg(UID, 3, -1, 9989, NPC, 7259, 501, 7260, 503, 7314, 505, 45240, 507);
end

if (EVENT == 501) then -- Dryads Peri
	DRYADS = HowmuchItem(UID, 800389000);
	if (DRYADS < 1 or DRYADS == 0) then
		SelectMsg(UID, 2, -1, 9991, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6892, 9990, NPC, 4006, 502, 4005, -1);
	end
end

if (EVENT == 502) then
	DRYADS = HowmuchItem(UID, 800389000);
	if (DRYADS < 1 or DRYADS == 0) then
		SelectMsg(UID, 2, -1, 9991, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange (UID,631,1)
		end
	end
end

if (EVENT == 503) then -- Oreads Peri
	OREADS = HowmuchItem(UID, 800386000);
	if (OREADS < 1 or OREADS == 0) then
		SelectMsg(UID, 2, -1, 9992, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6893, 9990, NPC, 4006, 504, 4005, -1);
	end
end

if (EVENT == 504) then
	OREADS = HowmuchItem(UID, 800386000);
	if (OREADS < 1 or OREADS == 0) then
		SelectMsg(UID, 2, -1, 9992, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange (UID,632,1)
		end
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 505) then  -- Alseids Peri
	ALSEIDS = HowmuchItem(UID, 800387000);
	if (ALSEIDS < 1 or ALSEIDS == 0) then
		SelectMsg(UID, 2, -1, 9992, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6894, 9990, NPC, 4006, 506, 4005, -1);
	end
end

if (EVENT == 506) then
	ALSEIDS = HowmuchItem(UID, 800387000);
	if (ALSEIDS < 1 or ALSEIDS == 0) then
		SelectMsg(UID, 2, -1, 9992, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange (UID,633,1)
		end
	end
end
----------------------------------------------------------------------------------------

if (EVENT == 507) then  -- Nereids Peri
	NEREIDS = HowmuchItem(UID, 814661000);
	if (NEREIDS < 1 or NEREIDS == 0) then
		SelectMsg(UID, 2, -1, 9992, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6945, 9990, NPC, 4006, 508, 4005, -1);
	end
end

if (EVENT == 508) then
	NEREIDS = HowmuchItem(UID, 814661000);
	if (NEREIDS < 1 or NEREIDS == 0) then
		SelectMsg(UID, 2, -1, 9992, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange (UID,663,1)
		end
	end
end
----------------------------------------------------------------------------------------

if (EVENT == 550) then -- Helmet Of Wraith
	HELWRATH = HowmuchItem(UID, 800451000);
	if (HELWRATH < 1 or HELWRATH == 0) then
		SelectMsg(UID, 2, -1, 43604, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6891, 43605, NPC, 4006, 551, 4005, -1);
	end
end

if (EVENT == 551) then
	HELWRATH = HowmuchItem(UID, 800451000);
	if (HELWRATH < 1 or HELWRATH == 0) then
		SelectMsg(UID, 2, -1, 43605, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,627, 1);
		end
	end
end

----------------------------------------------------------------------------------------
	
if (EVENT == 556) then -- Wing Of Dragon
	SelectMsg(UID, 3, -1, 3018, NPC,7679,205,45238,552,45239,35264, 40536,111,8611,118,9017,138,4296, 100);
end

if (EVENT == 205) then
	ITEMDRGN = HowmuchItem(UID, 810164000);
	if (ITEMDRGN < 1 or ITEMDRGN == 0) then
		SelectMsg(UID, 2, -1, 10593, NPC, 18, 5000);
	else
	NATION = CheckNation(UID);
	if (NATION == 1 ) then
		SelectMsg(UID, 5, 6900, 10592, NPC, 4006, 206, 4005,-1);
	elseif (NATION == 2 ) then
		SelectMsg(UID, 5, 6901, 10592, NPC, 4006, 207, 4005,-1);
		end
	end
end


if (EVENT == 206) then
	ITEMDRGN = HowmuchItem(UID, 810164000);
	if (ITEMDRGN < 1 or ITEMDRGN == 0) then
		SelectMsg(UID, 2, -1, 43605, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RunQuestExchange(UID,6900,STEP,1);	
		end
	end
end

if (EVENT == 207) then
	ITEMDRGN = HowmuchItem(UID, 810164000);
	if (ITEMDRGN < 1 or ITEMDRGN == 0) then
		SelectMsg(UID, 2, -1, 43605, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RunQuestExchange(UID,6901,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 552) then  -- War Wing
	WarWing = HowmuchItem(UID, 811164000);
	if (WarWing < 1 or WarWing == 0) then
		SelectMsg(UID, 2, -1, 45070, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6902, 43605, NPC, 4006, 553, 4005, -1);
	end
end

if (EVENT == 553) then
	WarWing = HowmuchItem(UID, 811164000);
	if (WarWing < 1 or WarWing == 0) then
		SelectMsg(UID, 2, -1, 45070, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,638, 1);
		end
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 35264) then -- Battle Hero Wing
	BattleHero = HowmuchItem(UID, 810251000);
	if (BattleHero < 1 or BattleHero == 0) then
		SelectMsg(UID, 2, -1, 45071, NPC, 18, 5000);
	else
	NATION = CheckNation(UID);
	if (NATION == 1 ) then
		SelectMsg(UID, 4, 6903, 10592, NPC, 4006, 554, 4005,-1);
	elseif (NATION == 2 ) then
		SelectMsg(UID, 4, 6904, 10592, NPC, 4006, 555, 4005,-1);
		end
	end
end


if (EVENT == 554) then
	BattleHero = HowmuchItem(UID, 810251000);
	if (BattleHero < 1 or BattleHero == 0) then
		SelectMsg(UID, 2, -1, 45071, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,639,STEP,1); 		
		end
	end
end

if (EVENT == 555) then
	BattleHero = HowmuchItem(UID, 810251000);
	if (BattleHero < 1 or BattleHero == 0) then
		SelectMsg(UID, 2, -1, 43605, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,640,STEP,1); 	
		end
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 111) then -- Cupid Wing
	CUPIDWING = HowmuchItem(UID, 810715000);
	if (CUPIDWING < 1 or CUPIDWING == 0) then
		SelectMsg(UID, 2, -1, 44440, NPC, 18, 5000);
	else
		SelectMsg(UID, 5, 6905, 44439, NPC, 4006, 112, 4005, -1);
	end
end

if (EVENT == 112) then
	CUPIDWING = HowmuchItem(UID, 810715000);
	if (CUPIDWING < 1 or CUPIDWING == 0) then
		SelectMsg(UID, 2, -1, 44440, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RunQuestExchange(UID,6905,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------

if (EVENT == 118) then -- Alencia Wing Menü
	SelectMsg(UID, 2, -1, 12056, NPC, 8629,119,8628,120);
end

----------------------------------------------------------------------------------------

if (EVENT == 119) then -- Alencia Wing Blue
	BLUEWING = HowmuchItem(UID, 810502000);
	if (BLUEWING < 1 or BLUEWING == 0) then
		SelectMsg(UID, 2, -1, 12061, NPC, 18,5000);
	else
		SelectMsg(UID, 5, 6911, 12060, NPC, 4006,121,4005,-1);
	end
end


if (EVENT == 121) then
	BLUEWING = HowmuchItem(UID, 810502000);
	if (BLUEWING < 1 or BLUEWING == 0) then
		SelectMsg(UID, 2, -1, 12061, NPC, 18,5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6911,STEP,1);
		end
    end
end

----------------------------------------------------------------------------------------

if (EVENT == 120) then -- Alencia Wing Red
	REDWING = HowmuchItem(UID, 810507000);
	if (REDWING < 1 or REDWING == 0) then
		SelectMsg(UID, 2, -1, 12058, NPC, 18,5000);
	else
		SelectMsg(UID, 5, 6912, 12057, NPC, 4006,125,4005,-1);
	end
end

if (EVENT == 125) then
	REDWING = HowmuchItem(UID, 810507000);
	if (REDWING < 1 or REDWING == 0) then
		SelectMsg(UID, 2, -1, 12058, NPC, 18,5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6912,STEP,1);
		end
    end
end

----------------------------------------------------------------------------------------

if (EVENT == 138) then -- Wings Of Hellfire
	Hellfire = HowmuchItem(UID, 810672000);
	if (Hellfire < 1 or Hellfire == 0) then
		SelectMsg(UID, 2, -1, 12461, NPC, 18, 5000);
	else
		SelectMsg(UID, 5, 6913, 12458, NPC, 4006, 156,4005,-1);
	end
end

if (EVENT == 156) then
	WING = HowmuchItem(UID, 810672000);
	if (WING == 0) then
		SelectMsg(UID, 2, -1, 12461, NPC, 10, -1);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6913,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------
if (EVENT == 5000) then
	ShowMap(UID, 450);
end