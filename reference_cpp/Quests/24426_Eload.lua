-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24426;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4597, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 4598, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 170) then
	SelectMsg(UID, 4, 280, 235, NPC, 22, 171, 23, -1);
end

if (EVENT == 171) then
	QuestStatus = GetQuestStatus(UID, 280)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 270);
	end
end

if (EVENT == 174) then
	QuestStatus = GetQuestStatus(UID, 280)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 280, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 280, 235, NPC, 18, -1);
		else
			SaveEvent(UID, 206);
			SelectMsg(UID, 2, 280, 235, NPC, 14, -1);
		end
	end
end

if (EVENT == 175) then
	QuestStatus = GetQuestStatus(UID, 280)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 280, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 280, 235, NPC, 18, -1);
		else
			SelectMsg(UID, 4, 280, 235, NPC, 41, 176, 27, -1);
		end
	end
end

if (EVENT == 177) then
	ShowMap(UID, 80);
end

if (EVENT == 176) then
	QuestStatus = GetQuestStatus(UID, 280)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 280, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 280, 235, NPC, 18, -1);
		else
			RunQuestExchange(UID, 13);
			SaveEvent(UID, 271); 
		end   
	end
end

if (EVENT == 180) then
	SelectMsg(UID, 4, 281, 253, NPC, 22, 181, 23, -1);
end

if (EVENT == 181) then
	QuestStatus = GetQuestStatus(UID, 281)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 214);
	end
end

if (EVENT == 184) then
	QuestStatus = GetQuestStatus(UID, 281)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 281, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 281, 253, NPC, 18, 187);
		else
			SaveEvent(UID, 216);
			SelectMsg(UID, 2, 281, 253, NPC, 14, -1);
		end
	end
end

if (EVENT == 185) then
	QuestStatus = GetQuestStatus(UID, 281)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 281, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 281, 253, NPC, 18, 187);
		else
			SelectMsg(UID, 4, 281, 253, NPC, 41, 186, 27, -1);
		end
	end
end

if (EVENT == 187) then
	ShowMap(UID, 86);
end

if (EVENT == 186) then
	QuestStatus = GetQuestStatus(UID, 281)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 281, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 281, 253, NPC, 18, 187);
		else
			RunQuestExchange(UID, 14);
			SaveEvent(UID, 215); 
		end   
	end
end

if (EVENT == 200) then
	SelectMsg(UID, 4, 282, 265, NPC, 22, 201, 23, -1);
end

if (EVENT == 201) then
	QuestStatus = GetQuestStatus(UID, 282)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 224);
	end
end

if (EVENT == 204) then
	QuestStatus = GetQuestStatus(UID, 282)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 282, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 282, 2);
		if (MonsterCount01 < 20 and MonsterCount02 < 20) then
			SelectMsg(UID, 2, 282, 265, NPC, 18, 207);
		else
			SaveEvent(UID, 226);
			SelectMsg(UID, 2, 282, 265, NPC, 14, -1);
		end
	end
end

if (EVENT == 205) then
	QuestStatus = GetQuestStatus(UID, 282)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 282, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 282, 2);
		if (MonsterCount01 < 20 and MonsterCount02 < 20) then
			SelectMsg(UID, 2, 282, 265, NPC, 18, 207);
		else
			SelectMsg(UID, 4, 282, 265, NPC, 41, 206, 27, -1);
		end
	end
end

if (EVENT == 207) then
	ShowMap(UID, 83);
end

if (EVENT == 206) then
	QuestStatus = GetQuestStatus(UID, 282)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 282, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 282, 2);
		if (MonsterCount01 < 20 and MonsterCount02 < 20) then
			SelectMsg(UID, 2, 282, 265, NPC, 18, 207);
		else
			RunQuestExchange(UID, 15);
			SaveEvent(UID, 225); 
		end   
	end
end

if (EVENT == 210) then
	SelectMsg(UID, 4, 283, 276, NPC, 22, 211, 23, -1);
end

if (EVENT == 211) then
	QuestStatus = GetQuestStatus(UID, 283)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 234);
	end
end

if (EVENT == 214) then
	QuestStatus = GetQuestStatus(UID, 283)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 283, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 283, 276, NPC, 18, 217);
		else
			SaveEvent(UID, 236);
			SelectMsg(UID, 2, 283, 276, NPC, 14, -1);
		end
	end
end

if (EVENT == 215) then
	QuestStatus = GetQuestStatus(UID, 283)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 283, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 283, 276, NPC, 18, 217);
		else
			SelectMsg(UID, 4, 283, 276, NPC, 41, 216, 27, -1);
		end
	end
end

if (EVENT == 217) then
	ShowMap(UID, 87);
end

if (EVENT == 216) then
	QuestStatus = GetQuestStatus(UID, 283)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 283, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 283, 276, NPC, 18, 217);
		else
			RunQuestExchange(UID, 16);
			SaveEvent(UID, 235); 
		end  
	end
