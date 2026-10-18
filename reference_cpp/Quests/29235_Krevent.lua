
--  ==========================================================  --
-- |		  RİMA GUARD  //  www.RimaGUARD.com 		      | --
-- | Knight Online Pvp v24xx Server Files & AntiCheat System  | --
--  ==========================================================  -- 

local NPC = 29235;

if (EVENT == 100) then-- 8365,101,9018,650,8534,300,8603,500,7582,600,40389,800,
	--SelectMsg(UID, 3, -1, 11624, NPC,7582,600,50546,2000,49006,1500,50540,950,50536,999,50541,997,50542,996,50544,850 );
	SelectMsg(UID, 3, -1, 11624, NPC, 45209, 1402,45216,2000,45217,1500,45227,996,45233,850);  --,50619,9000,7582,600
end

if (EVENT == 101) then
	SelectMsg(UID, 3, -1, 11624, NPC,8749,102,9001,106,8473,117,8290,130,8359,170,8360,180,8479,190,8466,200,8467,210);
end

if (EVENT == 102) then
	SelectMsg(UID, 3, -1, 12494, NPC,40613,103,40614,104,40615,105);
end

if (EVENT == 103) then
	EMBLEM = HowmuchItem(UID, 914042000);
	if (EMBLEM < 1) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 914042000,1);
		GiveItem(UID, 914041877,1,1);
    	end
    end
end

if (EVENT == 104) then
	EMBLEM = HowmuchItem(UID, 914044000);
	if (EMBLEM < 1) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 914044000,1);
		GiveItem(UID, 914041877,1,7);
    	end
    end
end	

if (EVENT == 105) then
	EMBLEM = HowmuchItem(UID, 914046000);
	if (EMBLEM < 1) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 914046000,1);
		GiveItem(UID, 914041877,1,30);
    	end
    end
end

if (EVENT == 106) then
	SelectMsg(UID, 3, -1, 12446, NPC,9003,107,9004,108,9005,109,9006,110,9007,111,9008,112,9009,113,9010,114,9011,115,9012,116);
end

if (EVENT == 107) then -- Dark Knight Dagger +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111466741,1);
    	end
    end
end

if (EVENT == 108) then -- Dark Knight Axe +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111473741,1);
    	end
    end
end

if (EVENT == 109) then -- Dark Knight Spear +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111468741,1);
    	end
    end
end

if (EVENT == 110) then -- Dark Knight Crossbow +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111469741,1);
    	end
    end
end

if (EVENT == 111) then -- Fire Dark Knight Staff +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111470561,1);
    	end
    end
end

if (EVENT == 112) then -- Ice Dark Knight Staff +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111470571,1);
    	end
    end
end

if (EVENT == 113) then -- Lightning Dark Knight Staff +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111470581,1);
    	end
    end
end

if (EVENT == 114) then -- Dark Knight Mace +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111471741,1);
    	end
    end
end

if (EVENT == 115) then -- Dark Knight Shield +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111472741,1);
    	end
    end
end

if (EVENT == 116) then -- Dark Knight Jamadar +1
	CHEST = HowmuchItem(UID, 810635000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12447, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810635000,1);
		GiveItem(UID, 1111467741,1);
    	end
    end
end

if (EVENT == 117) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
        SelectMsg(UID, 3, -1, 11677, NPC, 8474,118,8475,119,8476,120,8477,121);
    	end
end

if (EVENT == 118) then
    SelectMsg(UID, 3, -1, 11677, NPC, 8443,211,8444,212,8445,213,8446,214,8447,215);
end

if (EVENT == 119) then
    SelectMsg(UID, 3, -1, 11677, NPC, 8448,216,8449,217,8450,218,8451,219,8452,220);
end

if (EVENT == 120) then
    SelectMsg(UID, 3, -1, 11677, NPC, 8453,221,8454,222,8455,223,8456,224,8457,225);
end

if (EVENT == 121) then
    SelectMsg(UID, 3, -1, 11677, NPC, 8458,226,8459,227,8460,228,8461,229,8462,230);
end

if (EVENT == 211) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 521511007,1);
    	end
    end
end

if (EVENT == 212) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 521512007,1);
    	end
    end
end

if (EVENT == 213) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 521513007,1);
    	end
    end
end

if (EVENT == 214) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 521514007,1);
    	end
    end
end

if (EVENT == 215) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 521515007,1);
    	end
    end
end

if (EVENT == 216) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 522511007,1);
    	end
    end
end

if (EVENT == 217) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 522512007,1);
    	end
    end
end

if (EVENT == 218) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 522513007,1);
    	end
    end
end

if (EVENT == 219) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 522514007,1);
    	end
    end
end

if (EVENT == 220) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 522515007,1);
    	end
    end
end

if (EVENT == 221) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 523511007,1);
    	end
    end
