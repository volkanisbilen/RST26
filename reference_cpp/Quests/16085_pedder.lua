local NPC = 16085;

-----------------------------------// 1.Ana Menü	//-----------------------------------
if (EVENT == 100) then	
	SelectMsg(UID, 2, -1, 4131, NPC,45432, 200, 45217, 300, 45433, 400, 45441, 500, 45230, 550, 45231, 570, 45435, 600, 45434, 660, 45442, 680, 2002, 101); 
end
-----------------------------------// 2.Ana Menü	//-----------------------------------
if (EVENT == 101) then  
	SelectMsg(UID, 2, -1, 4131, NPC,45443, 700, 45444, 720, 45445, 750, 4344, 760, 45446, 780, 45437, 800, 45448, 820, 45451, 840, 45486, 860, 2002, 102);   
end
-----------------------------------// 3.Ana Menü	//-----------------------------------
if (EVENT == 102) then  
	SelectMsg(UID, 2, -1, 4131, NPC, 45487, 890, 8975, 1300, 8976, 1310, 4479, 1200, 2003, 101);   
end

if (EVENT == 5000) then -- Pus açar
	ShowMap(UID, 450);
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 200) then -- Change ID Menu
	SelectMsg(UID, 3, -1, 4131, NPC, 45438, 201, 45439, 202, 45209, 203, 4296, 100);
end

if (EVENT == 201) then -- NCS
	NCS = HowmuchItem(UID, 800032000);
	if (NCS < 1) then
		SelectMsg(UID, 2, -1, 4454, NPC, 18, 5000);
	else
		SendNameChange(UID);
	end
end

if (EVENT == 202) then -- Clan NCS
	CLANNCS = HowmuchItem(UID, 800086000);
	if (CLANNCS < 1) then
		SelectMsg(UID, 2, -1, 4670, NPC, 18, 5000);
	else
		Check = isClanLeader(UID)
		if (Check) then 
			SendClanNameChange(UID);
		else
			SelectMsg(UID, 2, -1, 4671, NPC, 10, -1);
		end
	end
end

if (EVENT == 203) then -- Tag Name Change
	TCS = HowmuchItem(UID, 800099000);
	if (TCS < 1) then
		SelectMsg(UID, 2, -1, 4454, NPC, 18, 5000);
	else
		SendTagNameChangePanel(UID);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 300) then  ----------- KNIGHT CASH MENU
	SelectMsg(UID, 3, -1, 45238, NPC, 45218, 301, 45219, 302, 45220, 303, 45222, 304, 45224, 305, 45226, 306, 4296, 100);
end

if (EVENT == 301) then  -- 100 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700082000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 45239, NPC, 18, 5000); --44302
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700082000,1);
		GiveBalance(UID, 100, 0); --bu şekilde bakiye verir

		
	end
end

if (EVENT == 302) then  -- 350 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700083000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 45240, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700083000,1);
		GiveBalance(UID, 350, 0);
	end
end

if (EVENT == 303) then  -- 700 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700084000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 45241, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700084000,1);
		GiveBalance(UID, 700, 0);
	end
end

if (EVENT == 304) then  -- 1200 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700089000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 45242, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700089000,1);
		GiveBalance(UID, 1200, 0);
	end
end

if (EVENT == 305) then -- 2100 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700079000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 45243, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700079000,1);
		GiveBalance(UID, 2100, 0);
	end
end


if (EVENT == 306) then  -- 10000 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700088000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 45244, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700088000,1);
		GiveBalance(UID, 10000, 0);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 400) then--7803,404,10005,405,8574,406, -- Premium Menu
	SelectMsg(UID, 3, -1, 45238, NPC, 7803, 900, 7197, 401, 7198, 420, 7252, 440, 8635, 460, 45241, 480, 4296, 100);
end

----------------------------------------------------------------------------------------------------------------------------------------------------------
if (EVENT == 900) then -- Gold Premium
	SelectMsg(UID, 2, -1, 9529, NPC, 7803, 901);
end

if (EVENT == 901) then -- Gold Premium ( 30 Days )
	DCPREM = HowmuchItem(UID, 300080930);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
	else
		EVENT = 902
	end
end

if (EVENT == 902) then
	DCPREM = HowmuchItem(UID, 300080930);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
		else
		SlotCheck = CheckGiveSlot(UID, 5)
     if SlotCheck == false then
    else   
		RobItem(UID, 300080930, 1);
		GiveItem(UID, 800013000, 1); -- 1500 HP+ Scroll(L)
		GiveItem(UID, 800010000, 1); -- 300 Defense+ Scroll(L)
		GiveItem(UID, 800014000, 1); -- Scroll of Attack
		GiveItem(UID, 800015000, 1); -- Speed-Up Potion
		GiveItem(UID, 800050000, 1); -- Mount Scroll - %50 EXP
		GivePremium(UID, 5, 30);
		end	
	end
end
----------------------------------------------------------------------------------------------------------------------------------------------------------
if (EVENT == 401) then -- DISC Premium
	SelectMsg(UID, 2, -1, 9529, NPC, 45452, 402, 45453, 404, 45454, 406, 45455, 408, 4296, 400);
end

if (EVENT == 402) then -- DC Premium ( 1 Day )
	DCPREM = HowmuchItem(UID, 399281915);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
	else
		EVENT = 403
	end
end

if (EVENT == 403) then
	DCPREM = HowmuchItem(UID, 399281915);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
		else
		RobItem(UID, 399281915, 1);
		GivePremium(UID, 10, 1);
	end	
end