end

if (EVENT == 220) then
	SelectMsg(UID, 4, 284, 8649, NPC, 22, 221, 23, -1);
end

if (EVENT == 221) then
	QuestStatus = GetQuestStatus(UID, 284)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 244);
	end
end

if (EVENT == 224) then
	QuestStatus = GetQuestStatus(UID, 284)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 284, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 284, 8649, NPC, 18, 227);
		else
			SaveEvent(UID, 246);
			SelectMsg(UID, 2, 284, 8649, NPC, 14, -1);
		end
	end
end

if (EVENT == 225) then
	QuestStatus = GetQuestStatus(UID, 284)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 284, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 284, 8649, NPC, 18, 227);
		else
			SelectMsg(UID, 4, 284, 8649, NPC, 41, 226, 27, -1);
		end
	end
end

if (EVENT == 227) then
	ShowMap(UID, 88);
end

if (EVENT == 226) then
	QuestStatus = GetQuestStatus(UID, 284)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 284, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 284, 8649, NPC, 18, 227);
		else
			RunQuestExchange(UID, 17);
			SaveEvent(UID, 245); 
		end  
	end
end

if (EVENT == 160) then
	SelectMsg(UID, 4, 279, 215, NPC, 22, 161, 23, -1);
end

if (EVENT == 161) then
	QuestStatus = GetQuestStatus(UID, 279)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 260);
	end
end

if (EVENT == 164) then
	QuestStatus = GetQuestStatus(UID, 279)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 279, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 279, 215, NPC, 18, 167);
		else
			SaveEvent(UID, 262);
			SelectMsg(UID, 2, 279, 215, NPC, 14, -1);
		end
	end
end

if (EVENT == 165) then
	QuestStatus = GetQuestStatus(UID, 279)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 279, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 279, 215, NPC, 18, 167);
		else
			SelectMsg(UID, 4, 279, 215, NPC, 41, 166, 27, -1);
		end
	end
end

if (EVENT == 167) then
	ShowMap(UID, 81);
end

if (EVENT == 166) then
	QuestStatus = GetQuestStatus(UID, 279)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 279, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 279, 215, NPC, 18, 167);
		else
			RunQuestExchange(UID, 12);
			SaveEvent(UID, 261); 
		end   
	end
end

if (EVENT == 532) then
	SelectMsg(UID, 4, 340, 723, NPC, 22, 533, 23, -1);
end

if (EVENT == 533) then
	QuestStatus = GetQuestStatus(UID, 340)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 827);
	end
end

if (EVENT == 535) then
	QuestStatus = GetQuestStatus(UID, 340)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 340, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 340, 723, NPC, 18, 538);
		else
			SaveEvent(UID, 829);
			SelectMsg(UID, 2, 340, 723, NPC, 14, -1);
		end
	end
end

if (EVENT == 536) then
	QuestStatus = GetQuestStatus(UID, 340)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 340, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 340, 723, NPC, 18, 538);
		else
			SelectMsg(UID, 4, 340, 723, NPC, 4172, 537, 4173, -1);
		end
	end
end

if (EVENT == 538) then
	ShowMap(UID, 489);
end

if (EVENT == 537) then
	QuestStatus = GetQuestStatus(UID, 340)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 340, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 340, 723, NPC, 18, 538);
		else
			RunQuestExchange(UID,133);
			SaveEvent(UID, 828);   
		end
	end
end

if (EVENT == 321) then
	SelectMsg(UID, 4, 342, 3120, NPC, 22, 322, 23, -1);
end

if (EVENT == 322) then
	QuestStatus = GetQuestStatus(UID, 342)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 906);
	end
end

if (EVENT == 324) then
	QuestStatus = GetQuestStatus(UID, 342)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 342, 1);
		if (MonsterCount < 250) then
			SelectMsg(UID, 2, 342, 3120, NPC, 18, 327);
		else
			SaveEvent(UID, 908);
			SelectMsg(UID, 2, 342, 3120, NPC, 14, -1);
		end
	end
end

if (EVENT == 325) then
	QuestStatus = GetQuestStatus(UID, 342)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 342, 1);
		if (MonsterCount < 250) then
			SelectMsg(UID, 2, 342, 3120, NPC, 18, 327);
		else
			SelectMsg(UID, 4, 342, 3120, NPC, 41, 326, 27, -1);
		end
	end
end

if (EVENT == 327) then
	ShowMap(UID, 169);
end

if (EVENT == 326) then
	QuestStatus = GetQuestStatus(UID, 342)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 342, 1);
		if (MonsterCount < 250) then
			SelectMsg(UID, 2, 342, 3120, NPC, 18, 327);
		else
			RunQuestExchange(UID,157);
			SaveEvent(UID, 907);   
		end
	end
end

if (EVENT == 311) then
	SelectMsg(UID, 4, 356, 365, NPC, 22, 312, 23, -1);
