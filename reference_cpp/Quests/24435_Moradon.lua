-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24435;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 217, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 8258, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 232) then
	SelectMsg(UID, 4, 127, 4931, NPC, 22, 233, 23, -1);
end

if (EVENT == 233) then
	QuestStatus = GetQuestStatus(UID, 127)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 10007);
	end
end

if (EVENT == 235) then
	QuestStatus = GetQuestStatus(UID, 127)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ANIMAL = HowmuchItem(UID, 379273000);
		if (ANIMAL < 3) then
			SelectMsg(UID, 2, 127, 4991, NPC, 19, 238);
		else
			SaveEvent(UID, 10009);
		end
	end
end

if (EVENT == 236) then
	QuestStatus = GetQuestStatus(UID, 127)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ANIMAL = HowmuchItem(UID, 379273000);
		if (ANIMAL < 3) then
			SelectMsg(UID, 2, 127, 4991, NPC, 19, 238);
		else
			SelectMsg(UID, 4, 127, 4998, NPC, 22, 237, 23, -1);
		end
	end
end

if (EVENT == 238) then
	ShowMap(UID, 95);
end

if (EVENT == 237) then
	QuestStatus = GetQuestStatus(UID, 127)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ANIMAL = HowmuchItem(UID, 379273000);
		if (ANIMAL < 3) then
			SelectMsg(UID, 2, 127, 4991, NPC, 19, 238);
		else
			RunQuestExchange(UID,541);
			SaveEvent(UID, 10008);
		end
	end
end

if (EVENT == 242) then
	SelectMsg(UID, 4, 130, 575, NPC, 22, 243, 23, -1);
end

if (EVENT == 243) then
	QuestStatus = GetQuestStatus(UID, 130)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 10019);
	end
end

if (EVENT == 245) then
	QuestStatus = GetQuestStatus(UID, 130)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ROTTEN = HowmuchItem(UID, 379274000);
		if (ROTTEN < 3) then
			SelectMsg(UID, 2, 130, 788, NPC, 19, 248);
		else
			SaveEvent(UID, 10021);
		end
	end
end

if (EVENT == 246) then
	QuestStatus = GetQuestStatus(UID, 130)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ROTTEN = HowmuchItem(UID, 379274000);
		if (ROTTEN < 3) then
			SelectMsg(UID, 2, 130, 788, NPC, 19, 248);
		else
			SelectMsg(UID, 4, 130, 6106, NPC, 22, 247, 23, -1);
		end
	end
end

if (EVENT == 248) then
	ShowMap(UID, 97);
end

if (EVENT == 247) then
	QuestStatus = GetQuestStatus(UID, 130)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ROTTEN = HowmuchItem(UID, 379274000);
		if (ROTTEN < 3) then
			SelectMsg(UID, 2, 130, 788, NPC, 19, 248);
		else
			RunQuestExchange(UID,542);
			SaveEvent(UID, 10020);
		end
	end
end

if (EVENT == 252) then
	SelectMsg(UID, 4, 133, 576, NPC, 22, 253, 23, -1);
end

if (EVENT == 253) then
	QuestStatus = GetQuestStatus(UID, 133)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9861);
	end
end

if (EVENT == 255) then
	QuestStatus = GetQuestStatus(UID, 133)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	FEATHER = HowmuchItem(UID, 379272000);
		if (FEATHER < 3) then
			SelectMsg(UID, 2, 133, 6187, NPC, 19, 258);
		else
			SaveEvent(UID, 9863);
		end
	end
end

if (EVENT == 256) then
	QuestStatus = GetQuestStatus(UID, 133)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	FEATHER = HowmuchItem(UID, 379272000);
		if (FEATHER < 3) then
			SelectMsg(UID, 2, 133, 6187, NPC, 19, 258);
		else
			SelectMsg(UID, 4, 133, 6196, NPC, 22, 257, 23, -1);
		end
	end
end

if (EVENT == 258) then
	ShowMap(UID, 99);
end