if (EVENT == 404) then -- DC Premium ( 3 Days )
	DCPREM = HowmuchItem(UID, 399281916);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
	else
		EVENT = 405
	end
end

if (EVENT == 405) then
	DCPREM = HowmuchItem(UID, 399281916);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
		else
		RobItem(UID, 399281916, 1);
		GivePremium(UID, 10, 3);
	end	
end

if (EVENT == 406) then -- DC Premium ( 7 Days )
	DCPREM = HowmuchItem(UID, 399281917);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
	else
		EVENT = 407
	end
end

if (EVENT == 407) then
	DCPREM = HowmuchItem(UID, 399281917);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
		else
		RobItem(UID, 399281917, 1);
		GivePremium(UID, 10, 7);
	end	
end

if (EVENT == 408) then -- DC Premium ( 30 Days )
	DCPREM = HowmuchItem(UID, 399281685);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
	else
		EVENT = 409
	end
end

if (EVENT == 409) then
	DCPREM = HowmuchItem(UID, 399281685);
	if (DCPREM < 1 or DCPREM == 0) then
		SelectMsg(UID, 2, -1, 9530, NPC, 18, 5000);
		else
		SlotCheck = CheckGiveSlot(UID, 8)
     if SlotCheck == false then
    else  
		RobItem(UID, 399281685, 1);
		GiveItem(UID, 800013000, 1); -- 1500 HP+ Scroll(L)
		GiveItem(UID, 800010000, 1); -- 300 Defense+ Scroll(L)
		GiveItem(UID, 800014000, 1); -- Scroll of Attack
		GiveItem(UID, 800015000, 1); -- Speed-Up Potion
		GiveItem(UID, 810227000, 1); -- Genie Hammer
		GiveItem(UID, 700002000, 1); -- Trina's Piece
		GiveItem(UID, 389320000, 1); -- Premium Potion HP
		GiveItem(UID, 389350000, 1); -- Premium Potion MP
		GivePremium(UID, 10, 30);
		end	
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 420) then -- EXP Premium
	SelectMsg(UID, 2, -1, 9544, NPC, 45456, 421, 45457, 423, 45458, 425, 45459, 427, 4296, 400);
end

if (EVENT == 421) then -- EXP Premium ( 1 Day )
	EXPPREM = HowmuchItem(UID, 399282918);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
    EVENT = 422
	end
end

if (EVENT == 422) then
	EXPPREM = HowmuchItem(UID, 399282918);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
	RobItem(UID, 399282918, 1);
	GivePremium(UID, 11, 1);
	end
end

if (EVENT == 423) then -- EXP Premium ( 3 Days )
	EXPPREM = HowmuchItem(UID, 399282919);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
    EVENT = 424
	end
end

if (EVENT == 424) then
	EXPPREM = HowmuchItem(UID, 399282919);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
	RobItem(UID, 399282919, 1);
	GivePremium(UID, 11, 3);
	end
end

if (EVENT == 425) then -- EXP Premium ( 7 Days )
	EXPPREM = HowmuchItem(UID, 399282920);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
    EVENT = 426
	end
end

if (EVENT == 426) then
	EXPPREM = HowmuchItem(UID, 399282920);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
	RobItem(UID, 399282920, 1);
	GivePremium(UID, 11, 7);
	end
end

if (EVENT == 427) then -- EXP Premium ( 30 Days )
	EXPPREM = HowmuchItem(UID, 399282686);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
    EVENT = 428
	end
end

if (EVENT == 428) then
	EXPPREM = HowmuchItem(UID, 399282686);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9531, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 8)
     if SlotCheck == false then
    else  
	RobItem(UID, 399282686, 1);
	GiveItem(UID, 800013000, 1); -- 1500 HP+ Scroll(L)
	GiveItem(UID, 800010000, 1); -- 300 Defense+ Scroll(L)
	GiveItem(UID, 800014000, 1); -- Scroll of Attack
	GiveItem(UID, 800015000, 1); -- Speed-Up Potion
	GiveItem(UID, 700002000, 1); -- Trina's Piece
	GiveItem(UID, 800050000, 5); -- Mount Scroll - %50 EXP
	GiveItem(UID, 389320000, 1); -- Premium Potion HP
	GiveItem(UID, 389350000, 1); -- Premium Potion MP
	GivePremium(UID, 11, 30);
		end
	end
end
----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 440) then -- WAR Premium
	SelectMsg(UID, 2, -1, 9954, NPC, 45460, 441, 45461, 443, 45462, 445, 45463, 447, 4296, 400);
end

if (EVENT == 441) then -- WAR Premium ( 1 Day )
	EXPPREM = HowmuchItem(UID, 399292921);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
    EVENT = 442
	end
end

if (EVENT == 442) then
	EXPPREM = HowmuchItem(UID, 399292921);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
	RobItem(UID, 399292921, 1);
	GivePremium(UID, 12, 1);
	end
end

if (EVENT == 443) then -- WAR Premium ( 3 Days )
	EXPPREM = HowmuchItem(UID, 399292922);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
    EVENT = 444
	end
end

if (EVENT == 444) then
	EXPPREM = HowmuchItem(UID, 399292922);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
	RobItem(UID, 399292922, 1);
	GivePremium(UID, 12, 3);
	end
end

if (EVENT == 445) then -- WAR Premium ( 7 Days )
	EXPPREM = HowmuchItem(UID, 399292923);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
    EVENT = 446
	end
end

