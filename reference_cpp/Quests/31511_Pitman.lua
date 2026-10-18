-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31511;

if (EVENT == 100) then             
	SelectMsg(UID, 3, -1, 3018, NPC,50600,101, 50611, 499);
end

if EVENT == 101 then
	QuestNum = SearchQuest(UID, NPC);
		if (QuestNum == 0) then 
			 SelectMsg(UID, 2, -1, 1186, NPC, 10, -1);
		elseif (QuestNum > 1 and  QuestNum < 101) then
			NpcMsg(UID, 9170,NPC)
      else 
          EVENT = QuestNum
		end
end

if (EVENT == 400) then
	QuestStatusCheck = GetQuestStatus(UID, 433)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 9172, NPC, 22, 402, 23, -1);
		else
			SelectMsg(UID, 2, -1, 9173, NPC, 4466, 401,4467,-1);
	end
end

if (EVENT == 401) then
	QuestStatusCheck = GetQuestStatus(UID, 433)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 9172, NPC, 10, -1);
		else
			SelectMsg(UID, 2, -1, 9174, NPC, 10, -1);
			GiveItem(UID, 389132000, 1);
			SaveEvent(UID, 20043);
	end
end

if (EVENT == 402) then
	ITEM001 = HowmuchItem(UID, 900000000);
	ITEM002 = HowmuchItem(UID, 389132000);
		if (ITEM001 < 1000000) then
			SelectMsg(UID, 2, -1, 9181, NPC, 18, 1000);
		else
		if (ITEM002 > 0) then
			SelectMsg(UID, 2, -1, 9169, NPC, 27, -1);
		else
			GoldLose(UID, 1000000);
			GiveItem(UID, 389132000, 1);
			SaveEvent(UID, 20050);
		end
	end
end

if (EVENT == 1000) then
	ShowMap(UID, 336);
end

if (EVENT == 200) then
	SelectMsg(UID, 19, -1, 9169, NPC, 10, -1);
end

if (EVENT == 300) then
	QuestStatusCheck = GetQuestStatus(UID, 619)
	QuestStatusCheckII = GetQuestStatus(UID, 620)	
		if(QuestStatusCheck == 1 or QuestStatusCheckII == 1) then
			SelectMsg(UID, 3, -1, 9171, NPC, 7253, 301,7254,302,7164,304);
		else
			SelectMsg(UID, 2, -1, 9171, NPC, 7253, 301,7254,302);
	end
end

if (EVENT == 301) then
	MysteriousORE = HowmuchItem(UID, 399210000);  
		if (MysteriousORE < 1) then 
			SelectMsg(UID, 2, -1, 9179, NPC, 10, -1); 
		else
			EVENT = 305
	end
end

if (EVENT == 302) then
	MysteriousGOLDORE = HowmuchItem(UID, 399200000);  
		if (MysteriousGOLDORE < 1) then 
			SelectMsg(UID, 2, -1, 9179, NPC, 10, -1); 
		else
			EVENT = 306
	end
end

if (EVENT == 305) then
	MysteriousORE = HowmuchItem(UID, 399210000);  
	if (MysteriousORE < 1) then 
		SelectMsg(UID, 2, -1, 9179, NPC, 10, -1); 
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RunMiningExchange(UID,2);
			SelectMsg(UID, 2, -1, 9176, NPC, 10, -1);
		end  
	end
end
	
if (EVENT == 306) then
	MysteriousGOLDORE = HowmuchItem(UID, 399200000);  
	if (MysteriousGOLDORE < 1) then 
		SelectMsg(UID, 2, -1, 9179, NPC, 10, -1); 
	else
		SlotCheck = CheckGiveSlot(UID, 2)
		if SlotCheck == false then
		else
			RunMiningExchange(UID,1);
			SelectMsg(UID, 2, -1, 9176, NPC, 10, -1); 
		end	
	end
end 

if (EVENT == 304) then
    SelectMsg(UID, 2, -1, 22154, NPC, 7253, 307,7254,308);
end

