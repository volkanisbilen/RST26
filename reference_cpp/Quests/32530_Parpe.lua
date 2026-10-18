-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32530;
------------------
if (EVENT == 100) then --50509,151(hpmpmmaestro)
	SelectMsg(UID, 3, -1, 91004, NPC,49009,110,49010,120,49011,121,49028,147,49015,125,49024,140);
end

if (EVENT == 110) then -- Gift Box
	GiftBox = HowmuchItem(UID, 900063000);
	if (GiftBox < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 6)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900063000,1);
		GiveItem(UID, 508013319,1,2);
		GiveItem(UID, 508011442,1,2);
		GiveItem(UID, 511573471,1,2);
		GiveItem(UID, 512573472,1,2);
		GiveItem(UID, 810932954,1,2);
		GiveItem(UID, 810638730,1,2);
    	end
    end
end


if (EVENT == 120) then -- BUFF BOX
	BUFFBOX = HowmuchItem(UID, 900668000);
	if (BUFFBOX < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900668000,1);
		GiveItem(UID, 800078000,1,1);
		GiveItem(UID, 800077000,1,1);
    	end
    end
end
----------------OREADS VOUCHER
if (EVENT == 121) then
SelectMsg(UID, 3, -1, 91004, NPC, 49012,122,49013,123,49014,124);
end

if (EVENT == 122) then -- OREADS VOUCHER 1 DAY
	OREADS = HowmuchItem(UID, 814038000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 814038000,1);
		GiveItem(UID, 700039768,1,1);
    	end
    end
end

if (EVENT == 123) then -- OREADS VOUCHER 3 DAY
	OREADS = HowmuchItem(UID, 814048000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 814048000,1);
		GiveItem(UID, 700039768,1,3);
    	end
    end
end

if (EVENT == 124) then -- OREADS VOUCHER 7 DAY
	OREADS = HowmuchItem(UID, 814058000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 814058000,1);
		GiveItem(UID, 700039768,1,7);
    	end
    end
end
-----------------------------------------------

if (EVENT == 125) then
SelectMsg(UID, 3, -1, 91004, NPC, 49016,126,49017,133,50500,199);
end

if (EVENT == 126) then
SelectMsg(UID, 2, -1, 12448, NPC,49018,127,49019,129,49020,131);
end

if (EVENT == 127) then -- WAR Premium 1 Day
	WARPREM = HowmuchItem(UID, 914012673);
	if (WARPREM < 1 or WARPREM == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 128
	end
end

if (EVENT == 128) then
	WARPREM = HowmuchItem(UID, 914012673);
	if (WARPREM < 1 or WARPREM == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 914012673, 1);
	GivePremium(UID, 12, 1);
end
end

-------War Pre 3 DAY

if (EVENT == 129) then -- WAR Premium 3 Day
	WARPREM = HowmuchItem(UID, 914022851);
	if (WARPREM < 1 or WARPREM == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 130
	end
end

if (EVENT == 130) then
	WARPREM = HowmuchItem(UID, 914022851);
	if (WARPREM < 1 or WARPREM == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 914022851, 1);
	GivePremium(UID, 12, 3);
end
end

--------War Pre 7 DAY

if (EVENT == 131) then -- WAR Premium 7 Day
	WARPREM = HowmuchItem(UID, 914032852);
	if (WARPREM < 1 or WARPREM == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 132
	end
end

if (EVENT == 132) then
	WARPREM = HowmuchItem(UID, 914032852);
	if (WARPREM < 1 or WARPREM == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 914032852, 1);
	GivePremium(UID, 12, 7);
end
end

--------DC PRE 1 DAY

if (EVENT == 133) then
SelectMsg(UID, 2, -1, 12448, NPC,49021,134,49022,136,49023,138);
end

if (EVENT == 134) then -- DC Premium 1 Day
	DcPre = HowmuchItem(UID, 399381883);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 135
	end
end

if (EVENT == 135) then
	DcPre = HowmuchItem(UID, 399381883);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 399381883, 1);
	GivePremium(UID, 10, 1);
end
end

-------DC Pre 3 DAY

if (EVENT == 136) then -- DC Premium 3 Day
	DcPre = HowmuchItem(UID, 399481745);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 137
	end
end

if (EVENT == 137) then
	DcPre = HowmuchItem(UID, 399481745);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 399481745, 1);
	GivePremium(UID, 10, 3);
end
end

--------Dc Pre 7 DAY

if (EVENT == 138) then -- DC Premium 7 Day
	DcPre = HowmuchItem(UID, 399581746);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 139
	end
end

if (EVENT == 139) then
	DcPre = HowmuchItem(UID, 399581746);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 399581746, 1);
	GivePremium(UID, 10, 7);
end
end

