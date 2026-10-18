-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24424;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4513, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 4514, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 9542) then
	SelectMsg(UID, 4, 302, 8772, NPC, 22, 9543, 23, -1);
end

if (EVENT == 9543) then
	QuestStatus = GetQuestStatus(UID, 302)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9724);
	end
end

if (EVENT == 9546) then
	QuestStatus = GetQuestStatus(UID, 302)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 302, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 302, 8772, NPC, 18, 9548);
		else
			SaveEvent(UID, 9726);
		end
	end
end

if (EVENT == 9547) then
	QuestStatus = GetQuestStatus(UID, 302)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 302, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 302, 8772, NPC, 18, 9548);
		else
			SelectMsg(UID, 4, 302, 8772, NPC, 41, 9549, 27, -1);
		end
	end
end

if (EVENT == 9548) then
	ShowMap(UID, 627);
end

if (EVENT == 9549) then
	QuestStatus = GetQuestStatus(UID, 302)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 302, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 302, 8772, NPC, 18, 9548);
		else
			RunQuestExchange(UID,1154);
			SaveEvent(UID, 9725);
		end
	end
end

if (EVENT == 532) then   
	SelectMsg(UID, 4, 317, 4505, NPC, 22, 533, 23, 534);
end

if (EVENT == 533) then
	GiveItem(UID, 910127000, 1);
	SaveEvent(UID, 4267);
end

if (EVENT == 535) then
	SaveEvent(UID, 4269);
end

if (EVENT == 536) then
	ItemA = HowmuchItem(UID, 910134000);
	ItemB = HowmuchItem(UID, 910127000);
	if (ItemA < 1) then
		if (ItemB < 1) then
			Check = isRoomForItem(UID, 910127000);
			if (Check == -1) then
				SelectMsg(UID, 2, 317, 832, NPC, 27 , -1);
			else
				GiveItem(UID, 910127000, 1);
				SelectMsg(UID, 2, 317, 4507, NPC, 18, 538);
			end
		else
			SelectMsg(UID, 2, 317, 4508, NPC, 18, 538);
		end
	else
		SelectMsg(UID, 2, 317, 4506, NPC, 4172, 537, 4173, -1);
	end
end

if (EVENT == 538) then
	ShowMap(UID, 454);
end

if (EVENT == 537) then
	QuestStatus = GetQuestStatus(UID, 317)	
	SlotCheck = CheckGiveSlot(UID, 4)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ItemA = HowmuchItem(UID, 910134000);
	ItemB = HowmuchItem(UID, 910127000);
	if (ItemA < 1) then
		if (ItemB < 1) then
			Check = isRoomForItem(UID, 910127000);
			if (Check == -1) then
				SelectMsg(UID, 2, 317, 832, NPC, 27 , -1);
			else
				RunQuestExchange(UID, 481);
				SaveEvent(UID, 4268); 
				end
			end
		end
	end
end

if (EVENT == 9372) then
	SelectMsg(UID, 4, 311, 8686, NPC, 22, 9373, 23, -1);
end

if (EVENT == 9373) then
	QuestStatus = GetQuestStatus(UID, 311)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9400);
	end
end

if (EVENT == 9376) then
	QuestStatus = GetQuestStatus(UID, 311)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 311, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 311, 8686, NPC, 18, 9378);
		else
			SaveEvent(UID, 9402);
		end
	end
end

if (EVENT == 9377) then
	QuestStatus = GetQuestStatus(UID, 311)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 311, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 311, 8686, NPC, 18, 9378);
		else
			SelectMsg(UID, 4, 311, 8686, NPC, 41, 9379, 27, -1);
		end
	end
end

if (EVENT == 9378) then
	ShowMap(UID, 607);
end

if (EVENT == 9379) then
	QuestStatus = GetQuestStatus(UID, 311)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 311, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 311, 8686, NPC, 18, 9378);
		else
			RunQuestExchange(UID,1096);
			SaveEvent(UID, 9401);
		end
	end
end

if (EVENT == 9382) then
	SelectMsg(UID, 4, 313, 8688, NPC, 22, 9383, 23, -1);
end

if (EVENT == 9383) then
	QuestStatus = GetQuestStatus(UID, 313)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9412);
	end
end

if (EVENT == 9386) then
	QuestStatus = GetQuestStatus(UID, 313)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 313, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 313, 8688, NPC, 18, 9388);
		else
			SaveEvent(UID, 9414);
		end
	end
end

if (EVENT == 9387) then
	QuestStatus = GetQuestStatus(UID, 313)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 313, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 313, 8688, NPC, 18, 9388);
		else
			SelectMsg(UID, 4, 313, 8688, NPC, 41, 9389, 27, -1);
		end
	end
end

if (EVENT == 9388) then
	ShowMap(UID, 609);
end

if (EVENT == 9389) then
	QuestStatus = GetQuestStatus(UID, 313)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 313, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 313, 8688, NPC, 18, 9388);
		else
			RunQuestExchange(UID,1098);
			SaveEvent(UID, 9413);
		end
	end
end

if (EVENT == 9392) then
	SelectMsg(UID, 4, 315, 8690, NPC, 22, 9393, 23, -1);