end

if (EVENT == 222) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 523512007,1);
    	end
    end
end

if (EVENT == 223) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 523513007,1);
    	end
    end
end

if (EVENT == 224) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 523514007,1);
    	end
    end
end

if (EVENT == 225) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 523515007,1);
    	end
    end
end

if (EVENT == 226) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 524511007,1);
    	end
    end
end

if (EVENT == 227) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 524512007,1);
    	end
    end
end

if (EVENT == 228) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 524513007,1);
    	end
    end
end

if (EVENT == 229) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 524514007,1);
    	end
    end
end

if (EVENT == 230) then
	CHEST = HowmuchItem(UID, 900672000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 10, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1);
     if SlotCheck == false then
       ;
	    else
		RobItem(UID, 900672000,1);
		GiveItem(UID, 524515007,1);
    	end
    end
end
	
if (EVENT == 130) then
	SelectMsg(UID, 3, -1, 11624, NPC,8293,131,8465,132);
end

if (EVENT == 131) then -- Wing of Hero
	WING = HowmuchItem(UID, 810251000);
	if (WING < 1) then
		SelectMsg(UID, 2, -1, 11739, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810251000,1);
		GiveItem(UID, 900030684,1,7);
    	end
    end
end

if (EVENT == 132) then -- Wing of Neophyte
	WING = HowmuchItem(UID, 900700000);
	if (WING < 1) then
		SelectMsg(UID, 2, -1, 11879, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900700000,1);
		GiveItem(UID, 900702863,1,30);
    	end
    end
end

if (EVENT == 170) then
	EMBLEM = HowmuchItem(UID, 900594000);
	if (EMBLEM < 1) then
		SelectMsg(UID, 2, -1, 11801, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900594000,1);
		GiveItem(UID, 900595860,30);
    	end
    end
end

if (EVENT == 180) then
	EMBLEM = HowmuchItem(UID, 900596000);
	if (EMBLEM < 1) then
		SelectMsg(UID, 2, -1, 11802, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900596000,1);
		GiveItem(UID, 900597861,30);
    	end
    end
end

if (EVENT == 190) then
	EMBLEM = HowmuchItem(UID, 900703000);
	if (EMBLEM < 1) then
		SelectMsg(UID, 2, -1, 11065, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900703000,1);
		GiveItem(UID, 900704864,30);
    	end
    end
end

if (EVENT == 200) then
	POT = HowmuchItem(UID, 889310000);
	if (POT < 1) then
		SelectMsg(UID, 2, -1, 11065, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 889310000,1);
		GiveItem(UID, 389310000,1);
    	end
    end
end

if (EVENT == 210) then
	POT = HowmuchItem(UID, 889340000);
	if (POT < 1) then
		SelectMsg(UID, 2, -1, 11065, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 889340000,1);
		GiveItem(UID, 389340000,1);
    	end
    end
end

if (EVENT == 500) then
SelectMsg(UID,3,-1,12006,NPC,8604,501,8605,502,8606,503,8607,504)
end

if (EVENT == 501) then
SelectMsg(UID,3,-1,11807,NPC,8564,505,8565,506,8566,507,8567,508,8568,509,8569,510,8570,511,8571,512,8572,513)
end

if (EVENT == 502) then
SelectMsg(UID,3,-1,11807,NPC,8564,514,8565,515,8566,516,8567,517,8568,518,8569,519,8570,520,8571,521,8572,522)
end

if (EVENT == 503) then
SelectMsg(UID,3,-1,11807,NPC,8564,523,8565,524,8566,525,8567,526,8568,527,8569,528,8570,529,8571,530,8572,531)
end

if (EVENT == 504) then
SelectMsg(UID,3,-1,11807,NPC,8564,532,8565,533,8566,534,8567,535,8568,536,8569,537,8570,538,8571,539,8572,540)
end

if (EVENT == 505) then
	GARGES = HowmuchItem(UID, 900764000); 
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110581734,1); -- Garges Dagger
    	end
    end
end

if (EVENT == 506) then
	GARGES = HowmuchItem(UID, 900764000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110582734,1);-- Garges Two-Handed Sword
    	end
    end
end

if (EVENT == 507) then
	GARGES = HowmuchItem(UID, 900764000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110583734,1);-- Garges Scythe
    	end
    end
end

if (EVENT == 508) then
	GARGES = HowmuchItem(UID, 900764000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110586734,1);
    	end
    end
end

if (EVENT == 509) then
	GARGES = HowmuchItem(UID, 900764000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110585534,1);
    	end
    end
end

if (EVENT == 510) then
	GARGES = HowmuchItem(UID, 900764000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110584734,1);
    	end
    end
end

if (EVENT == 511) then
	GARGES = HowmuchItem(UID, 900764000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110587734,1);
    	end
    end