if (EVENT == 257) then
	QuestStatus = GetQuestStatus(UID, 133)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	FEATHER = HowmuchItem(UID, 379272000);
		if (FEATHER < 3) then
			SelectMsg(UID, 2, 133, 6187, NPC, 19, 258);
		else
			RunQuestExchange(UID,543);
			SaveEvent(UID, 9862);
		end
	end
end

if (EVENT == 262) then
	SelectMsg(UID, 4, 136, 586, NPC, 22, 263, 23, 158);
end

if (EVENT == 263) then
	QuestStatus = GetQuestStatus(UID, 136)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9873);
	end
end

if (EVENT == 265) then
	QuestStatus = GetQuestStatus(UID, 136)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SKULL = HowmuchItem(UID, 810418000);
		if (SKULL < 3) then
			SelectMsg(UID, 2, 136, 6127, NPC, 19, 268);
		else
			SaveEvent(UID, 9875);
		end
	end
end

if (EVENT == 266) then
	QuestStatus = GetQuestStatus(UID, 136)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SKULL = HowmuchItem(UID, 810418000);
		if (SKULL < 3) then
			SelectMsg(UID, 2, 136, 6127, NPC, 19, 268);
		else
			SelectMsg(UID, 4, 136, 6128, NPC, 22, 267, 23, -1);
		end
	end
end

if (EVENT == 268) then
	ShowMap(UID, 101);
end

if (EVENT == 267) then
	QuestStatus = GetQuestStatus(UID, 136)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SKULL = HowmuchItem(UID, 810418000);
		if (SKULL < 3) then
			SelectMsg(UID, 2, 136, 6127, NPC, 19, 268);
		else
			RunQuestExchange(UID,544);
			SaveEvent(UID, 9874);
		end
	end
end

if (EVENT == 272) then
	SelectMsg(UID, 4, 139, 6139, NPC, 22, 273, 23, -1);
end

if (EVENT == 273) then
	QuestStatus = GetQuestStatus(UID, 139)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9885);
	end
end

if (EVENT == 275) then
	QuestStatus = GetQuestStatus(UID, 139)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COARSE = HowmuchItem(UID, 379275000);
		if (COARSE < 3) then
			SelectMsg(UID, 2, 139, 6141, NPC, 19, 278);
		else
			SaveEvent(UID, 9887);
		end
	end
end

if (EVENT == 276) then
	QuestStatus = GetQuestStatus(UID, 139)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COARSE = HowmuchItem(UID, 379275000);
		if (COARSE < 3) then
			SelectMsg(UID, 2, 139, 6141, NPC, 19, 278);
		else
			SelectMsg(UID, 4, 139, 6142, NPC, 22, 277, 23, -1);
		end
	end
end

if (EVENT == 278) then
	ShowMap(UID, 103);
end

if (EVENT == 277) then
	QuestStatus = GetQuestStatus(UID, 139)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COARSE = HowmuchItem(UID, 379275000);
		if (COARSE < 3) then
			SelectMsg(UID, 2, 139, 6141, NPC, 19, 278);
		else
			RunQuestExchange(UID,545);
			SaveEvent(UID, 9886);
		end
	end
end

if (EVENT == 282) then
	SelectMsg(UID, 4, 142, 590, NPC, 22, 283, 23, -1);
end

if (EVENT == 283) then
	QuestStatus = GetQuestStatus(UID, 142)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9897);
	end
end

if (EVENT == 285) then
	QuestStatus = GetQuestStatus(UID, 142)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	APPLE = HowmuchItem(UID, 810418000);
		if (APPLE < 10 ) then
			SelectMsg(UID, 2, 142, 6141, NPC, 19, 288);
		else
			SaveEvent(UID, 9899);
		end
	end
end

if (EVENT == 286) then
	QuestStatus = GetQuestStatus(UID, 142)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	APPLE = HowmuchItem(UID, 810418000);
		if (APPLE < 10 ) then
			SelectMsg(UID, 2, 142, 6141, NPC, 19, 288);
		else
			SelectMsg(UID, 4, 142, 6142, NPC, 22, 287, 23, -1);
		end
	end
end

if (EVENT == 288) then
	ShowMap(UID, 105);