end

if (EVENT == 9393) then
	QuestStatus = GetQuestStatus(UID, 315)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9424);
	end
end

if (EVENT == 9396) then
	QuestStatus = GetQuestStatus(UID, 315)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 315, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 315, 8690, NPC, 18, 9398);
		else
			SaveEvent(UID, 9426);
		end
	end
end

if (EVENT == 9397) then
	QuestStatus = GetQuestStatus(UID, 315)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 315, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 315, 8690, NPC, 18, 9398);
		else
			SelectMsg(UID, 4, 315, 8690, NPC, 41, 9399, 27, -1);
		end
	end
end

if (EVENT == 9398) then
	ShowMap(UID, 611);
end

if (EVENT == 9399) then
	QuestStatus = GetQuestStatus(UID, 315)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 315, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 315, 8690, NPC, 18, 9398);
		else
			RunQuestExchange(UID,1100);
			SaveEvent(UID, 9425);
		end
	end
end

if (EVENT == 9412) then
	SelectMsg(UID, 4, 319, 8694, NPC, 22, 9413, 23, -1);
end

if (EVENT == 9413) then
	QuestStatus = GetQuestStatus(UID, 319)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9448);
	end
end

if (EVENT == 9416) then
	QuestStatus = GetQuestStatus(UID, 319)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 319, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 319, 8694, NPC, 18, 9418);
		else
			SaveEvent(UID, 9450);
		end
	end
end

if (EVENT == 9417) then
	QuestStatus = GetQuestStatus(UID, 319)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 319, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 319, 8694, NPC, 18, 9418);
		else
			SelectMsg(UID, 4, 319, 8694, NPC, 41, 9419, 27, -1);
		end
	end
end

if (EVENT == 9418) then
	ShowMap(UID, 616);
end

if (EVENT == 9419) then
	QuestStatus = GetQuestStatus(UID, 319)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 319, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 319, 8694, NPC, 18, 9418);
		else
			RunQuestExchange(UID,1103);
			SaveEvent(UID, 9449);
		end
	end
end


if (EVENT == 9422) then
	SelectMsg(UID, 4, 321, 8696, NPC, 22, 9423, 23, -1);
end

if (EVENT == 9423) then
	QuestStatus = GetQuestStatus(UID, 321)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9460);
	end
end

if (EVENT == 9426) then
	QuestStatus = GetQuestStatus(UID, 321)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 321, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 321, 8696, NPC, 18, 9428);
		else
			SaveEvent(UID, 9462);
		end
	end
end

if (EVENT == 9427) then
	QuestStatus = GetQuestStatus(UID, 321)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 321, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 321, 8696, NPC, 18, 9428);
		else
			SelectMsg(UID, 4, 321, 8696, NPC, 41, 9429, 27, -1);
		end
	end
end

if (EVENT == 9428) then
	ShowMap(UID, 618);
end

if (EVENT == 9429) then
	QuestStatus = GetQuestStatus(UID, 321)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 321, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 321, 8696, NPC, 18, 9428);
		else
			RunQuestExchange(UID,1104);
			SaveEvent(UID, 9461);
		end
	end
end

if (EVENT == 632) then
	SelectMsg(UID, 4, 338, 4614, NPC, 22, 633, 23, -1);
end

if (EVENT == 633) then
	QuestStatus = GetQuestStatus(UID, 338)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 4334);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 4339);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 4344);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 4349);
		end
	end
end

if (EVENT == 280) then
	QuestStatus = GetQuestStatus(UID, 338)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 338,1);
	MonsterCount02 = CountMonsterQuestSub(UID, 338,2);
	MonsterCount03 = CountMonsterQuestSub(UID, 338,3);
	MonsterCount04 = CountMonsterQuestSub(UID, 338,4);
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 338, 4617, NPC, 18, 638);
		elseif ( MonsterCount02 < 1) then
			SelectMsg(UID, 2, 338, 4618, NPC, 18, 639);
		elseif ( MonsterCount03 < 1) then
			SelectMsg(UID, 2, 338, 4619, NPC, 18, 640);
		elseif ( MonsterCount04 < 1) then
			SelectMsg(UID, 2, 338, 4620, NPC, 18, 641);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 4336);
			EVENT = 281
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 4341);
			EVENT = 281
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 4346);
			EVENT = 281
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 4351);
			EVENT = 281
			end
		end
	end
end

if (EVENT == 281) then
	SelectMsg(UID, 2, 338, 4616, NPC, 14, -1);
end

if (EVENT == 636) then
	QuestStatus = GetQuestStatus(UID, 338)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 338,1);
	MonsterCount02 = CountMonsterQuestSub(UID, 338,2);
	MonsterCount03 = CountMonsterQuestSub(UID, 338,3);
	MonsterCount04 = CountMonsterQuestSub(UID, 338,4);
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 338, 4617, NPC, 18, 638);
		elseif ( MonsterCount02 < 1) then
			SelectMsg(UID, 2, 338, 4618, NPC, 18, 639);
		elseif ( MonsterCount03 < 1) then
			SelectMsg(UID, 2, 338, 4619, NPC, 18, 640);
		elseif ( MonsterCount04 < 1) then
			SelectMsg(UID, 2, 338, 4620, NPC, 18, 641);
		else
			SelectMsg(UID, 5, 338, 4621, NPC, 41, 637,27, -1);
		end
	end
