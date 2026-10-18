local NPC = 25177;

if (EVENT == 100) then  -- ,50546,2000 Silindi. 50520,944,
	SelectMsg(UID, 3, -1, 11804, NPC,45234,938,45235,940,45236,942,8354,946);  --,50577,7000
end

------------------------------------------------------------------------------------------------------------------------------
-- EXPERIENCE SEALED JAR EXCHANGE

if (EVENT == 946) then
	SelectMsg(UID, 3, -1, 11804, NPC,60001,947,60002,948);
end

if (EVENT == 947) then -- Experience Sealed Jar [50,000,000]
	if (RunQuestExchange(UID, 990001) == false) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	end
end

if (EVENT == 948) then -- Experience Sealed Jar [100,000,000]
	if (RunQuestExchange(UID, 990002) == false) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	end
end

------------------------------------------------------------------------------------------------------------------------------
-- VIP HAZIR PAKET

if (EVENT == 938) then -- Vip Hazır Paket Menüsü
	VIPPackage = HowmuchItem(UID, 810039000);
	if (VIPPackage < 1 or VIPPackage == 0) then
		SelectMsg(UID, 2, -1, 11804, NPC, 18, 5000);
	else
    EVENT = 939
	end
end

if (EVENT == 939) then
	VIPPackage = HowmuchItem(UID, 810039000);
	if (VIPPackage < 1 or VIPPackage == 0) then
		SelectMsg(UID, 2, -1, 11804, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 13)
     if SlotCheck == false then
       
         else
	RobItem(UID, 810039000, 1);
	GiveItem(UID, 399295859, 1, 90); -- Switching Premium
	GiveItem(UID, 508112000, 1, 90); --	Minerva Package
	GiveItem(UID, 508074000, 1, 90); --	Pathos Gloves Package
	GiveItem(UID, 800387000, 1, 90); -- Alseids Peri
	GiveItem(UID, 511000000, 1, 90); -- Automatic Mining + Robin Loot
	GiveItem(UID, 810926000, 1, 90); -- Solar Tatto
	GiveItem(UID, 700111000, 1, 90); -- Offline Merchant
	GiveItem(UID, 800442000, 1, 90); -- Vip Key
	GiveItem(UID, 800440000, 1, 90); --	Magic Bag
	GiveItem(UID, 800440000, 1, 90); --	Magic Bag
	GiveItem(UID, 800036000, 1, 90); -- Resaration Scroll %60
	GiveItem(UID, 800079000, 1, 90); -- HP Scroll %60
	GiveItem(UID, 800077000, 1, 90); -- Deff SC 400
	GiveItem(UID, 810022000, 1, 90); -- Duration Item
	GiveItem(UID, 800014000, 1, 90); --	Scroll Of Attack
	GiveItem(UID, 820075000, 1, 90); -- Symbol of Gladyator
	GiveItem(UID, 800074000, 1, 90); -- NP SC
	GiveItem(UID, 810378000, 1, 90); -- Genie
	GiveItem(UID, 800015000, 1, 90); -- SW SC
	
end
end
end

------------------------------------------------------------------------------------------------------------------------------
-- FARM HAZIR PAKET

if (EVENT == 940) then -- Farm Hazır Paket Menüsü
	FARMPackage = HowmuchItem(UID, 810037000);
	if (FARMPackage < 1 or FARMPackage == 0) then
		SelectMsg(UID, 2, -1, 11793, NPC, 18, 5000);
	else
    EVENT = 941
	end
end

if (EVENT == 941) then 
	FARMPackage = HowmuchItem(UID, 810037000);
	if (FARMPackage < 1 or FARMPackage == 0) then
		SelectMsg(UID, 2, -1, 11793, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 5)
     if SlotCheck == false then
       
         else
	RobItem(UID, 810037000, 1);
	GiveItem(UID, 399281685, 1, 90); -- DC Premium
	GiveItem(UID, 508112000, 1, 90); --	Minerva Package
	GiveItem(UID, 810378000, 1, 90); -- Genie
	GiveItem(UID, 814039000, 1, 90); -- Oreads Peri
	GiveItem(UID, 800440000, 1, 90); --	Magic Bag
	GiveItem(UID, 800440000, 1, 90); --	Magic Bag
	GiveItem(UID, 511000000, 1, 90); -- Automatic Mining + Robin Loot
	GiveItem(UID, 820075000, 1, 90); -- Symbol of Gladyator
	GiveItem(UID, 700111000, 1, 90); --	Offline Merchant
	GiveItem(UID, 810227000, 1, 90); --	Genie Hammer   
end
end
end

------------------------------------------------------------------------------------------------------------------------------
-- PK HAZIR PAKET