end

if (EVENT == 287) then
	QuestStatus = GetQuestStatus(UID, 142)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	APPLE = HowmuchItem(UID, 810418000);
		if (APPLE < 10 ) then
			SelectMsg(UID, 2, 142, 6141, NPC, 19, 288);
		else
			RunQuestExchange(UID,546);
			SaveEvent(UID, 9898);
		end
	end
end

if (EVENT == 292) then
	SelectMsg(UID, 4, 145, 6163, NPC, 22, 293, 23, -1);
end

if (EVENT == 293) then
	QuestStatus = GetQuestStatus(UID, 145)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9909);
	end
end

if (EVENT == 295) then
	QuestStatus = GetQuestStatus(UID, 145)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ORK = HowmuchItem(UID, 379277000);
		if (ORK < 7) then
			SelectMsg(UID, 2, 145, 6163, NPC, 19, 298);
		else
			SaveEvent(UID, 9911);
		end
	end
end

if (EVENT == 296) then
	QuestStatus = GetQuestStatus(UID, 145)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ORK = HowmuchItem(UID, 379277000);
		if (ORK < 7) then
			SelectMsg(UID, 2, 145, 6163, NPC, 19, 298);
		else
			SelectMsg(UID, 4, 145, 6142, NPC, 22, 297, 23, -1);
		end
	end
end

if (EVENT == 298) then
	ShowMap(UID, 107);
end

if (EVENT == 297) then
	QuestStatus = GetQuestStatus(UID, 145)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ORK = HowmuchItem(UID, 379277000);
		if (ORK < 7) then
			SelectMsg(UID, 2, 145, 6163, NPC, 19, 298);
		else
			RunQuestExchange(UID,547);
			SaveEvent(UID, 9910);
		end
	end
end

if (EVENT == 302) then
	SelectMsg(UID, 4, 148, 4932, NPC, 22, 303, 23, -1);
end

if (EVENT == 303) then
	QuestStatus = GetQuestStatus(UID, 148)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9921);
	end
end

if (EVENT == 305) then
	QuestStatus = GetQuestStatus(UID, 148)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	OATH = HowmuchItem(UID, 379276000);
		if (OATH < 3) then
			SelectMsg(UID, 2, 148, 4932, NPC, 19, 308);
		else
			SaveEvent(UID, 9923);
		end
	end
end

if (EVENT == 306) then
	QuestStatus = GetQuestStatus(UID, 148)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	OATH = HowmuchItem(UID, 379276000);
		if (OATH < 3) then
			SelectMsg(UID, 2, 148, 4932, NPC, 19, 308);
		else
			SelectMsg(UID, 4, 148, 4934, NPC, 22, 307, 23, -1);
		end
	end
end

if (EVENT == 308) then
	ShowMap(UID, 109);
end

if (EVENT == 307) then
	QuestStatus = GetQuestStatus(UID, 148)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	OATH = HowmuchItem(UID, 379276000);
		if (OATH < 3) then
			SelectMsg(UID, 2, 148, 4932, NPC, 19, 308);
		else
			RunQuestExchange(UID,548);
			SaveEvent(UID, 9922);
		end
	end
end

if (EVENT == 312) then
	SelectMsg(UID, 4, 151, 576, NPC, 22, 313, 23, -1);
end

if (EVENT == 313) then
	QuestStatus = GetQuestStatus(UID, 151)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9933);
	end
end

if (EVENT == 315) then
	QuestStatus = GetQuestStatus(UID, 151)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	FEATHER = HowmuchItem(UID, 379272000);
		if (FEATHER < 7) then
			SelectMsg(UID, 2, 151, 6187, NPC, 19, 318);
		else
			SaveEvent(UID, 9935);
		end
	end
end

if (EVENT == 316) then
	QuestStatus = GetQuestStatus(UID, 151)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	FEATHER = HowmuchItem(UID, 379272000);
		if (FEATHER < 7) then
			SelectMsg(UID, 2, 151, 6187, NPC, 19, 318);
		else
			SelectMsg(UID, 4, 151, 6196, NPC, 22, 317, 23, -1);
		end
	end