end

if (EVENT == 638) then
	ShowMap(UID, 474);
end

if (EVENT == 639) then
	ShowMap(UID, 475);
end

if (EVENT == 640) then
	ShowMap(UID, 476);
end

if (EVENT == 641) then
	ShowMap(UID, 477);
end

if (EVENT == 637) then
	QuestStatus = GetQuestStatus(UID, 338)	
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		elseif(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 338,1);
	MonsterCount02 = CountMonsterQuestSub(UID, 338,2);
	MonsterCount03 = CountMonsterQuestSub(UID, 338,3);
	MonsterCount04 = CountMonsterQuestSub(UID, 338,4);
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 338, 4617, NPC, 18, 638);
		elseif ( MonsterCount02 < 1) then
			SelectMsg(UID, 2, 338, 4618, NPC, 18, 639);
		elseif ( MonsterCount03 < 1) then
			SelectMsg(UID, 2, 338, 4619, NPC, 18, 640);
		elseif ( MonsterCount04 < 1) then
			SelectMsg(UID, 2, 338, 4620, NPC, 18, 641);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,493,STEP,1);
			SaveEvent(UID, 4335);
			ShowEffect(UID, 300391);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,494,STEP,1);
			SaveEvent(UID, 4340);
			ShowEffect(UID, 300391);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,495,STEP,1);
			SaveEvent(UID, 4345);
			ShowEffect(UID, 300391);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,496,STEP,1);
			SaveEvent(UID, 4350);
			ShowEffect(UID, 300391);
			end	 
		end
	end
end

if (EVENT == 202) then
	SelectMsg(UID, 4, 343, 1401, NPC, 22, 203, 23, -1);
end

if (EVENT == 203) then
	QuestStatus = GetQuestStatus(UID, 343)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 918);
	end
end

if (EVENT == 205) then
	QuestStatus = GetQuestStatus(UID, 343)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 343, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 343, 1401, NPC, 18, 207);
		else
			SaveEvent(UID, 920);
		end
	end
end

if (EVENT == 206) then
	QuestStatus = GetQuestStatus(UID, 343)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 343, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 343, 1401, NPC, 18, 207);
		else
			SelectMsg(UID, 4, 343, 1401, NPC, 41, 208, 27, -1);
		end
	end
end

if (EVENT == 207) then
	ShowMap(UID, 130);
end

if (EVENT == 208) then
	QuestStatus = GetQuestStatus(UID, 343)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 343, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 343, 1401, NPC, 18, 207);
		else
			RunQuestExchange(UID,158);
			SaveEvent(UID, 919);
		end
	end
end


if (EVENT == 212) then
	SelectMsg(UID, 4, 345, 1414, NPC, 22, 213, 23, -1);
end

if (EVENT == 213) then
	QuestStatus = GetQuestStatus(UID, 345)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 930);
	end
end

if (EVENT == 215) then
	QuestStatus = GetQuestStatus(UID, 345)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 345, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 345, 1414, NPC, 18, 217);
		else
			SaveEvent(UID, 932);
		end
	end
end

if (EVENT == 216) then
	QuestStatus = GetQuestStatus(UID, 345)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 345, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 345, 1414, NPC, 18, 217);
		else
			SelectMsg(UID, 4, 345, 1414, NPC, 41, 218, 27, -1);
		end
	end
end

if (EVENT == 217) then
	ShowMap(UID, 171);
end

if (EVENT == 218) then
	QuestStatus = GetQuestStatus(UID, 345)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 345, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 345, 1414, NPC, 18, 217);
		else
			RunQuestExchange(UID,160);
			SaveEvent(UID, 931);
		end
	end
end

if (EVENT == 222) then
	SelectMsg(UID, 4, 350, 1427, NPC, 22, 223, 23, -1);
end

if (EVENT == 223) then
	QuestStatus = GetQuestStatus(UID, 350)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 942);
	end
end

if (EVENT == 225) then
	QuestStatus = GetQuestStatus(UID, 350)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 350, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 350, 1427, NPC, 18, 227);
		else
			SaveEvent(UID, 944);
		end
	end
end

if (EVENT == 226) then
	QuestStatus = GetQuestStatus(UID, 350)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 350, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 350, 1427, NPC, 18, 227);
		else
			SelectMsg(UID, 4, 350, 1427, NPC, 41, 228, 27, -1);
		end
	end
end

if (EVENT == 227) then
	ShowMap(UID, 172);
end

if (EVENT == 228) then
	QuestStatus = GetQuestStatus(UID, 350)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 350, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 350, 1427, NPC, 18, 227);
		else
			RunQuestExchange(UID,162);
			SaveEvent(UID, 943);
		end
	end
end

if (EVENT == 232) then
	SelectMsg(UID, 4, 352, 1440, NPC, 22, 233, 23, -1);
end