--------------------------------------
if (EVENT == 199) then
SelectMsg(UID, 2, -1, 12448, NPC,50501,200,50502,202,50503,204);
end

if (EVENT == 200) then -- Switching Premium 1 DAY
	DcPre = HowmuchItem(UID, 399294738);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 201
	end
end

if (EVENT == 201) then
	DcPre = HowmuchItem(UID, 399294738);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 399294738, 1);
	GivePremium(UID, 10, 1);
	GivePremium(UID, 11, 1);
	GivePremium(UID, 12, 1);
end
end

-------Switching Premium  3 DAY

if (EVENT == 202) then -- Switching Premium  3 Day
	DcPre = HowmuchItem(UID, 399299739);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 203
	end
end

if (EVENT == 203) then
	DcPre = HowmuchItem(UID, 399299739);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 399299739, 1);
	GivePremium(UID, 10, 3);
	GivePremium(UID, 11, 3);
	GivePremium(UID, 12, 3);
end
end

-------Switching Premium  7 DAY

if (EVENT == 204) then -- Switching Premium 7 Day
	DcPre = HowmuchItem(UID, 399297740);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 205
	end
end

if (EVENT == 205) then
	DcPre = HowmuchItem(UID, 399297740);
	if (DcPre < 1 or DcPre == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
	RobItem(UID, 399297740, 1);
	GivePremium(UID, 10, 7);
	GivePremium(UID, 11, 7);
	GivePremium(UID, 12, 7);
end
end

--------------------------------------

if (EVENT == 140) then
SelectMsg(UID, 3, -1, 91004, NPC, 49025,141,49026,143,49027,145);
end

------Golden Mattock 1 DAY
if (EVENT == 141) then -- Golden Mattock 1 DAY
	Golden = HowmuchItem(UID, 504122000);
	if (Golden < 1 or Golden == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 142
	end
end

if (EVENT == 142) then
	OREADS = HowmuchItem(UID, 504122000);
	if (OREADS < 1 or OREADS == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RobItem(UID, 504122000, 1);
	GiveItem(UID, 389135000, 1,1);
end
end
end

------------Golden mattock 3 DAY
if (EVENT == 143) then -- Golden Mattock 3 DAY
	Golden = HowmuchItem(UID, 505122000);
	if (Golden < 1 or Golden == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 144
	end
end

if (EVENT == 144) then
	OREADS = HowmuchItem(UID, 505122000);
	if (OREADS < 1 or OREADS == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RobItem(UID, 505122000, 1);
	GiveItem(UID, 389135000, 1,3);
end
end
end

---------Golden Mattock 7 Day
if (EVENT == 145) then -- Golden Mattock 7 DAY
	Golden = HowmuchItem(UID, 506122000);
	if (Golden < 1 or Golden == 0) then
		SelectMsg(UID, 2, -1, 91002, NPC, 18, 5000);
	else
    EVENT = 146
	end
end

if (EVENT == 146) then
	OREADS = HowmuchItem(UID, 506122000);
	if (OREADS < 1 or OREADS == 0) then
		SelectMsg(UID, 2, -1, 91004, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RobItem(UID, 506122000, 1);
	GiveItem(UID, 389135000, 1,7);
end
end
end

-------------
----------------Alseids VOUCHER
if (EVENT == 147) then
SelectMsg(UID, 3, -1, 91004, NPC, 49029,148,49030,149,49031,150);
end

if (EVENT == 148) then -- Alseid VOUCHER 1 DAY
	Alseids = HowmuchItem(UID, 901387000);
	if (Alseids < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 901387000,1);
		GiveItem(UID, 700042769,1,1);
    	end
    end
end

if (EVENT == 149) then -- Alseids VOUCHER 3 DAY
	Alseids = HowmuchItem(UID, 902387000);
	if (Alseids < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 902387000,1);
		GiveItem(UID, 700042769,1,3);
    	end
    end
end

if (EVENT == 150) then -- Alseids VOUCHER 7 DAY
	Alseids = HowmuchItem(UID, 903387000);
	if (Alseids < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 903387000,1);
		GiveItem(UID, 700042769,1,7);
    	end
    end
end
----------------HP MP MAESTRO
if (EVENT == 151) then
SelectMsg(UID, 3, -1, 91004, NPC, 50510,152,50511,153);
end

if (EVENT == 152) then -- HP MAESTRO 30 DAY
	OREADS = HowmuchItem(UID, 810115000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810115000,1);
		GiveItem(UID, 810117000,1,30);
    	end
    end
end

if (EVENT == 153) then -- MP MAESTRO 30 DAY
	OREADS = HowmuchItem(UID, 810116000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810116000,1);
		GiveItem(UID, 810118000,1,30);
    	end
    end
end
-----------------------------------------------