end

if (EVENT == 512) then
	GARGES = HowmuchItem(UID, 900764000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110585544,1);
    	end
    end
end

if (EVENT == 513) then
	GARGES = HowmuchItem(UID, 900764000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900764000,1);
		GiveItem(UID, 1110585554,1);
    	end
    end
end

if (EVENT == 514) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110581735,1);
    	end
    end
end

if (EVENT == 515) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110582735,1);
    	end
    end
end

if (EVENT == 516) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110583735,1);
    	end
    end
end

if (EVENT == 517) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110586735,1);
    	end
    end
end

if (EVENT == 518) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110585535,1);
    	end
    end
end

if (EVENT == 519) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110584735,1);
    	end
    end
end

if (EVENT == 520) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110587735,1);
    	end
    end
end

if (EVENT == 521) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110585545,1);
    	end
    end
end

if (EVENT == 522) then
	GARGES = HowmuchItem(UID, 900763000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900763000,1);
		GiveItem(UID, 1110585555,1);
    	end
    end
end

if (EVENT == 523) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110581736,1);
    	end
    end
end

if (EVENT == 524) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110582736,1);
    	end
    end
end

if (EVENT == 525) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110583736,1);
    	end
    end
end

if (EVENT == 526) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110586736,1);
    	end
    end
end

if (EVENT == 527) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110585536,1);
    	end
    end
end

if (EVENT == 528) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110584736,1);
    	end
    end
end

if (EVENT == 529) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110587736,1);
    	end
    end
end

if (EVENT == 530) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110585546,1);
    	end
    end
end

if (EVENT == 531) then
	GARGES = HowmuchItem(UID, 900762000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900762000,1);
		GiveItem(UID, 1110585556,1);
    	end
    end
end

if (EVENT == 532) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900761000,1);
		GiveItem(UID, 1110581737,1);
    	end
    end
end

if (EVENT == 533) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900761000,1);
		GiveItem(UID, 1110582737,1);
    	end
    end
end

if (EVENT == 534) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900761000,1);
		GiveItem(UID, 1110583737,1);
    	end
    end
end

if (EVENT == 535) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900761000,1);
		GiveItem(UID, 1110586737,1);
    	end
    end
end

if (EVENT == 536) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900761000,1);
		GiveItem(UID, 1110585537,1);
    	end
    end
end

if (EVENT == 537) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900761000,1);
		GiveItem(UID, 1110584737,1);
    	end
    end
end

if (EVENT == 538) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 900761000,1);
		GiveItem(UID, 1110587737,1);
    	end
    end
end

if (EVENT == 539) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 900761000,1);
			GiveItem(UID, 1110585547,1);
    	end
    end
end

if (EVENT == 540) then
	GARGES = HowmuchItem(UID, 900761000);
	if (GARGES < 1) then
		SelectMsg(UID, 2, -1, 12007, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 900761000,1);
			GiveItem(UID, 1110585557,1);
    	end
    end
end

if (EVENT == 600) then
	TRADE = HowmuchItem(UID, 800149000); -- War Wing 30 DAY
	if (TRADE < 1 ) then
		SelectMsg(UID, 2, -1, 12378, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 2601, 43605, NPC, 4006, 601, 4005, -1);
	end
end