if (EVENT == 233) then
	QuestStatus = GetQuestStatus(UID, 352)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 954);
	end
end

if (EVENT == 235) then
	QuestStatus = GetQuestStatus(UID, 352)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 352, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 352, 1440, NPC, 18, 237);
		else
			SaveEvent(UID, 956);
		end
	end
end

if (EVENT == 236) then
	QuestStatus = GetQuestStatus(UID, 352)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 352, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 352, 1440, NPC, 18, 237);
		else
			SelectMsg(UID, 4, 352, 1440, NPC, 41, 238, 27, -1);
		end
	end
end

if (EVENT == 237) then
	ShowMap(UID, 174);
end

if (EVENT == 238) then
	QuestStatus = GetQuestStatus(UID, 352)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 352, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 352, 1440, NPC, 18, 237);
		else
			RunQuestExchange(UID,164);
			SaveEvent(UID, 955);
		end
	end
end

if (EVENT == 242) then
	SelectMsg(UID, 4, 354, 1451, NPC, 22, 243, 23, -1);
end

if (EVENT == 243) then
	QuestStatus = GetQuestStatus(UID, 354)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 966);
	end
end

if (EVENT == 245) then
	QuestStatus = GetQuestStatus(UID, 354)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 354, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 354, 1451, NPC, 18, 247);
		else
			SaveEvent(UID, 968);
		end
	end
end

if (EVENT == 246) then
	QuestStatus = GetQuestStatus(UID, 354)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 354, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 354, 1451, NPC, 18, 247);
		else
			SelectMsg(UID, 4, 354, 1451, NPC, 41, 248, 27, -1);
		end
	end
end

if (EVENT == 247) then
	ShowMap(UID, 175);
end

if (EVENT == 248) then
	QuestStatus = GetQuestStatus(UID, 354)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 354, 1);
		if (MonsterCount < 80) then
			SelectMsg(UID, 2, 354, 1451, NPC, 18, 247);
		else
			RunQuestExchange(UID,166)
			SaveEvent(UID, 967);
		end
	end
end

if (EVENT == 1000) then 
	SaveEvent(UID, 11391);
end

if (EVENT == 1002) then
	SelectMsg(UID, 2, 546, 20389, NPC, 10, 1003);
	SaveEvent(UID, 11392);
end

if (EVENT == 1003) then
	SelectMsg(UID, 2, 546, 20390, NPC, 10, 1005);
	SaveEvent(UID, 11403);
end

if (EVENT == 1005) then
	SelectMsg(UID, 4, 546, 20391, NPC, 10, 1006,27,-1);
	SaveEvent(UID, 11394);
end

if (EVENT == 1006) then
	SelectMsg(UID, 2, 546, 20392, NPC, 10, 1008);
	SaveEvent(UID, 11393);
	SaveEvent(UID, 11404);
end

if (EVENT == 1008) then
	QuestStatus = GetQuestStatus(UID, 546)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			RunQuestExchange(UID,3033);
			SaveEvent(UID, 11393);
	end
end

if (EVENT == 1105) then
	CHERICHERO = HowmuchItem(UID, 910229000);
	if (CHERICHERO < 1) then
		SelectMsg(UID, 2, 550, 21624, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 550, 20066, NPC, 10, 1109, 27, -1);
	end
end


if (EVENT == 1109) then
	RELICHERO = HowmuchItem(UID, 910229000);
	if (RELICHERO < 1) then
		SelectMsg(UID, 2, 550, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3040);
	SaveEvent(UID, 11471);
	SaveEvent(UID, 11482);
	SaveEvent(UID, 11484);
    end
end

if (EVENT == 1203) then
	SelectMsg(UID, 2, 552, 20464, NPC, 10, 1205);
end

if (EVENT == 1205) then
	SelectMsg(UID, 4, 552, 20390, NPC, 10, 1202, 27, -1);
	SaveEvent(UID, 11502);
end

if (EVENT == 1202) then
	SelectMsg(UID, 2, 552, 20465, NPC, 10, -1);
	SaveEvent(UID, 11506);
	SaveEvent(UID, 11495);
end

if (EVENT == 1300) then --test
	SaveEvent(UID, 11510);
end

if (EVENT == 1302) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 553, 20465, NPC, 22, 1303, 23, -1);
	else
		SelectMsg(UID, 2, 553, 20465, NPC, 10, -1);
	end
end

if (EVENT == 1303) then
	SaveEvent(UID, 11506);
end


if (EVENT == 1305) then
	MonsterCount01 = CountMonsterQuestSub(UID, 553, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 553, 2);
		SlotCheck = CheckGiveSlot(UID, 3)
	if SlotCheck == false then
	elseif(MonsterCount01 > 49 and MonsterCount02 > 49 ) then
		SelectMsg(UID, 5, 553, 20465, NPC, 41, 1307, 27, -1);
	else
	if (MonsterCount01 > 49) then
		SelectMsg(UID, 2, 553, 20465, NPC, 18, 1311);
	elseif ( MonsterCount02 > 49) then
		SelectMsg(UID, 2, 553, 20465, NPC, 18, 1312);
		end
	end