if (EVENT == 446) then
	EXPPREM = HowmuchItem(UID, 399292923);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
	RobItem(UID, 399292923, 1);
	GivePremium(UID, 12, 7);
	end
end

if (EVENT == 447) then -- WAR Premium ( 30 Days )
	EXPPREM = HowmuchItem(UID, 399292764);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
    EVENT = 448
	end
end

if (EVENT == 448) then
	EXPPREM = HowmuchItem(UID, 399292764);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9955, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 8)
     if SlotCheck == false then
    else   
	RobItem(UID, 399292764, 1);
	GiveItem(UID, 800013000, 1); -- 1500 HP+ Scroll(L)
	GiveItem(UID, 800010000, 1); -- 300 Defense+ Scroll(L)
	GiveItem(UID, 800014000, 1); -- Scroll of Attack
	GiveItem(UID, 800015000, 1); -- Speed-Up Potion
	GiveItem(UID, 700002000, 1); -- Trina's Piece
	GiveItem(UID, 800074000, 1); -- NP increase item
	GiveItem(UID, 389320000, 1); -- Premium Potion HP
	GiveItem(UID, 389350000, 1); -- Premium Potion MP
	GivePremium(UID, 12, 30);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 460) then -- Switching Premium
	SelectMsg(UID, 2, -1, 9532, NPC, 45464, 461, 45465, 463, 45466, 465, 45467, 467, 4296, 400);
end

if (EVENT == 461) then -- Switching Premium ( 1 Day )
	EXPPREM = HowmuchItem(UID, 399295925);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
    EVENT = 462
	end
end

if (EVENT == 462) then
	EXPPREM = HowmuchItem(UID, 399295925);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
	RobItem(UID, 399295925, 1);
	GiveSwitchPremium(UID, 10, 1);
	GiveSwitchPremium(UID, 11, 1);
	GiveSwitchPremium(UID, 12, 1);
	end
end

if (EVENT == 463) then -- Switching Premium ( 3 Day )
	EXPPREM = HowmuchItem(UID, 399295926);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
    EVENT = 464
	end
end

if (EVENT == 464) then
	EXPPREM = HowmuchItem(UID, 399295926);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
	RobItem(UID, 399295926, 1);
	GiveSwitchPremium(UID, 10, 3);
	GiveSwitchPremium(UID, 11, 3);
	GiveSwitchPremium(UID, 12, 3);
	end
end

if (EVENT == 465) then -- Switching Premium ( 7 Day )
	EXPPREM = HowmuchItem(UID, 399295927);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
    EVENT = 466
	end
end

if (EVENT == 466) then
	EXPPREM = HowmuchItem(UID, 399295927);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
	RobItem(UID, 399295927, 1);
	GiveSwitchPremium(UID, 10, 7);
	GiveSwitchPremium(UID, 11, 7);
	GiveSwitchPremium(UID, 12, 7);
	end
end

if (EVENT == 467) then -- Switching Premium ( 30 Day )
	EXPPREM = HowmuchItem(UID, 399295859);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
	else
    EVENT = 468
	end
end

if (EVENT == 468) then
	EXPPREM = HowmuchItem(UID, 399295859);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 9533, NPC, 18, 5000);
			else
SlotCheck = CheckGiveSlot(UID, 10)
     if SlotCheck == false then
	else
	RobItem(UID, 399295859, 1);
	GiveItem(UID, 800013000, 1); -- 1500 HP+ Scroll(L)
	GiveItem(UID, 800010000, 1); -- 300 Defense+ Scroll(L)
	GiveItem(UID, 800014000, 1); -- Scroll of Attack
	GiveItem(UID, 800015000, 1); -- Speed-Up Potion
	GiveItem(UID, 800050000, 5); -- Mount Scroll - %50 EXP
	GiveItem(UID, 700002000, 1); -- Trina's Piece
	GiveItem(UID, 800087000, 1); -- Spirit of Merchant
	GiveItem(UID, 700111000, 1); -- Voucher of Offline Merchant
	GiveItem(UID, 389320000, 1); -- Premium Potion HP
	GiveItem(UID, 389350000, 1); -- Premium Potion MP
	GiveSwitchPremium(UID, 10, 30);
	GiveSwitchPremium(UID, 11, 30);
	GiveSwitchPremium(UID, 12, 30);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 480) then -- Clan Premium
    CheckLeader = isClanLeader(UID) 
    if (CheckLeader == false) then
        SelectMsg(UID, 2, -1, 45035, NPC, 10, -1);
    else
    CLANPREM = HowmuchItem(UID, 399300914);
    if (CLANPREM < 1) then
        SelectMsg(UID, 2, -1, 45245, NPC, 18, 5000);
    else
        RobItem(UID, 399300914, 1);
        GiveClanPremium(UID,2,30)
		SelectMsg(UID, 2, -1, 45034, NPC, 10, -1);
		end
    end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 500) then -- Genie Voucher
	SelectMsg(UID, 2, -1, 45238, NPC, 45468, 501, 45469, 503, 45470, 505, 45471, 507, 45472, 509, 4296, 100);
end

if (EVENT == 501) then -- Genie Voucher ( 2 Hours ) 
	EXPPREM = HowmuchItem(UID, 700092000);
	if (EXPPREM < 1) then
		SelectMsg(UID, 2, -1, 45246, NPC, 18, 5000);
	else
		GenieExchange(UID, 700092000, 2);
	end
end

if (EVENT == 503) then -- Genie Voucher ( 1 Day ) 
	EXPPREM = HowmuchItem(UID, 700093000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45246, NPC, 18, 5000);
	else
		GenieExchange(UID, 700093000, 24);
	end