if (EVENT == 942) then -- PK Hazır Paket Menüsü
	PKPackage = HowmuchItem(UID, 810038000);
	if (PKPackage < 1 or PKPackage == 0) then
		SelectMsg(UID, 2, -1, 11795, NPC, 18, 5000);
	else
    EVENT = 943
	end
end

if (EVENT == 943) then
	PKPackage = HowmuchItem(UID, 810038000);
	if (PKPackage < 1 or PKPackage == 0) then
		SelectMsg(UID, 2, -1, 11795, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 6)
     if SlotCheck == false then
       
         else
	RobItem(UID, 810038000, 1);
	GiveItem(UID, 399292764, 1, 90); --	War Premium
	GiveItem(UID, 508112000, 1, 90); --	Minerva Package
	GiveItem(UID, 508074000, 1, 90); --	Pathos Gloves Package 
	GiveItem(UID, 810926000, 1, 90); -- Solar Tatto  
	GiveItem(UID, 800079000, 1, 90); -- HP Scroll %60
	GiveItem(UID, 800077000, 1, 90); -- Deff SC 400
	GiveItem(UID, 810022000, 1, 90); -- Duration Item
	GiveItem(UID, 800014000, 1, 90); --	Scroll Of Attack
	GiveItem(UID, 800074000, 1, 90); --	NP Increase Scroll 
	GiveItem(UID, 800442000, 1, 90); -- Vip Key
	GiveItem(UID, 800015000, 1, 90); -- SW SC
end
end
end

-------------------------------------------------------------------------------------
if (EVENT == 944) then
	SelectMsg(UID, 3, -1, 11804, NPC,50521,3512,50522,3513,50523,3514,50524,3515,50525,3516,50526,3517);
end


if (EVENT == 3512) then -- Valkry Helmet to Bahamut Helmet
	OREADS = HowmuchItem(UID, 800170000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 800170000,1);
		GiveItem(UID, 800260000,1);
    	end
    end
end

if (EVENT == 3513) then -- Valkry Armor to Bahamut Armor
	OREADS = HowmuchItem(UID, 800180000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 800180000,1);
		GiveItem(UID, 800270000,1);
    	end
    end
end

if (EVENT == 3514) then -- Valkry helmet to gryphon helmet.
	OREADS = HowmuchItem(UID, 800170000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 800170000,1);
		GiveItem(UID, 800230000,1);
    	end
    end
end

if (EVENT == 3515) then -- Valkry Armor to gryphon Armor.
	OREADS = HowmuchItem(UID, 800180000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 800180000,1);
		GiveItem(UID, 800240000,1);
    	end
    end
end

if (EVENT == 3516) then -- valkry Helmet to Yeniceri Helmet
	OREADS = HowmuchItem(UID, 800170000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 800170000,1);
		GiveItem(UID, 508116000,1);
    	end
    end
end

if (EVENT == 3517) then -- valkry Armor to Yeniceri Armor
	OREADS = HowmuchItem(UID, 800180000);
	if (OREADS < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 800180000,1);
		GiveItem(UID, 508117000,1);
    	end
    end
end

----------------------------------------------------------------------------------------
if (EVENT == 5000) then
	ShowMap(UID, 450);
end
----------------------------------------------------------------------------------------


if (EVENT == 2000) then
	SelectMsg(UID, 3, -1, 11804, NPC,50547,2001,50548,2002,50549,2003,50550,2004,50551,2005);
end
----------------------------------------------------------------------------------------
if (EVENT == 2001) then --LUNAR TATTOO
	SelectMsg(UID, 3, -1, 11804, NPC,50552,3001,50553,3002,50554,3003,50555,3004,50556,3005);
end

if (EVENT == 3001) then -- Lunar Tattoo 1 Günlük
	TATTO = HowmuchItem(UID, 931706000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931706000,1);
		GiveItem(UID, 810931953,1,1);
    	end
    end
end

if (EVENT == 3002) then -- Lunar Tattoo 3 Günlük
	TATTO = HowmuchItem(UID, 910925000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 910925000,1);
		GiveItem(UID, 810931953,1,3);
    	end
    end
end
if (EVENT == 3003) then -- Lunar Tattoo 7 Günlük
	TATTO = HowmuchItem(UID, 910926000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 910926000,1);
		GiveItem(UID, 810931953,1,7);
    	end
    end
end

if (EVENT == 3004) then -- Lunar Tattoo 15 Günlük
	TATTO = HowmuchItem(UID, 810713000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810713000,1);
		GiveItem(UID, 810931953,1,15);
    	end
    end
end

if (EVENT == 3005) then -- Lunar Tattoo 30 Günlük
	TATTO = HowmuchItem(UID, 810927000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810927000,1);
		GiveItem(UID, 810931953,1,30);
    	end
    end