end

if (EVENT == 318) then
	ShowMap(UID, 111);
end

if (EVENT == 317) then
	QuestStatus = GetQuestStatus(UID, 151)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	FEATHER = HowmuchItem(UID, 379272000);
		if (FEATHER < 7) then
			SelectMsg(UID, 2, 151, 6187, NPC, 19, 318);
		else
			RunQuestExchange(UID,549);
			SaveEvent(UID, 9934);
		end
	end
end

if (EVENT == 500) then
	SelectMsg(UID, 4, 1373, 44200, NPC, 22, 501, 23, -1);
end

if (EVENT == 501) then
	QuestStatus = GetQuestStatus(UID, 1373)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
		SaveEvent(UID, 3959);
	end
end

if (EVENT == 506) then
	QuestStatus = GetQuestStatus(UID, 1373)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810494000);
		if(ITEMA < 5) then
			SelectMsg(UID, 2, 1373, 44200, NPC, 10, 504);
		else
			SaveEvent(UID, 3961);
		end
	end
end

if (EVENT == 503) then
	QuestStatus = GetQuestStatus(UID, 1373)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810494000);
		if(ITEMA > 4) then
			SelectMsg(UID, 4, 1373, 44200, NPC, 41, 505, 27, -1);
		else
			SelectMsg(UID, 2, 1373, 44200, NPC, 10, 504);
		end
	end
end

if (EVENT == 504) then
	ShowMap(UID, 587);
end

if (EVENT == 505) then
	QuestStatus = GetQuestStatus(UID, 1373)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810494000);
		if(ITEMA < 5) then
			SelectMsg(UID, 2, 1373, 44200, NPC, 10, 504);
		else
			RunQuestExchange(UID, 6161);
			SaveEvent(UID, 3960);
		end
	end
end

if (EVENT == 510) then
	SelectMsg(UID, 4, 1374, 44204, NPC, 22, 511, 23, -1);
end
	
if (EVENT == 511) then
	QuestStatus = GetQuestStatus(UID, 1374)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3969);
	end
end

if (EVENT == 516) then
	QuestStatus = GetQuestStatus(UID, 1374)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810495000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, 1374, 44204, NPC, 10, 217);
		else
			SaveEvent(UID, 3971);
		end
	end
end

if (EVENT == 513) then
	QuestStatus = GetQuestStatus(UID, 1374)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810495000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, 1374, 44204, NPC, 10, 217);
		else
			SelectMsg(UID, 4, 1374, 44204, NPC, 41, 514, 27, -1);
		end
	end
end

if (EVENT == 514)then
	QuestStatus = GetQuestStatus(UID, 1374)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810495000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, 1374, 44204, NPC, 10, 217);
		else
			RunQuestExchange(UID, 6162);
			SaveEvent(UID,3970);
		end
	end
end

if (EVENT == 520) then
	SelectMsg(UID, 4, 1375, 44208, NPC, 22, 521, 23, -1);
end
	
if (EVENT == 521) then
	QuestStatus = GetQuestStatus(UID, 1375)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3979);
	end
end

if (EVENT == 526) then
	QuestStatus = GetQuestStatus(UID, 1375)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810496000);
		if(ITEMA < 15) then
			SelectMsg(UID, 2, 1375, 44208, NPC, 10, 525);
		else
			SaveEvent(UID, 3981);
		end
	end
end

if (EVENT == 523) then
	QuestStatus = GetQuestStatus(UID, 1375)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810496000);
		if(ITEMA < 15) then
			SelectMsg(UID, 2, 1375, 44208, NPC, 10, 525);
		else
			SelectMsg(UID, 4, 1375, 44208, NPC, 41, 524, 27, -1);
		end
	end
end

if (EVENT == 525) then
	ShowMap(UID, 37);
end

if (EVENT == 524)then
	QuestStatus = GetQuestStatus(UID, 1375)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810496000);
		if(ITEMA < 15) then
			SelectMsg(UID, 2, 1375, 44208, NPC, 10, 525);
		else
			RunQuestExchange(UID, 6163);
			SaveEvent(UID,3980);
		end
	end