end

if (EVENT == 505) then -- Genie Voucher ( 3 Days ) 
	EXPPREM = HowmuchItem(UID, 700094000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45246, NPC, 18, 5000);
	else
    GenieExchange(UID, 700094000, 72);
	end
end

if (EVENT == 507) then -- Genie Voucher ( 7 Days ) 
	EXPPREM = HowmuchItem(UID, 700095000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45246, NPC, 18, 5000);
	else
    GenieExchange(UID, 700095000, 168);
	end
end

if (EVENT == 509) then -- Genie Voucher ( 15 Days ) 
	EXPPREM = HowmuchItem(UID, 700091000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45246, NPC, 18, 5000);
	else
    GenieExchange(UID, 700091000, 360);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 550) then -- Voucher of Automatic Loot ( 15 Days ) 
	LOOTING = HowmuchItem(UID, 750680000);
	if (LOOTING < 1 or LOOTING == 0) then
		SelectMsg(UID, 2, -1, 45247, NPC, 18, 5000);
	else
    EVENT = 551
	end
end

if (EVENT == 551) then
	LOOTING = HowmuchItem(UID, 750680000);
	if (LOOTING < 1 or LOOTING == 0) then
		SelectMsg(UID, 2, -1, 45247, NPC, 18, 5000);
	else
	RobItem(UID, 750680000, 1);
	GiveItem(UID, 850680000, 1, 15);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 570) then -- Voucher of Genie (15 Days) + Auto Loot (15 Days)
	EXPPREM = HowmuchItem(UID, 700191000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45248, NPC, 18, 5000);
	else
    EVENT = 571
	end
end

if (EVENT == 571) then
	EXPPREM = HowmuchItem(UID, 700191000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45248, NPC, 18, 5000);
	else
	RobItem(UID, 700191000, 1);
	GiveItem(UID, 700091000, 1);
	GiveItem(UID, 750680000, 1);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 600) then
	SelectMsg(UID, 3, -1, 4901, NPC, 4285, 601, 4286, 602, 4287, 1150, 4420, 611, 4421, 616, 4589, 621, 4588, 626, 4504, 631, 4296, 100);
end

if (EVENT == 601) then
	SelectMsg(UID, 11, savenum, 4432, NPC);
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 602) then -- Valkyrie Armor
	ITEMARMOR = HowmuchItem(UID, 800180000);
	if (ITEMARMOR > 0) then
		SelectMsg(UID, 3, -1, 4902, NPC, 4288, 603, 4289, 604, 4290, 605, 4291, 606);
	else
		SelectMsg(UID, 2, -1, 4921, NPC, 18, 5000);
	end
end

if (EVENT == 603) then
	Check = isRoomForItem(UID, 508011441);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800180000, 1)
		GiveItem(UID, 508011441, 1, 30)
	end
end

if (EVENT == 604) then
	Check = isRoomForItem(UID, 508011442);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800180000, 1)
		GiveItem(UID, 508011442, 1, 30)
	end
end

if (EVENT == 605) then
	Check = isRoomForItem(UID, 508011443);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800180000, 1)
		GiveItem(UID, 508011443, 1, 30)
	end
end

if (EVENT == 606) then
	Check = isRoomForItem(UID, 508011444);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800180000, 1)
		GiveItem(UID, 508011444, 1, 30)
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 1150) then -- Valkyrie Helmet
	ITEMHELMET = HowmuchItem(UID, 800170000);
	if (ITEMHELMET > 0) then
		SelectMsg(UID, 3, -1, 4902, NPC, 4292, 607, 4293, 608, 4294, 609, 4295, 610);
	else
		SelectMsg(UID, 2, -1, 4911, NPC, 18, 5000);
	end
end

if (EVENT == 607) then
	Check = isRoomForItem(UID, 508013318);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800170000, 1)
		GiveItem(UID, 508013318, 1, 30)
	end
end

if (EVENT == 608) then
	Check = isRoomForItem(UID, 508013319);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800170000, 1)
		GiveItem(UID, 508013319, 1, 30)
	end
end

if (EVENT == 609) then
	Check = isRoomForItem(UID, 508013320);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800170000, 1)
		GiveItem(UID, 508013320, 1, 30)
	end
end

if (EVENT == 610) then
	Check = isRoomForItem(UID, 508013321);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800170000, 1)
		GiveItem(UID, 508013321, 1, 30)
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 611) then -- Gryphon Armor
	ITEMGRYPA = HowmuchItem(UID, 800240000);
	if (ITEMGRYPA > 0) then
		SelectMsg(UID, 3, -1, 4902, NPC, 4288, 612, 4289, 613, 4290, 614, 4291, 615);
	else
		SelectMsg(UID, 2, -1, 6488, NPC, 18, 5000);
	end
end

if (EVENT == 612) then
	Check = isRoomForItem(UID, 508471453);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800240000, 1)
		GiveItem(UID, 508471453, 1, 30)
	end
end

if (EVENT == 613) then
	Check = isRoomForItem(UID, 508471454);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800240000, 1)
		GiveItem(UID, 508471454, 1, 30)
	end
end

if (EVENT == 614) then
	Check = isRoomForItem(UID, 508471455);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800240000, 1)
		GiveItem(UID, 508471455, 1, 30)
	end
end