end
----------------------------------------------------------------------------------------

if (EVENT == 2002) then  --Solar tattoo
	SelectMsg(UID, 3, -1, 11804, NPC,50557,4001,50558,4002,50559,4003,50560,4004,50561,4005);
end

if (EVENT == 4001) then -- Solar Tattoo 1 Günlük
	TATTO = HowmuchItem(UID, 810921000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810921000,1);
		GiveItem(UID, 810431970,1,1);
    	end
    end
end

if (EVENT == 4002) then -- Solar Tattoo 3 Günlük
	TATTO = HowmuchItem(UID, 810922000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810922000,1);
		GiveItem(UID, 810431970,1,3);
    	end
    end
end
if (EVENT == 4003) then -- Solar Tattoo 7 Günlük
	TATTO = HowmuchItem(UID, 810923000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810923000,1);
		GiveItem(UID, 810431970,1,7);
    	end
    end
end

if (EVENT == 4004) then -- Solar Tattoo 15 Günlük
	TATTO = HowmuchItem(UID, 810924000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810924000,1);
		GiveItem(UID, 810431970,1,15);
    	end
    end
end

if (EVENT == 4005) then -- Solar Tattoo 30 Günlük
	TATTO = HowmuchItem(UID, 810926000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810926000,1);
		GiveItem(UID, 810431970,1,30);
    	end
    end
end

----------------------------------------------------------------------------------------

if (EVENT == 2003) then  -- Nimbus Tattoo		
	SelectMsg(UID, 3, -1, 11804, NPC,50562,5001,50563,5002,50564,5003,50565,5004,50566,5005);
end

if (EVENT == 5001) then -- Nimbus Tattoo 1 Günlük
	TATTO = HowmuchItem(UID, 931705000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931705000,1);
		GiveItem(UID, 8104340970,1,1);
    	end
    end
end