if EVENT == 601 then 
	TRADE = HowmuchItem(UID, 800149000);
	if (TRADE < 1) then
		SelectMsg(UID, 2, -1, 12378, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			SelectMsg(UID,2,-1,12377,NPC,10,-1);
			Check = CheckExchange(UID, 2626)
			if  (Check == true) then   
				Roll = RollDice(UID, 25) 
				found = Roll + 2601
				RunQuestExchange(UID, found);
			end  
		end
	end
end

if (EVENT == 650) then
	SelectMsg(UID, 2, -1, 12467, NPC, 9030, 651);
end

if (EVENT == 651) then
	SelectMsg(UID, 2, -1, 12468, NPC, 9031, 652,27,-1);
end

if (EVENT == 652) then
	SelectMsg(UID, 3, -1, 12469, NPC, 9032,653,9033,654,9034,655,9035,656);
end

if (EVENT == 653) then
	CHEST = HowmuchItem(UID, 998011000);
	if (CHEST < 1) then
		SelectMsg(UID, 2, -1, 12474, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 998011000,1);
			SaveEvent(UID, 00000);
    	end
    end
end

if (EVENT == 654) then
	CHEST = HowmuchItem(UID, 998011000);
	if (CHEST < 5) then
		SelectMsg(UID, 2, -1, 12474, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 998011000,5);
			SaveEvent(UID, 00000);
    	end
    end
end

if (EVENT == 655) then
	CHEST = HowmuchItem(UID, 998011000);
	if (CHEST < 10) then
		SelectMsg(UID, 2, -1, 12474, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 998011000,10);
			SaveEvent(UID, 00000);
    	end
    end
end

if (EVENT == 656) then
	CHEST = HowmuchItem(UID, 998011000);
	if (CHEST < 20) then
		SelectMsg(UID, 2, -1, 12474, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 998011000,20);
			SaveEvent(UID, 00000);
    	end
    end
end

if (EVENT == 800) then
	SelectMsg(UID, 3, -1, 12448, NPC, 40671,801,40601,803,40602,805,40591,807,40592,810);
end

if (EVENT == 801) then
	SelectMsg(UID, 2, -1, 12448, NPC,40672,802);
end

if (EVENT == 802) then
	TATTO = HowmuchItem(UID, 914009000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 914009000,1);
			GiveItem(UID, 810932954,1,1);
    	end
    end
end

if (EVENT == 803) then
	SelectMsg(UID, 2, -1, 12448, NPC,40603,804,49007,35264);
end

if (EVENT == 804) then
	TATTO = HowmuchItem(UID, 910925000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 910925000,1);
			GiveItem(UID, 810941998,1,3);
    	end
    end
end

if (EVENT == 805) then
	SelectMsg(UID, 2, -1, 12448, NPC,40604,806,49008,35265);
end

if (EVENT == 806) then
	TATTO = HowmuchItem(UID, 910926000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 910926000,1);
			GiveItem(UID, 810942999,1,7);
    	end
    end
end

if (EVENT == 35265) then
	TATTO = HowmuchItem(UID, 810922000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810922000,1);
		GiveItem(UID, 810930952,1,7);
    	end
    end
end

if (EVENT == 807) then
	SelectMsg(UID, 2, -1, 12448, NPC,40390,808,40535,809);
end

if (EVENT == 808) then
	TATTO = HowmuchItem(UID, 810686000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 810686000,1);
			GiveItem(UID, 810932954,1,15);
    	end
    end
end

if (EVENT == 809) then
	TATTO = HowmuchItem(UID, 810713000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 810713000,1);
			GiveItem(UID, 810931953,1,15);
    	end
    end
end

if (EVENT == 810) then
	SelectMsg(UID, 3, -1, 12448, NPC,40587,811,40588,812,40589,813,40590,814);
end

if (EVENT == 811) then
	TATTO = HowmuchItem(UID, 810926000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 810926000,1);
			GiveItem(UID, 810930952,1,30);
    	end
    end
end

if (EVENT == 812) then
	TATTO = HowmuchItem(UID, 810927000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 810927000,1);
			GiveItem(UID, 810931953,1,30);
    	end
    end
end

if (EVENT == 813) then
	TATTO = HowmuchItem(UID, 810928000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 810928000,1);
			GiveItem(UID, 810932954,1,30);
    	end
    end
end

if (EVENT == 814) then
	TATTO = HowmuchItem(UID, 810929000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 810929000,1);
			GiveItem(UID, 810933955,1,30);
    	end
    end
end

------------------------------------------------------------------------------------------------------------------------------
-- KNİGHT CASH EXCHANGE (100-350-700-1000-2000-5000 CASH)

if (EVENT == 1500) then  ----------- KNIGHT CASH MENU
	SelectMsg(UID, 3, -1, 45054, NPC, 45218, 1501, 45219, 1502, 45220, 1503, 45221, 1504, 45223, 1505,45225,1506,45226,1507,4296, 100);
end

if (EVENT == 1501) then  -- 100 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700082000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 91000, NPC, 40326, -1); --44302
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700082000,1);
		GiveCash(UID,100);
	end
end

if (EVENT == 1502) then  -- 350 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700083000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 91000, NPC, 40326, -1);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700083000,1);
		GiveCash(UID,350);
	end
end

if (EVENT == 1503) then  -- 700 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700084000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 91000, NPC, 40326, -1);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700084000,1);
		GiveCash(UID,700);
	end
end

if (EVENT == 1504) then  -- 1000 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700085000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 91000, NPC, 40326, -1);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700085000,1);
		GiveCash(UID,1000);
	end
end

if (EVENT == 1505) then -- 2000 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700086000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 91000, NPC, 40326, -1);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700086000,1);
		GiveCash(UID,2000);
	end
end

if (EVENT == 1506) then  -- 5000 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700087000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 91000, NPC, 40326, -1);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700087000,1);
		GiveCash(UID,5000);
	end
end

if (EVENT == 1507) then  -- 10000 KNIGHT CASH
	ITEMA = HowmuchItem(UID, 700088000);
	if (ITEMA == 0) then
		SelectMsg(UID, 2, -1, 91000, NPC, 40326, -1);
	else
		SelectMsg(UID, 2, -1, 91001, NPC, 40326, -1);
		RobItem(UID,700088000,1);
		GiveCash(UID,10000);
	end