if (EVENT == 615) then
	Check = isRoomForItem(UID, 508471456);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800240000, 1)
		GiveItem(UID, 508471456, 1, 30)
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 616) then -- Gryphon Helmet
	ITEMGRYPH = HowmuchItem(UID, 800230000);
	if (ITEMGRYPH > 0) then
		SelectMsg(UID, 3, -1, 4902, NPC, 4288, 617, 4289, 618, 4290, 619, 4291, 620);
	else
		SelectMsg(UID, 2, -1, 6497, NPC, 18, 5000);
	end
end

if (EVENT == 617) then
	Check = isRoomForItem(UID, 508473453);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800230000, 1)
		GiveItem(UID, 508473453, 1, 30)
	end
end

if (EVENT == 618) then
	Check = isRoomForItem(UID, 508473454);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800230000, 1)
		GiveItem(UID, 508473454, 1, 30)
	end
end

if (EVENT == 619) then
	Check = isRoomForItem(UID, 508473455);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800230000, 1)
		GiveItem(UID, 508473455, 1, 30)
	end
end

if (EVENT == 620) then
	Check = isRoomForItem(UID, 508473456);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800230000, 1)
		GiveItem(UID, 508473456, 1, 30)
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 621) then -- Bahamut Armor
	ITEMBHMTA = HowmuchItem(UID, 800270000);
	if (ITEMBHMTA > 0) then
		SelectMsg(UID, 3, -1, 4902, NPC, 4288, 622, 4289, 623, 4290, 624, 4291, 625);
	else
		SelectMsg(UID, 2, -1, 1126, NPC, 18, 5000);
	end
end

if (EVENT == 622) then
	Check = isRoomForItem(UID, 508051466);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800270000, 1)
		GiveItem(UID, 508051466, 1, 30)
	end
end

if (EVENT == 623) then
	Check = isRoomForItem(UID, 508051467);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800270000, 1)
		GiveItem(UID, 508051467, 1, 30)
	end
end

if (EVENT == 624) then
	Check = isRoomForItem(UID, 508051468);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800270000, 1)
		GiveItem(UID, 508051468, 1, 30)
	end
end

if (EVENT == 625) then
	Check = isRoomForItem(UID, 508051469);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800270000, 1)
		GiveItem(UID, 508051469, 1, 30)
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 626) then -- Bahamut Helmet
	ITEMBHMTH = HowmuchItem(UID, 800260000);
	if (ITEMBHMTH > 0) then
		SelectMsg(UID, 3, -1, 4902, NPC, 4288, 627, 4289, 628, 4290, 629, 4291, 630);
	else
		SelectMsg(UID, 2, -1, 1126, NPC, 18, 1000);
	end
end

if (EVENT == 627) then
	Check = isRoomForItem(UID, 508053466);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800260000, 1)
		GiveItem(UID, 508053466, 1, 30)
	end
end

if (EVENT == 628) then
	Check = isRoomForItem(UID, 508053467);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800260000, 1)
		GiveItem(UID, 508053467, 1, 30)
	end
end

if (EVENT == 629) then
	Check = isRoomForItem(UID, 508053468);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800260000, 1)
		GiveItem(UID, 508053468, 1, 30)
	end
end

if (EVENT == 630) then
	Check = isRoomForItem(UID, 508053469);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800260000, 1)
		GiveItem(UID, 508053469, 1, 30)
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 631) then -- Pathos Attack
	ITEMPTHS = HowmuchItem(UID, 800250000);
	if (ITEMPTHS > 0) then
		SelectMsg(UID, 3, -1, 748, NPC, 4509, 632, 4510, 633);
	else
		SelectMsg(UID, 2, -1, 749, NPC, 18, 1000);
	end
end

if (EVENT == 632) then
	SelectMsg(UID, 3, -1, 750, NPC, 4505, 634, 4506, 635, 4507, 636, 4508, 637);
end

if (EVENT == 634) then
	Check = isRoomForItem(UID, 800250000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800250000, 1)
		GiveItem(UID, 502573462, 1, 30)
		SelectMsg(UID, 2, -1, 752, NPC, 27, 168);
	end
end

if (EVENT == 635) then
	Check = isRoomForItem(UID, 800250000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800250000, 1)
		GiveItem(UID, 503573463, 1, 30)
		SelectMsg(UID, 2, -1, 752, NPC, 27, 168);
	end
end

if (EVENT == 636) then
	Check = isRoomForItem(UID, 800250000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800250000, 1)
		GiveItem(UID , 504573464, 1, 30)
		SelectMsg(UID, 2, -1, 752, NPC, 27, 168);
	end
end

if (EVENT == 637) then
	Check = isRoomForItem(UID, 800250000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800250000, 1)
		GiveItem(UID, 505573465, 1, 30)
		SelectMsg(UID, 2, -1, 752, NPC, 27, 168);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 633) then -- Pathos Defans
	SelectMsg(UID, 3, -1, 751, NPC, 4514, 638, 4515, 639, 4516, 640, 4517, 641);
end

if (EVENT == 638) then
	Check = isRoomForItem(UID, 800250000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800250000, 1)
		GiveItem(UID, 511573471, 1, 30)
		SelectMsg(UID, 2, -1, 752, NPC, 27, 168);
	end
end

if (EVENT == 639) then
	Check = isRoomForItem(UID, 800250000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800250000, 1)
		GiveItem(UID, 512573472, 1, 30)
		SelectMsg(UID, 2, -1, 752, NPC, 27, 168);
	end
end

if (EVENT == 640) then
	Check = isRoomForItem(UID, 800250000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800250000, 1)
		GiveItem(UID , 513573473, 1, 30)
		SelectMsg(UID, 2, -1, 752, NPC, 27, 168);
	end
end