end

if (EVENT == 530) then
	SelectMsg(UID, 4, 1376, 44212, NPC, 22, 531, 23, -1);
end

if (EVENT == 531) then
	QuestStatus = GetQuestStatus(UID, 1376)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3989);
	end
end

if (EVENT == 536) then
	QuestStatus = GetQuestStatus(UID, 1376)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 810497000);
		if (ITEMA < 3) then
			SelectMsg(UID, 2, 1376, 44212, NPC, 19, -1);
		else
			SaveEvent(UID, 3991);
		end
	end
end

if (EVENT == 533) then
	QuestStatus = GetQuestStatus(UID, 1376)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 810497000);
		if (ITEMA < 3) then
			SelectMsg(UID, 2, 1376, 44212, NPC, 19, -1);
		else
			SelectMsg(UID, 4, 1376, 44212, NPC, 22, 535, 23, -1);
		end
	end
end

if (EVENT == 535) then
	QuestStatus = GetQuestStatus(UID, 1376)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 810497000);
		if (ITEMA < 3) then
			SelectMsg(UID, 2, 1376, 44212, NPC, 19, -1);
		else
			RunQuestExchange(UID, 6164);
			SaveEvent(UID, 3990);
		end
	end
end

if (EVENT == 1002)then
	SelectMsg(UID, 2, 516, 20122, NPC, 4161, 1003);
end

if (EVENT == 1003)then
	SelectMsg(UID, 2, 516, 20123, NPC, 4552, 1004,6004,-1);
	SaveEvent(UID,11002);
end

if (EVENT == 1004)then
	SelectMsg(UID, 4, 516, 20124, NPC, 4161, 1005,3005,-1);
	SaveEvent(UID,11004);
end

if (EVENT == 1005) then 
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	    SaveEvent(UID, 11003);
		SaveEvent(UID, 11035);
		SelectMsg(UID, 2, 516, 20207, NPC,6002, -1);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 11003);
		SaveEvent(UID, 11040);
		SelectMsg(UID, 2, 516, 20207, NPC,6002, -1);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 11003);
		SaveEvent(UID, 11045);
		SelectMsg(UID, 2, 516, 20207, NPC,6002, -1);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 11003);
		SaveEvent(UID, 11050);
		SelectMsg(UID, 2, 516, 20207, NPC,6002, -1);
	end
end

if (EVENT == 1101)then
	SelectMsg(UID, 4, 520, 20009, NPC, 22, 1102,23,-1);
end

if (EVENT == 1102)then
	QuestStatus = GetQuestStatus(UID, 520)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID,11080);
	end
end

if (EVENT == 1106)then
	QuestStatus = GetQuestStatus(UID, 520)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910210000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 520, 20009, NPC, 18, -1);
		else
			SaveEvent(UID,11082);
		end
	end
end

if (EVENT == 1103) then
	QuestStatus = GetQuestStatus(UID, 520)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910210000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 520, 20009, NPC, 18, -1);
		else
			SelectMsg(UID, 4, 520, 20009, NPC, 22, 1105, 27, -1); 
		end
	end
end

if (EVENT == 1105)then
	QuestStatus = GetQuestStatus(UID, 520)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910210000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 520, 20009, NPC, 18, -1);
		else
			RunQuestExchange(UID,3007);
			SaveEvent(UID,11081);
			SaveEvent(UID,11092);
		end
	end
end

if (EVENT == 1201)then
	SelectMsg(UID, 4, 521, 20011, NPC, 22, 1202,23,-1);
end

if (EVENT == 1202)then
	SaveEvent(UID,11092);
end

if (EVENT == 1206)then
	SaveEvent(UID,11094);
end

if (EVENT == 1203) then
	SelectMsg(UID, 4, 521, 20011, NPC, 22, 1205, 27, -1); 
end

if (EVENT == 1205)then
RunQuestExchange(UID,3008);
	SaveEvent(UID,11093);
	SaveEvent(UID,11104);
end