end

if (EVENT == 35264) then
	TATTO = HowmuchItem(UID, 810921000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810921000,1);
		GiveItem(UID, 810930952,1,3);
    	end
    end
end

if (EVENT == 35264) then
	TATTO = HowmuchItem(UID, 810921000);
	if (TATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810921000,1);
		GiveItem(UID, 810930952,1,3);
    	end
    end
end

--Just Go
if (EVENT == 1000) then 
    --Ret = 1;
end

------------------------------------------------------------------------------------------------------------------------------
-- NATION TRANSFER EXCHANGE (IRK DEĞİŞTİRME)

if (EVENT == 999) then  ----------- NATION TRANSFER MENU
	SelectMsg(UID, 3, -1, 45056, NPC, 50537, 10000, 50538,410, 50539,699);
end

if (EVENT == 10000) then
	SelectMsg(UID, 19, 73, 1549, NPC, 10,10001);
end

if (EVENT == 10001) then
    SaveEvent(UID, 4070);
end

if (EVENT == 410) then
	SelectMsg(UID, 2, -1, 1522, NPC, 7014, 411,73,-1);
end

if (EVENT == 411) then
	SelectMsg(UID, 2, -1, 1533, NPC, 10, 412);
end


if (EVENT == 412) then
	SelectMsg(UID, 2, -1, 1534, NPC, 3000, 413,3005,-1);
end

if (EVENT == 413) then
	NTS = HowmuchItem(UID, 800360000);
		if NTS == 0 then
			SelectMsg(UID, 2, -1, 1532, NPC,10,-1);
		else
			RobItem(UID, 800360000, 1);
			GiveItem(UID, 810096000, 1,1);
   end
end

if (EVENT == 699) then
	NTS2 = HowmuchItem(UID, 810096000);
		if NTS2 == 0 then
			SelectMsg(UID, 2, -1, 1523, NPC, 18,5000);
		else
			SelectMsg(UID, 2, -1, 1524, NPC, 72, 700,73,-1);
	end
end

if (EVENT == 700) then
	NTS2 = HowmuchItem(UID, 810096000);
		if NTS2 == 0 then
			SelectMsg(UID, 2, -1, 1523, NPC,10,-1);
		else
			SendNationTransfer(UID);
	end
end	  

------------------------------------------------------------------------------------------------------------------------------
-- GENDER EXCHANGE (TİP DEĞİŞTİRME)

if (EVENT == 997) then  ----------- GENDER CHANGE  MENU
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
-- AUTO MINING - AUTO FISHING - ROBIN LOOTING

if (EVENT == 996) then    ----------- MINING-FISHING-ROBIN MENU
	SelectMsg(UID, 3, -1, 45036, NPC, 45228, 1031, 45229, 1034, 45230,1556, 45231,1041,45232,1051, 4296, 100);
end

if (EVENT == 1555) then
	SelectMsg(UID, 2, -1, 45062, NPC, 3000, 1556, 3005, -1);
end

if (EVENT == 15566) then  -- ROBIN LOOTING VOUCHER
	SelectMsg(UID, 2, -1, 45061, NPC, 50580, -1);  -- 45058 kırdırılan item
	ROBINLOOT = HowmuchItem(UID, 750680000);
	if (ROBINLOOT < 1 or ROBINLOOT == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 18, 5000);
	else
    EVENT = 1557
	end
end

if (EVENT == 1556) then
	ROBINLOOT = HowmuchItem(UID, 750680000); 
	if (ROBINLOOT < 1 or ROBINLOOT == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6931, 45062, NPC, 4006, 1557, 4005, -1);
	end
end

if (EVENT == 1557) then
	ROBINLOOT = HowmuchItem(UID, 750680000);
	if (ROBINLOOT < 1 or ROBINLOOT == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,649, 1);
		end
	end
end

------------------------------------------------------------------------------------------------------------------------------
-- AUTO MİNİNG VOUCHER

if (EVENT == 1030) then
	SelectMsg(UID, 2, -1, 45062, NPC, 3000, 1031, 3005, -1); --  45062 İtemi kırdırmak istediğinize Eminmisiniz?
end 

if (EVENT == 10031) then   -- AUTO MİNİNG VOUCHER - 15 Günlük
	SelectMsg(UID, 2, -1, 45061, NPC, 50580, -1);  -- 45058 Tebrikler! |Auto Mining Voucher Kırdırılmıştır.
	AUTOMINING = HowmuchItem(UID, 800610000);
	if (AUTOMINING < 1 or AUTOMINING == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50581, 5000);
	else
    EVENT = 1032
	end
end

if (EVENT == 1031) then
	AUTOMINING = HowmuchItem(UID, 800610000); 
	if (AUTOMINING < 1 or AUTOMINING == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6929, 45062, NPC, 4006, 1032, 4005, -1);
	end