if (EVENT == 641) then
	Check = isRoomForItem(UID, 800250000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, 168);
	else
		RobItem(UID, 800250000, 1)
		GiveItem(UID, 514573474, 1, 30)
		SelectMsg(UID, 2, -1, 752, NPC, 27, 168);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 660) then -- Starter Pack
	VIPPackage = HowmuchItem(UID, 810035000);
	if (VIPPackage < 1 or VIPPackage == 0) then
		SelectMsg(UID, 2, -1, 45249, NPC, 18, 5000);
	else
    EVENT = 661
	end
end

if (EVENT == 661) then
	VIPPackage = HowmuchItem(UID, 810035000);
	if (VIPPackage < 1 or VIPPackage == 0) then
		SelectMsg(UID, 2, -1, 45249, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 18)
     if SlotCheck == false then
    else   
   	RobItem(UID, 810035000, 1);
	GiveItem(UID, 399295859, 1); -- Switching Premium
	GiveItem(UID, 700191000, 1); -- Genie + Auto Loot
	GiveItem(UID, 353500000, 1); -- Bronze Star
	GiveItem(UID, 800440000, 1); -- Magic Bag
	GiveItem(UID, 800440000, 1); -- Magic Bag
	GiveItem(UID, 700111000, 1); -- Offline Merchant
	GiveItem(UID, 389390000, 1); -- Premium Potion HP
	GiveItem(UID, 389400000, 1); -- Premium Potion MP
	GiveItem(UID, 800050000, 5); -- Mount Scroll - %50 EXP
	GiveItem(UID, 800008000, 1); -- Lion Scroll (L)
	GiveItem(UID, 800015000, 1); -- Speed-Up Potion
	GiveItem(UID, 800061000, 1); -- Weapon Enchant Scroll
	GiveItem(UID, 800062000, 1); -- Armor Enchant Scroll
	GiveItem(UID, 700001000, 1); -- Redistribution Item
	GiveItem(UID, 700089000, 1); -- 1200 Knight Cash
	GiveItem(UID, 800074000, 1); -- NP increase item
	GiveItem(UID, 800442000, 1); -- VIP Key
	GiveItem(UID, 810227000, 1); -- Genie Hammer
	end
end
end
----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 680) then -- Voucher of TL
	SelectMsg(UID, 2, -1, 45238, NPC, 45473, 681, 45474, 683, 45475, 685, 45476, 687, 45477, 689, 45478, 691);
end

if (EVENT == 681) then -- 1 TL Voucher
	EXPPREM = HowmuchItem(UID, 346600000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45250, NPC, 18, 5000);
	else
    EVENT = 682
	end
end

if (EVENT == 682) then
	EXPPREM = HowmuchItem(UID, 346600000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45250, NPC, 18, 5000);
	else
	RobItem(UID, 346600000, 1);
	GiveBalance(UID, 0, 1);
	end
end

if (EVENT == 683) then -- 5 TL Voucher
	EXPPREM = HowmuchItem(UID, 346700000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45251, NPC, 18, 5000);
	else
    EVENT = 684
	end
end

if (EVENT == 684) then
	EXPPREM = HowmuchItem(UID, 346700000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45251, NPC, 18, 5000);
	else
	RobItem(UID, 346700000, 1);
	GiveBalance(UID, 0, 5);
	end
end

if (EVENT == 685) then -- 10 TL Voucher
	EXPPREM = HowmuchItem(UID, 346800000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45252, NPC, 18, 5000);
	else
    EVENT = 686
	end
end

if (EVENT == 686) then
	EXPPREM = HowmuchItem(UID, 346800000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45252, NPC, 18, 5000);
	else
	RobItem(UID, 346800000, 1);
	GiveBalance(UID, 0, 10);
	end
end

if (EVENT == 687) then -- 20 TL Voucher
	EXPPREM = HowmuchItem(UID, 346900000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45253, NPC, 18, 5000);
	else
    EVENT = 688
	end
end

if (EVENT == 688) then
	EXPPREM = HowmuchItem(UID, 346900000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45253, NPC, 18, 5000);
	else
	RobItem(UID, 346900000, 1);
	GiveBalance(UID, 0, 20);
	end
end

if (EVENT == 689) then -- 50 TL Voucher
	EXPPREM = HowmuchItem(UID, 347000000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45254, NPC, 18, 5000);
	else
    EVENT = 690
	end
end

if (EVENT == 690) then
	EXPPREM = HowmuchItem(UID, 347000000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45254, NPC, 18, 5000);
	else
	RobItem(UID, 347000000, 1);
	GiveBalance(UID, 0, 50);
	end
end

if (EVENT == 691) then -- 100 TL Voucher
	EXPPREM = HowmuchItem(UID, 347100000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45255, NPC, 18, 5000);
	else
    EVENT = 692
	end
end

if (EVENT == 692) then
	EXPPREM = HowmuchItem(UID, 347100000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45255, NPC, 18, 5000);
	else
	RobItem(UID, 347100000, 1);
	GiveBalance(UID, 0, 100);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 700) then -- +EXP Scroll
	SelectMsg(UID, 2, -1, 45238, NPC, 45479, 701, 45480, 703, 45481, 705);
end


if (EVENT == 701) then -- +EXP Scroll ( 1M )
	EXPPREM = HowmuchItem(UID, 347500000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45256, NPC, 18, 5000);
	else
    EVENT = 702
	end
end

if (EVENT == 702) then
	EXPPREM = HowmuchItem(UID, 347500000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45256, NPC, 18, 5000);
	else
	RobItem(UID, 347500000, 1);
	ExpChange(UID,1000000);
	end