end

if (EVENT == 1311) then
	ShowMap(UID, 553);
end

if (EVENT == 1312) then
	ShowMap(UID, 518);
end

if (EVENT == 1306) then
	SaveEvent(UID, 11508);
end

if (EVENT == 1307)then
	MonsterCount01 = CountMonsterQuestSub(UID, 553, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 553, 2);
	if(MonsterCount01 > 49 and MonsterCount01 > 49) then
	RunQuestExchange(UID,3043)
	SelectMsg(UID, 2, -1, 20479, NPC, 10, -1);
	SaveEvent(UID,11507)
	SaveEvent(UID,11518)
	end
end

savenum				=1066
talknum				=11386
exchangeid			=1376
moncount			=25 	  
accept				=4501 
iscomplate			=4503 
complate			=4502 
event1				=2412 
event2				=2413 
event3				=2415 
event4				=2416 
event5				=2417 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1066)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount < moncount) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount >= moncount) then
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID, complate)
			end
		end
	end
end

savenum				=1070
talknum				=11392
exchangeid			=1378
moncount			=25 	  
accept				=4521 
iscomplate			=4523 
complate			=4522 
event1				=2432 
event2				=2433 
event3				=2435 
event4				=2436 
event5				=2437 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1070)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount < moncount) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount >= moncount) then
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID, complate)
			end
		end
	end
end


savenum				=1074
talknum				=11396
exchangeid			=1380
moncount			=30 	  
accept				=4541 
iscomplate			=4543 
complate			=4542 
event1				=2452 
event2				=2453 
event3				=2455 
event4				=2456 
event5				=2457 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1074)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount < moncount) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount >= moncount) then
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID, complate)
			end
		end
	end
end

savenum				=1078
talknum				=11402
exchangeid			=1382
moncount			=30 	  
accept				=4561 
iscomplate			=4563 
complate			=4562 
event1				=2472 
event2				=2473 
event3				=2475 
event4				=2476 
event5				=2477 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1078)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount < moncount) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount >= moncount) then
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID, complate)
			end
		end
	end
end

savenum				=1082
talknum				=11406
exchangeid			=1384
moncount			=30 	  
accept				=4581 
iscomplate			=4583 
complate			=4582 
event1				=2492 
event2				=2493 
event3				=2495 
event4				=2496 
event5				=2497 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1082)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount < moncount) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount >= moncount) then
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID, complate)
			end
		end
	end
end

savenum				=1086
talknum				=11410
exchangeid			=1386
moncount			=30 	  
accept				=4601 
iscomplate			=4603 
complate			=4602 
event1				=2512 
event2				=2513 
event3				=2515 
event4				=2516 
event5				=2517 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1086)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount < moncount) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount >= moncount) then
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID, complate)
			end
		end
	end
end

local savenum=1090
local talknum=11414
local exchangeid=1388

if (EVENT == 2532) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2533, 23, -1);
end
	
if (EVENT == 2533) then
	QuestStatus = GetQuestStatus(UID, 1090)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4621);
	end
end

if (EVENT == 2535) then
	QuestStatus = GetQuestStatus(UID, 1090)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4623);
	end
end

if (EVENT == 2536) then
	QuestStatus = GetQuestStatus(UID, 1090)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810294000);
		if(ITEMA > 9) then
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2534, 27, -1);
		else
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
		end
	end
end

if (EVENT == 2534)then
	QuestStatus = GetQuestStatus(UID, 1090)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810294000);
		if(ITEMA < 10) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID, 4622);
		end
	end
end

local savenum=1092
local talknum=11417
local exchangeid=1389

if (EVENT == 2542) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2543, 23, -1);
end
	
if (EVENT == 2543) then
	QuestStatus = GetQuestStatus(UID, 1092)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4631);
	end
end

if (EVENT == 2545) then
	QuestStatus = GetQuestStatus(UID, 1092)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4633);
	end
end

if (EVENT == 2546) then
	QuestStatus = GetQuestStatus(UID, 1092)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810295000);
		if(ITEMA < 10) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2544, 27, -1);
		end
	end
end

if (EVENT == 2544)then
	QuestStatus = GetQuestStatus(UID, 1092)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810295000);
		if(ITEMA < 10) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4632);
		end
	end
end

local savenum=1094
local talknum=11421
local exchangeid=1390

if (EVENT == 2552) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2553, 23, -1);
end
	
if (EVENT == 2553) then
	QuestStatus = GetQuestStatus(UID, 1094)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4641);
	end
end

if (EVENT == 2555) then
	QuestStatus = GetQuestStatus(UID, 1094)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810296000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4643);
		end
	end	
end

if (EVENT == 2556) then
	QuestStatus = GetQuestStatus(UID, 1094)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810296000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2554, 27, -1);
		end
	end
end

if (EVENT == 2554)then
	QuestStatus = GetQuestStatus(UID, 1094)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810296000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4642);
		end
	end
end

local savenum=1096
local talknum=11425
local exchangeid=1391

if (EVENT == 2562) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2563, 23, -1);
end
	