end


if (EVENT == 1032) then
	AUTOMINING = HowmuchItem(UID, 800610000);
	if (AUTOMINING < 1 or AUTOMINING == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50580, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
    if SlotCheck == false then
    else
	RunGiveItemExchange(UID,647, 1);
		end
	end
end


------------------------------------------------------------------------------------------------------------------------------
-- AUTO MİNİNG + ROBIN VOUCHER

if (EVENT == 1040) then
	SelectMsg(UID, 2, -1, 45062, NPC, 3000, 1041, 3005, -1); --  45062 İtemi kırdırmak istediğinize Eminmisiniz?
end 

if (EVENT == 10041) then   -- AUTO MİNİNG VOUCHER - 15 Günlük
	SelectMsg(UID, 2, -1, 45064, NPC, 50580, -1);  -- 45058 Tebrikler! |Auto Mining Voucher Kırdırılmıştır.
	MININGROBIN = HowmuchItem(UID, 511000000);
	if (MININGROBIN < 1 or MININGROBIN == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50581, 5000);
	else
    EVENT = 1042
	end
end

if (EVENT == 1041) then
	MININGROBIN = HowmuchItem(UID, 511000000); 
	if (MININGROBIN < 1 or MININGROBIN == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6932, 45062, NPC, 4006, 1042, 4005, -1);
	end
end


if (EVENT == 1042) then
	MININGROBIN = HowmuchItem(UID, 511000000);
	if (MININGROBIN < 1 or MININGROBIN == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50580, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,650, 1);
		end
	end
end


-----------------------------------------------------------------------------------------------------------------------------
-- AUTO FISHING VOUCHER
if (EVENT == 1033) then
	SelectMsg(UID, 2, -1, 45062, NPC, 3000, 1034, 3005, -1); --  45062 İtemi kırdırmak istediğinize Eminmisiniz?
end 

if (EVENT == 10034) then  -- AUTO FISHING + ROBİN LOOT VOUCHER - 15 Günlük
	SelectMsg(UID, 2, -1, 45060, NPC, 50580, -1);  -- 45060 Tebrikler! |Auto Mining Voucher Kırdırılmıştır.
	AUTOFISHING = HowmuchItem(UID, 800620000);
	if (AUTOFISHING < 1 or AUTOFISHING == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50581, 5000);
	else
    EVENT = 1035
	end
end

if (EVENT == 1034) then
	AUTOFISHING = HowmuchItem(UID, 800620000); 
	if (AUTOFISHING < 1 or AUTOFISHING == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6930, 45062, NPC, 4006, 1035, 4005, -1);
	end
end

if (EVENT == 1035) then
	AUTOFISHING = HowmuchItem(UID, 800620000);
	if (AUTOFISHING < 1 or AUTOFISHING == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50580, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,648, 1);
end
end
end

------------------------------------------------------------------------------------------------------------------------------
-- AUTO FISHING + ROBIN VOUCHER

if (EVENT == 1050) then
	SelectMsg(UID, 2, -1, 45062, NPC, 3000, 1051, 3005, -1); --  45062 İtemi kırdırmak istediğinize Eminmisiniz?
end 

if (EVENT == 10051) then   -- AUTO MİNİNG + ROBİN LOOTVOUCHER - 15 Günlük 
	SelectMsg(UID, 2, -1, 45063, NPC, 50580, -1);  -- 45058 Tebrikler! |Auto Mining Voucher Kırdırılmıştır. -- yanlış kodlama
	FISHINGROBIN = HowmuchItem(UID, 522000000);
	if (FISHINGROBIN < 1 or FISHINGROBIN == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50581, 5000);
	else
    EVENT = 1052
	end
end

if (EVENT == 1051) then
	FISHINGROBIN = HowmuchItem(UID, 522000000); 
	if (FISHINGROBIN < 1 or FISHINGROBIN == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6933, 45062, NPC, 4006, 1052, 4005, -1);
	end
end


if (EVENT == 1052) then
	FISHINGROBIN = HowmuchItem(UID, 522000000);
	if (FISHINGROBIN < 1 or FISHINGROBIN == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50580, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,651, 1);
		end
	end
end

------------------------------------------------------------------------------------------------------------------------------
-- OFFLINE MERCHANT
--if (EVENT == 850) then
	--SelectMsg(UID, 2, -1, 45062, NPC, 3000, 851, 3005, -1);   --  45062 İtemi kırdırmak istediğinize Eminmisiniz?
--end 

if (EVENT == 850) then
	OFFLINEMERCHANT = HowmuchItem(UID, 700111000); 
	if (OFFLINEMERCHANT < 1 or OFFLINEMERCHANT == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6934, 45062, NPC, 4006, 853, 4005, -1);
	end
end


if (EVENT == 853) then
	OFFLINEMERCHANT = HowmuchItem(UID, 700111000);
	if (OFFLINEMERCHANT < 1 or OFFLINEMERCHANT == 0) then
		SelectMsg(UID, 2, -1, 45059, NPC, 50580, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunGiveItemExchange(UID,652, 1);
	--SelectMsg(UID, 2, -1, 45058, NPC, 50580, -1);  -- 45058 Tebrikler! |Offline Merchant Voucher Kırdırılmıştır -->reward ekranı ekli gerek kalmadı.
		end
	end
end


if (EVENT == 2000) then
	SelectMsg(UID, 3, -1, 44263, NPC,45210,3004,45211,4004,45212,5004,45213,6004,45215,7004, 4296, 100);
end
----------------------------------------------------------------------------------------
if (EVENT == 2001) then --LUNAR TATTOO
	SelectMsg(UID, 3, -1, 44263, NPC, 50556,3005); --50552,3001,50553,3002,50554,3003,50555,3004,
end

if (EVENT == 3001) then -- Lunar Tattoo 1 Günlük
	LUNARTATTO = HowmuchItem(UID, 931706000);
	if (LUNARTATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
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
	LUNARTATTO = HowmuchItem(UID, 910925000);
	if (LUNARTATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
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
	LUNARTATTO = HowmuchItem(UID, 910926000);
	if (LUNARTATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 910926000,1);
		GiveItem(UID, 810931953,1,7);
    	end
    end
end

if (EVENT == 30004) then -- Lunar Tattoo 15 Günlük
	LUNARTATTO = HowmuchItem(UID, 810713000);
	if (LUNARTATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810713000,1);
		GiveItem(UID, 810931953,1,15);
    	end
    end
end

if (EVENT == 3004) then
	LUNARTATTO = HowmuchItem(UID, 810927000); -- Lunar Tattoo 30 Günlük
	if (LUNARTATTO < 1 or LUNARTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6924, 43605, NPC, 4006, 3005, 4005, -1);
	end
end


if (EVENT == 3005) then
	LUNARTATTO = HowmuchItem(UID, 810927000);
	if (LUNARTATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
	RunGiveItemExchange(UID,642, 1);
    	end
    end
end
----------------------------------------------------------------------------------

if (EVENT == 2002) then  --Solar tattoo
	SelectMsg(UID, 3, -1, 11804, NPC, 50561,4005); --50557,4001,50558,4002,50559,4003,50560,4004,
end

if (EVENT == 4001) then -- Solar Tattoo 1 Günlük
	SOLARTATTO = HowmuchItem(UID, 810921000);
	if (SOLARTATTO < 1) then
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
	SOLARTATTO = HowmuchItem(UID, 810922000);
	if (SOLARTATTO < 1) then
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
	SOLARTATTO = HowmuchItem(UID, 810923000);
	if (SOLARTATTO < 1) then
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


if (EVENT == 40004) then -- Solar Tattoo 15 Günlük
	SOLARTATTO = HowmuchItem(UID, 810924000);
	if (SOLARTATTO < 1 or LUNARTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 810924000,1);
		GiveItem(UID, 810431970,1,15);
    	end
    end
end

if (EVENT == 4004) then
	SOLARTATTO = HowmuchItem(UID, 810926000); -- Solar Tattoo 30 Günlük
	if (SOLARTATTO < 1 or SOLARTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6925, 43605, NPC, 4006, 4005, 4005, -1);
	end
end

if (EVENT == 4005) then
	SOLARTATTO = HowmuchItem(UID, 810926000);
	if (SOLARTATTO < 1 or SOLARTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RunGiveItemExchange(UID,643, 1);
    	end
    end
end

----------------------------------------------------------------------------------------

if (EVENT == 2003) then  -- Nimbus Tattoo		
	SelectMsg(UID, 3, -1, 11804, NPC, 50566,5005); --50562,5001,50563,5002,50564,5003,50565,5004,
end

if (EVENT == 5001) then -- Nimbus Tattoo 1 Günlük
	NIMBUSTATTO = HowmuchItem(UID, 931705000);
	if (NIMBUSTATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931705000,1);
		GiveItem(UID, 810434973,1,1);
    	end
    end
end

if (EVENT == 5002) then -- Nimbus Tattoo 3 Günlük
	NIMBUSTATTO = HowmuchItem(UID, 931755000);
	if (NIMBUSTATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(931755000);
		GiveItem(UID, 810434973,1,3);
    	end
    end
end
if (EVENT == 5003) then -- Nimbus Tattoo 7 Günlük
	NIMBUSTATTO = HowmuchItem(UID, 931765000);
	if (NIMBUSTATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931765000,1);
		GiveItem(UID, 810434973,1,7);
    	end
    end
end

if (EVENT == 50004) then -- Nimbus Tattoo 15 Günlük
	NIMBUSTATTO = HowmuchItem(UID, 931775000);
	if (NIMBUSTATTO < 1) then
		SelectMsg(UID, 2, -1, 91002, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 931775000,1);
		GiveItem(UID, 810434973,1,15);
    	end
    end
end

if (EVENT == 5004) then
	NIMBUSTATTO = HowmuchItem(UID, 810929000); -- Nimbus Tattoo 30 Günlük
	if (NIMBUSTATTO < 1 or NIMBUSTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6926, 43605, NPC, 4006, 5005, 3005, -1);
	end
end

if (EVENT == 5005) then 
	NIMBUSTATTO = HowmuchItem(UID, 810929000);
	if (NIMBUSTATTO < 1 or NIMBUSTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RunGiveItemExchange(UID,644, 1);
    	end
    end
end

----------------------------------------------------------------------------------------
if (EVENT == 2004) then  -- Stellar Tattoo
	SelectMsg(UID, 3, -1, 11804, NPC, 50571,6005); --50567,6001,50568,6002,50569,6003,50570,6004,
end

if (EVENT == 6001) then -- Stellar Tattoo 1 Günlük
	STELLARTATTO = HowmuchItem(UID, 914009000);
	if (STELLARTATTO < 1) then
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
	STELLARTATTO = HowmuchItem(UID, 931766000);
	if (STELLARTATTO < 1) then
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
	STELLARTATTO = HowmuchItem(UID, 931666000);
	if (STELLARTATTO < 1) then
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
	STELLARTATTO = HowmuchItem(UID, 931657000);
	if (STELLARTATTO < 1) then
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

if (EVENT == 6004) then
	STELLARTATTO = HowmuchItem(UID, 810928000);  -- Stellar Tattoo 30 Günlük
	if (STELLARTATTO < 1 or STELLARTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6927, 43605, NPC, 4006, 6005, 4005, -1);
	end
end


if (EVENT == 6005) then
	STELLARTATTO = HowmuchItem(UID, 810928000);
	if (STELLARTATTO < 1 or STELLARTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RunGiveItemExchange(UID,645, 1);
    	end
    end
end

----------------------------------------------------------------------------------------

if (EVENT == 2005) then  --War Tattoo
	SelectMsg(UID, 3, -1, 11804, NPC, 50576,7005); --50572,7001,50573,7002,50574,7003,50575,7004,
end

if (EVENT == 7001) then -- War Tattoo 1 Günlük
	WARTATTO = HowmuchItem(UID, 814613000);
	if (WARTATTO < 1) then
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

if (EVENT == 7002) then -- War Tattoo 3 Günlük
	WARTATTO = HowmuchItem(UID, 814623000);
	if (WARTATTO < 1) then
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
	WARTATTO = HowmuchItem(UID, 814633000);
	if (WARTATTO < 1) then
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

if (EVENT == 70004) then -- War Tattoo 15 Günlük
	WARTATTO = HowmuchItem(UID, 814643000);
	if (WARTATTO < 1) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 814643000,1);
		GiveItem(UID, 814664796,1,15);
    	end
    end
end

if (EVENT == 7004) then
	WARTATTO = HowmuchItem(UID, 814663000); -- War Tattoo 30 Günlük
	if (WARTATTO < 1 or WARTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 18, 5000);
	else
		SelectMsg(UID, 4, 6928, 43605, NPC, 4006, 7005, 3005, -1);
	end
end


if (EVENT == 7005) then 
	WARTATTO = HowmuchItem(UID, 814663000);
	if (WARTATTO < 1 or WARTATTO == 0) then
		SelectMsg(UID, 2, -1, 44268, NPC, 27, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RunGiveItemExchange(UID,646, 1);
    	end
    end
end

if (EVENT == 5000) then
	ShowMap(UID, 450);
end
-----------------------------------

if (EVENT == 9000) then -- Change ID Menu
	SelectMsg(UID, 3, -1, 9527, NPC, 50620, 1400, 50621, 1401, 50622, 1402,4296,100);
end

if (EVENT == 1400) then -- NCS
	NCS = HowmuchItem(UID, 800032000);
	if (NCS < 1) then
		SelectMsg(UID, 2, -1, 4454, NPC, 18, 5000);
	else
		SendNameChange(UID);
	end
end

if (EVENT == 1401) then -- Clan NCS
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

if (EVENT == 1402) then -- TCS
	TCS = HowmuchItem(UID, 800099000);
	if (TCS < 1) then
		SelectMsg(UID, 2, -1, 4454, NPC, 18, 5000);
	else
		SendTagNameChangePanel(UID);
	end
end