end

if (EVENT == 703) then -- +EXP Scroll ( 3M )
	EXPPREM = HowmuchItem(UID, 347600000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45257, NPC, 18, 5000);
	else
    EVENT = 704
	end
end

if (EVENT == 704) then
	EXPPREM = HowmuchItem(UID, 347600000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45257, NPC, 18, 5000);
	else
	RobItem(UID, 347600000, 1);
	ExpChange(UID,3000000);
	end
end

if (EVENT == 705) then -- +EXP Scroll ( 5M )
	EXPPREM = HowmuchItem(UID, 347700000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45258, NPC, 18, 5000);
	else
    EVENT = 706
	end
end

if (EVENT == 706) then
	EXPPREM = HowmuchItem(UID, 347700000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45258, NPC, 18, 5000);
	else
	RobItem(UID, 347700000, 1);
	ExpChange(UID,5000000);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 720) then -- +NP Scroll
	SelectMsg(UID, 2, -1, 45238, NPC, 45482, 721, 45483, 723, 45484, 725);
end


if (EVENT == 721) then -- +NP Scroll ( 1M )
	EXPPREM = HowmuchItem(UID, 347200000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45259, NPC, 18, 5000);
	else
    EVENT = 722
	end
end

if (EVENT == 722) then
	EXPPREM = HowmuchItem(UID, 347200000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45259, NPC, 18, 5000);
	else
	RobItem(UID, 347200000, 1);
	GiveLoyalty(UID, 1000);
	end
end

if (EVENT == 723) then -- +NP Scroll ( 3M )
	EXPPREM = HowmuchItem(UID, 347300000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45260, NPC, 18, 5000);
	else
    EVENT = 724
	end
end

if (EVENT == 724) then
	EXPPREM = HowmuchItem(UID, 347300000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45260, NPC, 18, 5000);
	else
	RobItem(UID, 347300000, 1);
	GiveLoyalty(UID, 3000);
	end
end

if (EVENT == 725) then -- +NP Scroll ( 5M )
	EXPPREM = HowmuchItem(UID, 347400000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45261, NPC, 18, 5000);
	else
    EVENT = 726
	end
end

if (EVENT == 726) then
	EXPPREM = HowmuchItem(UID, 347400000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45261, NPC, 18, 5000);
	else
	RobItem(UID, 347400000, 1);
	GiveLoyalty(UID, 5000);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 750) then -- Death Knight Skulls
	SelectMsg(UID, 2, -1, 45238, NPC, 45485, 751);
end

if (EVENT == 751) then -- Death Knight Skulls Emblem
	EXPPREM = HowmuchItem(UID, 346250000);
	if (EXPPREM < 5000 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45262, NPC, 18, 5000);
	else
    EVENT = 752
	end
end

if (EVENT == 752) then
	EXPPREM = HowmuchItem(UID, 346250000);
	if (EXPPREM < 5000 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45262, NPC, 18, 5000);
	else
	RobItem(UID, 346250000, 5000);
	GiveItem(UID, 914055747, 1, 30);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 760) then -- Extra Inventory
	EXPPREM = HowmuchItem(UID, 800440000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45263, NPC, 18, 5000);
	else
    EVENT = 762
	end
end

if (EVENT == 762) then
	EXPPREM = HowmuchItem(UID, 800440000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45263, NPC, 18, 5000);
	else
	RobItem(UID, 800440000, 1);
	GiveItem(UID, 700011001, 1, 30);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 780) then -- Voucher of Offline Merchant
	EXPPREM = HowmuchItem(UID, 700111000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45264, NPC, 18, 5000);
	else
    EVENT = 781
	end
end

if (EVENT == 781) then
	EXPPREM = HowmuchItem(UID, 700111000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45264, NPC, 18, 5000);
	else
	RobItem(UID, 700111000, 1);
	GiveItem(UID, 924041913, 1, 30);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 800) then -- Voucher of Merchant's Eye
	EXPPREM = HowmuchItem(UID, 810163000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45265, NPC, 18, 5000);
	else
    EVENT = 801
	end
end

if (EVENT == 801) then
	EXPPREM = HowmuchItem(UID, 810163000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45265, NPC, 18, 5000);
	else
	RobItem(UID, 810163000, 1);
	GiveItem(UID, 810168000, 1, 30);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 820) then -- Voucher of Infinite Arrow (15 Days)
	EXPPREM = HowmuchItem(UID, 800605000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45266, NPC, 18, 5000);
	else
    EVENT = 821
	end
end

if (EVENT == 821) then
	EXPPREM = HowmuchItem(UID, 800605000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45266, NPC, 18, 5000);
	else
	RobItem(UID, 800605000, 1);
	GiveItem(UID, 800606000, 1, 15);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 840) then -- Voucher of Infinite Cure (15 Days)
	EXPPREM = HowmuchItem(UID, 346390000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45267, NPC, 18, 5000);
	else
    EVENT = 841
	end
end