if (EVENT == 307) then
	MysteriousORE = HowmuchItem(UID, 399210000);  
		if (MysteriousORE < 1) then 
			SelectMsg(UID, 2, -1, 9179, NPC, 10, -1); 
		else
			EVENT = 309
	end
end

if (EVENT == 308) then
	MysteriousGOLDORE = HowmuchItem(UID, 399200000);  
		if (MysteriousGOLDORE < 1) then 
			SelectMsg(UID, 2, -1, 9179, NPC, 10, -1); 
		else
			EVENT = 310
	end
end

if (EVENT == 309) then
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
        else
			GiveItem(UID,389770000,1);
			RobItem(UID, 399210000, 1);
	end
end

if (EVENT == 310) then
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			GiveItem(UID,389770000,1);
			RobItem(UID, 399200000, 1);
	end
end

--------Yeni Eklenenler Ferhat--

if (EVENT == 499) then             
	SelectMsg(UID, 3, -1, 3018, NPC,50601,500, 50602, 501,50603,502,50604,503,50605,504,50606,505,50607,506,50608,507,50609,508,50610,509);
end

---300 Sling Kırmızı KUTU
if (EVENT == 500) then
	KIRMIZI = HowmuchItem(UID, 388043000);
	if (KIRMIZI < 300) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,300);
		GiveItem(UID, 379154000,1);
    	end
    end
end


---600 Sling Yeşil KUTU
if (EVENT == 501) then
	YESIL = HowmuchItem(UID, 388043000);
	if (YESIL < 600) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,600);
		GiveItem(UID, 379155000,1);
    	end
    end
end

---900 Sling Blue KUTU
if (EVENT == 502) then
	BLUE = HowmuchItem(UID, 388043000);
	if (BLUE < 900) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,900);
		GiveItem(UID, 379156000,1);
    	end
    end
end

---1200 Silver Gem Blue KUTU
if (EVENT == 503) then
	SILVER = HowmuchItem(UID, 388043000);
	if (SILVER < 1200) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,1200);
		GiveItem(UID, 389196000,1);
    	end
    end
end


---1500 8'li Merchant Kurma İtemi
if (EVENT == 504) then
	MERCHANT = HowmuchItem(UID, 388043000);
	if (MERCHANT < 1500) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,1500);
		GiveItem(UID, 800087000,1);
    	end
    end
end

---2000 Nation Transfer
if (EVENT == 505) then
	NATION = HowmuchItem(UID, 388043000);
	if (NATION < 2000) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,2000);
		GiveItem(UID, 800360000,1);
    	end
    end
end

---3000 Gender Change
if (EVENT == 506) then
	GENDER = HowmuchItem(UID, 388043000);
	if (GENDER < 3000) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,3000);
		GiveItem(UID, 810594000,1);
    	end
    end
end

---3500 Job Change
if (EVENT == 507) then
	JOB = HowmuchItem(UID, 388043000);
	if (JOB < 3500) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,3500);
		GiveItem(UID, 700112000,1);
    	end
    end
end


---5000 500x Rainworm Balık yemi
if (EVENT == 508) then
	RAINWORM = HowmuchItem(UID, 388043000);
	if (RAINWORM < 5000) then
		SelectMsg(UID, 2, -1, 12494, NPC, 7477, -1);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
	    else
		RobItem(UID, 388043000,5000);
		GiveItem(UID, 508226000,500);
    	end
    end
end

---9999 Sling 1 haftalık Dreads Peri
if (EVENT == 509) then 
	DRYADS = HowmuchItem(UID, 388043000);
	if (DRYADS < 1 or DRYADS == 0) then
		SelectMsg(UID, 2, -1, 45065, NPC, 18, 5000);
	else
    EVENT =505
	end
end
---------------
if (EVENT == 505) then 
	DRYADS = HowmuchItem(UID, 388043000);
	if (DRYADS < 9999 or DRYADS == 0) then
		SelectMsg(UID, 2, -1, 45065, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RobItem(UID, 388043000, 9999);
	GiveItem(UID, 700038767, 1,7);
end
end
end