if (EVENT == 2563) then
	QuestStatus = GetQuestStatus(UID, 1096)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4651);
	end
end

if (EVENT == 2565) then
	QuestStatus = GetQuestStatus(UID, 1096)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4653);
	end	
end

if (EVENT == 2566) then
	QuestStatus = GetQuestStatus(UID, 1096)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810297000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2564, 27, -1);
		end
	end
end

if (EVENT == 2564)then
	QuestStatus = GetQuestStatus(UID, 1096)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810297000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4652);
		end
	end
end

local savenum=1098
local talknum=11429
local exchangeid=1392

if (EVENT == 2572) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2573, 23, -1);
end
	
if (EVENT == 2573) then
	QuestStatus = GetQuestStatus(UID, 1098)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4661);
	end
end

if (EVENT == 2575) then
	QuestStatus = GetQuestStatus(UID, 1098)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810298000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4663);
		end
	end	
end

if (EVENT == 2576) then
	QuestStatus = GetQuestStatus(UID, 1098)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810298000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2574, 27, -1);
		end
	end
end

if (EVENT == 2574)then
	QuestStatus = GetQuestStatus(UID, 1098)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810298000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid);
			SaveEvent(UID,4662);
		end
	end
end

local savenum=1100
local talknum=11433
local exchangeid=1393

if (EVENT == 2582) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2583, 23, -1);
end
	
if (EVENT == 2583) then
	QuestStatus = GetQuestStatus(UID, 1100)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4671);
	end
end

if (EVENT == 2585) then
	QuestStatus = GetQuestStatus(UID, 1100)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4673);
	end	
end

if (EVENT == 2586) then
	QuestStatus = GetQuestStatus(UID, 1100)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810299000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2584, 27, -1);
		end
	end
end

if (EVENT == 2584)then
	QuestStatus = GetQuestStatus(UID, 1100)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810299000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4672);
		end
	end
end

local savenum=1102
local talknum=11437
local exchangeid=1394

if (EVENT == 2592) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2593, 23, -1);
end
	
if (EVENT == 2593) then
	QuestStatus = GetQuestStatus(UID, 1102)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4681);
	end
end

if (EVENT == 2595) then
	QuestStatus = GetQuestStatus(UID, 1102)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810301000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4683);
		end
	end	
end

if (EVENT == 2596) then
	QuestStatus = GetQuestStatus(UID, 1102)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810301000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2594, 27, -1);
		end
	end
end

if (EVENT == 2594)then
	QuestStatus = GetQuestStatus(UID, 1102)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810301000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4682);
		end
	end
end

local savenum=1104
local talknum=11441
local exchangeid=1395

if (EVENT == 2602) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2603, 23, -1);
end
	
if (EVENT == 2603) then
	QuestStatus = GetQuestStatus(UID, 1104)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4691);
	end
end

if (EVENT == 2605) then
	QuestStatus = GetQuestStatus(UID, 1104)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810302000);
		if(ITEMA < 1) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4693);
		end
	end	
end

if (EVENT == 2606) then
	QuestStatus = GetQuestStatus(UID, 1104)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810302000);
		if(ITEMA < 1) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2604, 27, -1);
		end
	end
end

if (EVENT == 2604)then
	QuestStatus = GetQuestStatus(UID, 1104)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810302000);
		if(ITEMA < 1) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4692);
		end
	end
end

local savenum=1106
local talknum=11444
local exchangeid=1396

if (EVENT == 2612) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2613, 23, -1);
end
	
if (EVENT == 2613) then
	QuestStatus = GetQuestStatus(UID, 1106)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4701);
	end
end

if (EVENT == 2615) then
	QuestStatus = GetQuestStatus(UID, 1106)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810303000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4703);
		end
	end	
end

if (EVENT == 2616) then
	QuestStatus = GetQuestStatus(UID, 1106)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810303000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2614, 27, -1);
		end
	end
end

if (EVENT == 2614)then
	QuestStatus = GetQuestStatus(UID, 1106)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810303000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4702);
		end
	end
end

local savenum=1034
local talknum=11339
local exchangeid=1360

if (EVENT == 2252) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2253, 23, -1);
end
	
if (EVENT == 2253) then
	QuestStatus = GetQuestStatus(UID, 1034)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 5833);
	end
end

if (EVENT == 2255) then
	QuestStatus = GetQuestStatus(UID, 1034)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 5835);
		end
	end	
end

if (EVENT == 2256) then
	QuestStatus = GetQuestStatus(UID, 1034)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2254, 27, -1);
		end
	end
end

if (EVENT == 2254)then
	QuestStatus = GetQuestStatus(UID, 1034)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,5834);
		end
	end
end

local savenum=1038
local talknum=11345
local exchangeid=1362

if (EVENT == 2272) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2273, 23, -1);
end
	
if (EVENT == 2273) then
	QuestStatus = GetQuestStatus(UID, 1038)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 5853);
	end
end

if (EVENT == 2275) then
	QuestStatus = GetQuestStatus(UID, 1038)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 5855);
		end
	end	
end

if (EVENT == 2276) then
	QuestStatus = GetQuestStatus(UID, 1038)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2274, 27, -1);
		end
	end
