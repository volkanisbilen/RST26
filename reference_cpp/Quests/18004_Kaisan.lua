-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 18004;

if (EVENT == 100) then
	NpcMsg(UID, 147, NPC)
end

if (EVENT == 403) then
	ITEM_COUNT = HowmuchItem(UID, 900000000);
		if (ITEM_COUNT < 3000) then
			SelectMsg(UID, 2, 71, 4065, NPC, 10, -1);
		else
	QuestStatusCheck = GetQuestStatus(UID, 71) 
		if(QuestStatusCheck == 1) then
			SaveEvent(UID, 4064);
			SelectMsg(UID, 2, 71, 4064, NPC, 4062, 404,4063,-1);
		else
		if(QuestStatusCheck == 3) then
			SelectMsg(UID, 2, 71, 4064, NPC, 4062, 404,4063,-1);
		else
			SaveEvent(UID, 4062);
			SaveEvent(UID, 4064);
			SelectMsg(UID, 2, 71, 4064, NPC, 4062, 404,4063,-1);
			end
		end
	end
end

if (EVENT == 404) then
	PromoteUserNovice(UID);
	GoldLose(UID, 3000);
	SaveEvent(UID, 4063);
end

if (EVENT == 407) then
	SelectMsg(UID, 2, -1, 4070, NPC, 4070, 408,10,-1);
end

if (EVENT == 408) then
	SendStatSkillDistribute(UID);
end

if (EVENT == 10000) then
	SelectMsg(UID, 2, 73, 1549, NPC, 10,10001);
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

if (EVENT == 499) then
	SelectMsg(UID, 2, -1, 1546, NPC, 7012, 500,7013,505);
end

if (EVENT == 500) then
	SelectMsg(UID, 2, -1, 1532, NPC, 10, -1);
end

if (EVENT == 505) then
	SelectMsg(UID, 2, -1, 1532, NPC, 10, -1);
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

if (EVENT == 799) then
	SelectMsg(UID, 2, -1, 10745, NPC, 22,800,23,-1);
end

if (EVENT == 800) then
	SendRepurchaseMsg(UID);
end

if (EVENT == 200) then
	CLANCONT = HowmuchItem(UID, 810323000);   
		if (CLANCONT == 0) then
			SelectMsg(UID, 2, -1, 11644, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 11644, NPC, 7014, 201, 73, -1);
	end
end	

if (EVENT == 201) then
	SelectMsg(UID, 2, -1, 11644, NPC, 3000, 202, 73, -1);
end

if (EVENT == 202) then
	CLANCONT = HowmuchItem(UID, 810323000);   
		if (CLANCONT == 0) then
			SelectMsg(UID, 2, -1, 11644, NPC, 18,5000);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810323000, 1);
			GiveItem(UID, 810324000, 1,1);
		end
	end
end

if (EVENT == 302) then
	SelectMsg(UID, 2, 1208, 43642, NPC, 40148, 303);
end

if (EVENT == 303) then
	SelectMsg(UID, 4, 1208, 43643, NPC, 22, 304, 23, -1);
end

if (EVENT == 304) then
	SaveEvent(UID, 7357)
end

if(EVENT == 305) then
	CountA = HowmuchItem(UID, 810418000)
	if( CountA < 20) then
		SelectMsg(UID, 2, 1208, 43643, NPC, 18, 306);
	else
		SelectMsg(UID, 4, 1208, 43643, NPC, 41, 308, 27, -1);
	end
end

if (EVENT == 306 ) then
	ShowMap(UID, 2)
end

if (EVENT == 307) then
	SaveEvent(UID, 7359)
end

if(EVENT == 308) then
	QuestStatusCheck = GetQuestStatus(UID, 1208) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	CountA = HowmuchItem(UID, 810418000)
		if( CountA < 20) then
			SelectMsg(UID, 2, 1208, 43643, NPC, 18, 306);
		else
			RunQuestExchange(UID, 6005);
			SaveEvent(UID, 7358)
		end
	end
end

if (EVENT == 1201) then
	SelectMsg(UID, 4, 661, 21302, NPC, 22, 1202, 23, -1);
end

if (EVENT == 1202) then
	SaveEvent(UID, 12733);
end

if (EVENT == 1206) then
	SaveEvent(UID, 12735);
end

if (EVENT == 1205) then
	ITEM1_COUNT = HowmuchItem(UID, 900194000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 661, 21302, NPC, 18,-1);
	else
		SelectMsg(UID, 4, 661, 21302, NPC, 22, 1207, 27, -1);
	end
end	

if (EVENT == 1207) then
	QuestStatusCheck = GetQuestStatus(UID, 661) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		SelectMsg(UID, 2, 661, 21851, NPC, 10,-1);
		SaveEvent(UID,12734);
		SaveEvent(UID,12745);
		RunQuestExchange(UID,3146);
	end
end

if (EVENT == 1000) then
	ShowMap(UID, 450);
	SelectMsg(UID, 8, -1, -1, NPC);	
end

if (EVENT == 1010) then
	SelectMsg(UID, 2, -1, 12144, NPC, 40206, 1011);
end

if (EVENT == 1011) then
	SelectMsg(UID, 2, -1, 12145, NPC, 3000, 1012,3005,-1);
end

if (EVENT == 1012) then
MONEY = HowmuchItem(UID, 900000000);
WARPRE = GetPremium(UID);
	if (WARPRE == 12 or WARPRE == 13) then 
		SelectMsg(UID, 52, -1, -1, NPC);
	else
	if (MONEY >= 100000000) then
		SelectMsg(UID, 52, -1, -1, NPC);
	else
		SelectMsg(UID, 2, -1, 12147, NPC, 56,-1);
		end
	end
end

if (EVENT == 5000) then
	ShowMap(UID, 450);
end