end

if (EVENT == 312) then
	QuestStatus = GetQuestStatus(UID, 356)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 894);
	end
end

if (EVENT == 314) then
	QuestStatus = GetQuestStatus(UID, 356)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 356, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 356, 2);
	MonsterCount03 = CountMonsterQuestSub(UID, 356, 3);
		if ( MonsterCount01 < 60) then
			SelectMsg(UID, 2, 356, 4626, NPC, 18, 317);
		elseif ( MonsterCount02 < 60) then
			SelectMsg(UID, 2, 356, 4627, NPC, 18, 318);
		elseif ( MonsterCount03 < 80) then
			SelectMsg(UID, 2, 356, 4628, NPC, 18, 319);
		else
			SaveEvent(UID, 896);
			SelectMsg(UID, 2, 356, 365, NPC, 14, -1);
		end
	end
end

if (EVENT == 315) then
	QuestStatus = GetQuestStatus(UID, 356)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 356, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 356, 2);
	MonsterCount03 = CountMonsterQuestSub(UID, 356, 3);
		if ( MonsterCount01 < 60) then
			SelectMsg(UID, 2, 356, 4626, NPC, 18, 317);
		elseif ( MonsterCount02 < 60) then
			SelectMsg(UID, 2, 356, 4627, NPC, 18, 318);
		elseif ( MonsterCount03 < 80) then
			SelectMsg(UID, 2, 356, 4628, NPC, 18, 319);
		else
			SelectMsg(UID, 4, 356, 371, NPC, 41, 316, 27, -1);
		end
	end
end


if (EVENT == 317) then
	ShowMap(UID, 168);
end

if (EVENT == 318) then
	ShowMap(UID, 167);
end

if (EVENT == 319) then
	ShowMap(UID, 166);
end

if (EVENT == 316) then
	QuestStatus = GetQuestStatus(UID, 356)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 356, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 356, 2);
	MonsterCount03 = CountMonsterQuestSub(UID, 356, 3);
		if (MonsterCount01 < 60) then
			SelectMsg(UID, 2, 356, 4626, NPC, 18, 317);
		elseif (MonsterCount02 < 60) then
			SelectMsg(UID, 2, 356, 4627, NPC, 18, 318);
		elseif (MonsterCount03 < 80) then
			SelectMsg(UID, 2, 356, 4628, NPC, 18, 319);
		else
			RunQuestExchange(UID,156);
			SaveEvent(UID, 895);
		end	
	end
end

if (EVENT == 302) then
	SelectMsg(UID, 4, 369, 357, NPC, 22, 303, 23, -1);
end

if (EVENT == 303) then
	QuestStatus = GetQuestStatus(UID, 369)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 852);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 857);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 862);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 867);
		end
	end
end

if (EVENT == 305) then
	QuestStatus = GetQuestStatus(UID, 369)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 369,1);
	MonsterCount2 = CountMonsterQuestSub(UID, 369,2);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, 369, 357, NPC, 18, 306);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, 369, 357, NPC, 18, 309);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 854);
			SelectMsg(UID, 2, 369, 357, NPC, 14, -1);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 859);
			SelectMsg(UID, 2, 369, 357, NPC, 14, -1);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 864);
			SelectMsg(UID, 2, 369, 357, NPC, 14, -1);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 869);
			SelectMsg(UID, 2, 369, 357, NPC, 14, -1);
			end
		end
	end
end

if (EVENT == 307) then
	QuestStatus = GetQuestStatus(UID, 369)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 369,1);
	MonsterCount2 = CountMonsterQuestSub(UID, 369,2);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, 369, 357, NPC, 18, 306);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, 369, 357, NPC, 18, 309);
		else
			SelectMsg(UID, 5, 369, 357, NPC, 41, 308, 27, -1);
		end
	end
end

if (EVENT == 306) then
	ShowMap(UID, 165);
end

if (EVENT == 309) then
	ShowMap(UID, 163);
end

if (EVENT == 308) then
	QuestStatus = GetQuestStatus(UID, 369)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 369,1);
	MonsterCount2 = CountMonsterQuestSub(UID, 369,2);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, 369, 357, NPC, 18, 306);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, 369, 357, NPC, 18, 309);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,148,STEP,1);
			SaveEvent(UID, 853);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,149,STEP,1);
			SaveEvent(UID, 858);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,150,STEP,1);
			SaveEvent(UID, 863);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,151,STEP,1);
			SaveEvent(UID, 868);
			end  
		end
	end
end

if (EVENT == 400) then
SelectMsg(UID, 4, 444, 6133, NPC, 22, 401,23,-1);
end

if (EVENT == 401) then
	QuestStatus = GetQuestStatus(UID, 444)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SelectMsg(UID, 15, 444, -1, NPC);
			SaveEvent(UID, 7145);
			RunQuestExchange(UID,536);
	end
end