end

if (EVENT == 2274)then
	QuestStatus = GetQuestStatus(UID, 1038)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,5854);
		end
	end
end

local savenum			=1042 			-- Event kayıt numarası
local talknum			=11351 			-- mesaj 
local moncount			=20 				
local exchangeid		=1364 			-- Hediye
local accept			=5873 			
local iscomplate		=5875 			
local complate			=5874 			
local event1			=2292 			
local event2			=2293 			
local event3			=2295 			
local event4			=2296 			
local event5			=2298 			

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1042)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount < moncount) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
		if (MonsterCount >= moncount) then
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID, complate)
			end
		end
	end
end

local savenum=1046
local talknum=11357
local moncount=40
local exchangeid=1366
local accept=5893
local iscomplate=5895
local complate=5894
local event1=2312 
local event2=2313 
local event3=2315 
local event4=2316 
local event5=2318 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1046)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
			end
		end
	end
end

local savenum				=1050
local talknum				=11364
local exchangeid			=1368
local moncount				=40 	  
local accept				=5913 
local iscomplate			=5915 
local complate				=5914 
local event1				=2332 
local event2				=2333 
local event3				=2335 
local event4				=2336 
local event5				=2337 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1050)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
			end
		end
	end
end

local savenum				=1054
local talknum				=11371
local exchangeid			=1370
local moncount				=40 	  
local accept				=5933 
local iscomplate			=5935 
local complate				=5934 
local event1				=2352 
local event2				=2353 
local event3				=2355 
local event4				=2356 
local event5				=2357 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1054)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
			end
		end
	end
end

savenum				=1058
talknum				=11378
exchangeid			=1372
moncount			=40 	  
accept				=5953 
iscomplate			=5955 
complate			=5954 
event1				=2372 
event2				=2373 
event3				=2375 
event4				=2376 
event5				=2377 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1058)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
			end
		end
	end
end

savenum				=1062
talknum				=11382
exchangeid			=1374
moncount			=25 	  
accept				=5973 
iscomplate			=5975 
complate			=5974 
event1				=2392 
event2				=2393 
event3				=2395 
event4				=2396 
event5				=2397 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1062)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
		end
	end
end
end

savenum				=1068
talknum				=11390
exchangeid			=1377
moncount			=25 	  
accept				=4511 
iscomplate			=4513 
complate			=4512 
event1				=2422 
event2				=2423 
event3				=2425 
event4				=2426 
event5				=2427 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1068)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
		end
	end
end
end

savenum				=1072
talknum				=11394
exchangeid			=1379
moncount			=25 	  
accept				=4531 
iscomplate			=4533 
complate			=4532 
event1				=2442 
event2				=2443 
event3				=2445 
event4				=2446 
event5				=2447 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1072)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
		end
	end
end
end

savenum				=1076
talknum				=11399
exchangeid			=1381
moncount			=30 	  
accept				=4551 
iscomplate			=4553 
complate			=4552 
event1				=2462 
event2				=2463 
event3				=2465 
event4				=2466 
event5				=2467 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1076)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
		end
	end
	end
end

local savenum=1080
local talknum=11404
local exchangeid=1383

if (EVENT == 2482) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2483, 23, -1);
end
	
if (EVENT == 2483) then
	QuestStatus = GetQuestStatus(UID, 1080)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4571);
	end
end

if (EVENT == 2485) then
	QuestStatus = GetQuestStatus(UID, 1080)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4573);
		end
	end	
end

if (EVENT == 2486) then
	QuestStatus = GetQuestStatus(UID, 1080)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2484, 27, -1);
		end
	end
end

if (EVENT == 2484)then
	QuestStatus = GetQuestStatus(UID, 1080)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4572);
		end
	end
end

local savenum=1084
local talknum=11408
local exchangeid=1385

if (EVENT == 2502) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2503, 23, -1);
end
	
if (EVENT == 2503) then
	QuestStatus = GetQuestStatus(UID, 1084)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4591);
	end
end

if (EVENT == 2505) then
	QuestStatus = GetQuestStatus(UID, 1084)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4593);
		end
	end	
end

if (EVENT == 2506) then
	QuestStatus = GetQuestStatus(UID, 1084)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2504, 27, -1);
		end
	end
end

if (EVENT == 2504)then
	QuestStatus = GetQuestStatus(UID, 1084)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4592);
		end
	end
end

local savenum=1088
local talknum=11412
local exchangeid=1387

if (EVENT == 2522) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2523, 23, -1);
end
	
if (EVENT == 2523) then
	QuestStatus = GetQuestStatus(UID, 1088)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4611);
	end
end

if (EVENT == 2525) then
	QuestStatus = GetQuestStatus(UID, 1088)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4613);
		end
	end	
end

if (EVENT == 2526) then
	QuestStatus = GetQuestStatus(UID, 1088)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2524, 27, -1);
		end
	end
end

if (EVENT == 2524)then
	QuestStatus = GetQuestStatus(UID, 1088)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, savenum, 1);
		if(MonsterCount01 < 30) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4612);
		end
	end
end