if (EVENT == 5002) then -- Nimbus Tattoo 3 Günlük
	TATTO = HowmuchItem(UID, 931755000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(931755000);
		GiveItem(UID, 8104340970,1,3);
    	end
    end
end
if (EVENT == 5003) then -- Nimbus Tattoo 7 Günlük
	TATTO = HowmuchItem(UID, 931765000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931765000,1);
		GiveItem(UID, 8104340970,1,7);
    	end
    end
end

if (EVENT == 5004) then -- Nimbus Tattoo 15 Günlük
	TATTO = HowmuchItem(UID, 931775000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931775000,1);
		GiveItem(UID, 8104340970,1,15);
    	end
    end
end

if (EVENT == 5005) then -- Nimbus Tattoo 30 Günlük
	TATTO = HowmuchItem(UID, 810929000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810929000,1);
		GiveItem(UID, 8104340970,1,30);
    	end
    end
end

----------------------------------------------------------------------------------------
if (EVENT == 2004) then  -- Stellar Tattoo
	SelectMsg(UID, 3, -1, 11804, NPC,50567,6001,50568,6002,50569,6003,50570,6004,50571,6005);
end

if (EVENT == 6001) then -- Stellar Tattoo 1 Günlük
	TATTO = HowmuchItem(UID, 914009000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 914009000,1);
		GiveItem(UID, 810433972,1,1);
    	end
    end
end

if (EVENT == 6002) then -- Stellar Tattoo 3 Günlük
	TATTO = HowmuchItem(UID, 931766000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(931766000);
		GiveItem(UID, 810433972,1,3);
    	end
    end
end
if (EVENT == 6003) then -- Stellar Tattoo 7 Günlük
	TATTO = HowmuchItem(UID, 931666000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931666000,1);
		GiveItem(UID, 810433972,1,7);
    	end
    end
end

if (EVENT == 6004) then -- Stellar Tattoo 15 Günlük
	TATTO = HowmuchItem(UID, 931657000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931657000,1);
		GiveItem(UID, 810433972,1,15);
    	end
    end
end

if (EVENT == 6005) then -- Stellar Tattoo 30 Günlük
	TATTO = HowmuchItem(UID, 810928000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810928000,1);
		GiveItem(UID, 810433972,1,30);
    	end
    end
end

----------------------------------------------------------------------------------------

if (EVENT == 2005) then  --War Tattoo
	SelectMsg(UID, 3, -1, 11804, NPC,50572,7011,50573,7012,50574,7003,50575,7004,50576,7005);
end

if (EVENT == 7011) then -- War Tattoo 1 Günlük
	TATTO = HowmuchItem(UID, 814613000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

	    else
		RobItem(UID, 814613000,1);
		GiveItem(UID, 814664796,1,1);
    	end
    end
end

if (EVENT == 7012) then -- War Tattoo 3 Günlük
	TATTO = HowmuchItem(UID, 814623000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(814623000);
		GiveItem(UID, 814664796,1,3);
    	end
    end
end
if (EVENT == 7003) then -- War Tattoo 7 Günlük
	TATTO = HowmuchItem(UID, 814633000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 814633000,1);
		GiveItem(UID, 814664796,1,7);
    	end
    end
end

if (EVENT == 7004) then -- War Tattoo 15 Günlük
	TATTO = HowmuchItem(UID, 814643000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 814643000,1);
		GiveItem(UID, 814664796,1,15);
    	end
    end
end

if (EVENT == 7005) then -- War Tattoo 30 Günlük
	TATTO = HowmuchItem(UID, 814663000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

	    else
		RobItem(UID, 814663000,1);
		GiveItem(UID, 814664796,1,30);
    	end
    end
end

if (EVENT == 2000000) then -- valkry Armor to Yeniceri Armor
	TATTO = HowmuchItem(UID, 800180000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 800180000,1);
		GiveItem(UID, 508117000,1);
    	end
    end
end

----------------------------------------------------------------------------------------

if (EVENT == 7000) then-- YENİ ÇERİ 3 GÜNLÜK EVENT 
	SelectMsg(UID, 3, -1, 11804, NPC,50578,7001,50579,7002);
end

if (EVENT == 7001) then -- YENİ ÇERİ HELMET 3 GÜNLÜK
	ITEMYENIA = HowmuchItem(UID, 508118000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
	SelectMsg(UID, 3, -1, 44474, NPC, 4288, 443, 4289, 444, 4290, 445, 4291, 446);
	end
end

if (EVENT == 7002) then  -- YENİ ÇERİ ARMOR 3 GÜNLÜK 
	ITEMYENIH = HowmuchItem(UID, 508119000);
	if (ITEMYENIH < 1 or ITEMYENIH == 0) then
	SelectMsg(UID, 2, -1, 11797, NPC, 18, 5000);
	else
	SelectMsg(UID, 3, -1, 44474, NPC, 4292, 447, 4293, 448, 4294, 449, 4295, 450);
	end
end
----------------------------------------------------------------------------------------
if (EVENT == 443) then --YENİ ÇERİ HELMET 3 GÜNLÜK
	ITEMYENIA = HowmuchItem(UID, 508118000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RobItem(UID, 508118000, 1);
		GiveItem(UID, 518003636, 1,3);
	end
end
end

if (EVENT == 444) then --YENİ ÇERİ HELMET 3 GÜNLÜK
	ITEMYENIA = HowmuchItem(UID, 508118000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RobItem(UID, 508118000, 1);
		GiveItem(UID, 518003636,1,3);
	end
end
end

if (EVENT == 445) then --YENİ ÇERİ HELMET 3 GÜNLÜK
	ITEMYENIA = HowmuchItem(UID, 508118000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RobItem(UID, 508118000, 1);
		GiveItem(UID, 518003636, 1,3);
	end
end
end

if (EVENT == 446) then--YENİ ÇERİ HELMET 3 GÜNLÜK
	ITEMYENIA = HowmuchItem(UID, 508118000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RobItem(UID, 508118000, 1);
		GiveItem(UID, 518001636, 1,3);
	end
end
end
----------------------------------------------------------------------------------------
if (EVENT == 447) then -- YENİ ÇERİ ARMOR 3 GÜNLÜK 
	ITEMYENIA = HowmuchItem(UID, 508119000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RobItem(UID, 508119000, 1);
		GiveItem(UID, 518001636, 1,30);
	end
end
end

if (EVENT == 448) then -- YENİ ÇERİ ARMOR 3 GÜNLÜK 
	ITEMYENIA = HowmuchItem(UID, 508119000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RobItem(UID, 508119000, 1);
		GiveItem(UID, 518001637,1,30);
	end
end
end

if (EVENT == 449) then -- YENİ ÇERİ ARMOR 3 GÜNLÜK 
	ITEMYENIA = HowmuchItem(UID, 508119000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RobItem(UID, 508119000, 1);
		GiveItem(UID, 518001638, 1,30);
	end
end
end

if (EVENT == 450) then -- YENİ ÇERİ ARMOR 3 GÜNLÜK 
	ITEMYENIA = HowmuchItem(UID, 508119000);
	if (ITEMYENIA < 1 or ITEMYENIA == 0) then
	SelectMsg(UID, 2, -1, 11806, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
		RobItem(UID, 508119000, 1);
		GiveItem(UID, 518001639, 1,30);
	end
end
end
-----------------------------------------------------------------------------