if (EVENT == 841) then
	EXPPREM = HowmuchItem(UID, 346390000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45267, NPC, 18, 5000);
	else
	RobItem(UID, 346390000, 1);
	GiveItem(UID, 346391000, 1, 15);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 860) then -- Job Change Menü
	SelectMsg(UID, 3, -1, 4131, NPC, 45426, 870, 45440, 880, 4296, 100);
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 870) then  -- Job Change (Master)
JOBCHANGEITEM = HowmuchItem(UID, 700112000);
	if (JOBCHANGEITEM > 0) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
		SelectMsg(UID, 2, -1, 45268, NPC, 45429, 872, 45430, 873, 45431, 874);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SelectMsg(UID, 2, -1, 45268, NPC, 45428, 871, 45430, 873, 45431, 874);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SelectMsg(UID, 2, -1, 45268, NPC, 45428, 871, 45429, 872, 45431, 874);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SelectMsg(UID, 2, -1, 45268, NPC, 45428, 871, 45429, 872, 45430, 873);
	end
    else
        SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
    end
end

if (EVENT == 871) then
JOBCHANGEITEM = HowmuchItem(UID, 700112000);
	if (JOBCHANGEITEM < 1) then
		SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
	else
		check = JobChange(UID,0,1);
		print(check);
	end
end

if (EVENT == 872) then
    JOBCHANGEITEM = HowmuchItem(UID, 700112000);
	if (JOBCHANGEITEM < 1) then
		SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
	else
		JobChange(UID,0,2);
	end
end

if (EVENT == 873) then
    JOBCHANGEITEM = HowmuchItem(UID, 700112000);
	if (JOBCHANGEITEM < 1) then
		SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
	else
		JobChange(UID,0,3);
	end
end

if (EVENT == 874) then
    JOBCHANGEITEM = HowmuchItem(UID, 700112000);
	if (JOBCHANGEITEM < 1) then
		SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
	else
		JobChange(UID,0,4);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 880) then  -- Job Change (NO Master)
JOBCHANGEITEM = HowmuchItem(UID, 700113000);
	if (JOBCHANGEITEM > 0) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
		SelectMsg(UID, 2, -1, 45268, NPC, 45429, 882, 45430, 883, 45431, 884);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SelectMsg(UID, 2, -1, 45268, NPC, 45428, 881, 45430, 883, 45431, 884);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SelectMsg(UID, 2, -1, 45268, NPC, 45428, 881, 45429, 882, 45431, 884);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SelectMsg(UID, 2, -1, 45268, NPC, 45428, 881, 45429, 882, 45430, 883);
	end
    else
        SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
    end
end

if (EVENT == 881) then
JOBCHANGEITEM = HowmuchItem(UID, 700113000);
	if (JOBCHANGEITEM < 1) then
		SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
	else
		JobChange(UID,1,1);
	end
end

if (EVENT == 882) then
    JOBCHANGEITEM = HowmuchItem(UID, 700113000);
	if (JOBCHANGEITEM < 1) then
		SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
	else
		JobChange(UID,1,2);
	end
end

if (EVENT == 883) then
    JOBCHANGEITEM = HowmuchItem(UID, 700113000);
	if (JOBCHANGEITEM < 1) then
		SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
	else
		JobChange(UID,1,3);
	end
end

if (EVENT == 884) then
    JOBCHANGEITEM = HowmuchItem(UID, 700113000);
	if (JOBCHANGEITEM < 1) then
		SelectMsg(UID, 2, -1, 45055, NPC, 18, 5000);
	else
		JobChange(UID,1,4);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 890) then  -- Exchange Stars
	SelectMsg(UID, 3, -1, 45238, NPC, 45488, 891, 45489, 893, 45490, 895);
end

if (EVENT == 891) then -- Voucher of Bronze Star (30 Days)
	EXPPREM = HowmuchItem(UID, 353500000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45269, NPC, 18, 5000);
	else
    EVENT = 892
	end
end

if (EVENT == 892) then
	EXPPREM = HowmuchItem(UID, 353500000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45269, NPC, 18, 5000);
	else
	RobItem(UID, 353500000, 1);
	GiveItem(UID, 930520722, 1, 30);
	end
end

if (EVENT == 893) then -- Voucher of Silver Star (30 Days)
	EXPPREM = HowmuchItem(UID, 353400000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45270, NPC, 18, 5000);
	else
    EVENT = 894
	end
end

if (EVENT == 894) then
	EXPPREM = HowmuchItem(UID, 353400000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45270, NPC, 18, 5000);
	else
	RobItem(UID, 353400000, 1);
	GiveItem(UID, 930530723, 1, 30);
	end
end

if (EVENT == 895) then -- Voucher of Gold Star (30 Days)
	EXPPREM = HowmuchItem(UID, 353300000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45271, NPC, 18, 5000);
	else
    EVENT = 896
	end
end

if (EVENT == 896) then
	EXPPREM = HowmuchItem(UID, 353300000);
	if (EXPPREM < 1 or EXPPREM == 0) then
		SelectMsg(UID, 2, -1, 45271, NPC, 18, 5000);
	else
	RobItem(UID, 353300000, 1);
	GiveItem(UID, 930540724, 1, 30);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if (EVENT == 1200) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST > 0) then
		SelectMsg(UID, 2, -1, 4456, NPC, 4189, 1201, 4190, 1202);
	else
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	end
end

if (EVENT == 1201) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 4456, NPC, 3000, 1203, 3005, -1);
	end
end

if (EVENT == 1202) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 4456, NPC, 3000, 1204, 3005, -1);
	end
end


if (EVENT == 1203) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		ResetSkillPoints(UID);
	end
end

if (EVENT == 1204) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		ResetStatPoints(UID);
	end
end

----------------------------------------------------------------------------------------------------------------------------------------------------------

if EVENT == 1300 then
    SelectMsg(UID, 24, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1);

end
if EVENT == 1310 then
    SelectMsg(UID, 53, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1);

end

----------------------------------------------------------------------------------------------------------------------------------------------------------