local savenum=1036
local talknum=11343
local exchangeid=1361
local moncount=20 
local accept=5843 
local iscomplate=5845 
local complate=5844 
local event1=2262 
local event2=2263 
local event3=2265 
local event4=2266 
local event5=2267 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1036)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
		end
	end
end
end

local savenum=1108
local talknum=11447
local exchangeid=1397

if (EVENT == 2622) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 2623, 23, -1);
end
	
if (EVENT == 2623) then
	QuestStatus = GetQuestStatus(UID, 1108)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4711);
	end
end

if (EVENT == 2625) then
	QuestStatus = GetQuestStatus(UID, 1108)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810304000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SaveEvent(UID, 4713);
		end
	end	
end

if (EVENT == 2626) then
	QuestStatus = GetQuestStatus(UID, 1108)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810304000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			SelectMsg(UID, 4, savenum, talknum, NPC, 41, 2624, 27, -1);
		end
	end
end

if (EVENT == 2624)then
	QuestStatus = GetQuestStatus(UID, 1108)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA  = HowmuchItem(UID, 810304000);
		if(ITEMA < 20) then
			SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
		else
			RunQuestExchange(UID, exchangeid)
			SaveEvent(UID,4712);
		end
	end
end

local savenum=1007
local talknum=8772
local exchangeid=1300
local moncount=20 
local accept=5702 
local iscomplate=5704 
local complate=5703 
local event1=8542 
local event2=8543 
local event3=8546 
local event4=8547 
local event5=8548 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1007)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
		end
	end
end
end

local savenum				=1009
local talknum				=8686
local exchangeid			=1302
local moncount				=40 	  
local accept				=5714 
local iscomplate			=5716 
local complate				=5715 
local event1				=8372 
local event2				=8373 
local event3				=8376 
local event4				=8377 
local event5				=8378 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1009)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

savenum				=1011
talknum				=8688
exchangeid			=1304
moncount			=40	  
accept				=5726 
iscomplate			=5728 
complate			=5727 
event1				=8382 
event2				=8383 
event3				=8386 
event4				=8387 
event5				=8388 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1011)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
	end
	end
end

savenum				=1013
talknum				=8690
exchangeid			=1306
moncount			=7 	  
accept				=5738 
iscomplate			=5740 
complate			=5739 
event1				=8392 
event2				=8393 
event3				=8396 
event4				=8397 
event5				=8398 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1013)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

savenum				=1015
talknum				=8694
exchangeid			=1308
moncount			=40 	  
accept				=5750 
iscomplate			=5752 
complate			=5751 
event1				=8412 
event2				=8413 
event3				=8416 
event4				=8417 
event5				=8418 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1015)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

local savenum=1040
local talknum=11349
local exchangeid=1363
local moncount=20 
local accept=5863 
local iscomplate=5865 
local complate=5864 
local event1=2282 
local event2=2283 
local event3=2285 
local event4=2286 
local event5=2287 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1040)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

local savenum=1044
local talknum=11355
local exchangeid=1365
local moncount=20 
local accept=5883 
local iscomplate=5885 
local complate=5884 
local event1=2302 
local event2=2303 
local event3=2305 
local event4=2306 
local event5=2307 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1044)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

local savenum				=1048
local talknum				=11360
local exchangeid			=1367
local moncount				=40 	  
local accept				=5903 
local iscomplate			=5905 
local complate				=5904 
local event1				=2322 
local event2				=2323 
local event3				=2325 
local event4				=2326 
local event5				=2327 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1048)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

savenum				=1052
talknum				=11368
exchangeid			=1369
moncount			=40 	  
accept				=5923 
iscomplate			=5925 
complate			=5924 
event1				=2342 
event2				=2343 
event3				=2345 
event4				=2346 
event5				=2347 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1052)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

savenum				=1056
talknum				=11375
exchangeid			=1371
moncount			=40 	  
accept				=5943 
iscomplate			=5945 
complate			=5944 
event1				=2362 
event2				=2363 
event3				=2365 
event4				=2366 
event5				=2367 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1056)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

savenum				=1060
talknum				=11380
exchangeid			=1373
moncount			=40	  
accept				=5963 
iscomplate			=5965 
complate			=5964 
event1				=2382 
event2				=2383 
event3				=2385 
event4				=2386 
event5				=2387 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1060)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end

savenum				=1064
talknum				=11384
exchangeid			=1375
moncount			=25 	  
accept				=5983 
iscomplate			=5985 
complate			=5984 
event1				=2402 
event2				=2403 
event3				=2405 
event4				=2406 
event5				=2407 

if(EVENT == event1) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, event2, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end

if(EVENT == event2) then
	SaveEvent(UID, accept)
end

if(EVENT == event3) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount ==	moncount) then
		SaveEvent(UID, iscomplate)
	end
end

if(EVENT == event4) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, event5, 27, -1);
	end
end

if(EVENT == event5) then
	QuestStatus = GetQuestStatus(UID, 1064)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount < moncount) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount >= moncount) then
		RunQuestExchange(UID, exchangeid)
		SaveEvent(UID, complate)
	end
end
end
end