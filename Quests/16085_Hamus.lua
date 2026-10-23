local NPC = 16085;
----------------------------------------------------------------------------------------------------------------------

if (EVENT == 165) then
	NpcMsg(UID, 4131, NPC)
end

if (EVENT == 520) then
	SelectMsg(UID, 3, -1, 1111, NPC, 4583, 521, 4584, 522, 7162, 525, 4585, 523, 4586, 524, 7163, 526, 45486, 2850); --535 (the client window shows at most 7 buttons: Job Change / Nation Transfer live in a sub menu)
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 521) then
	HP100 = HowmuchItem(UID, 889310000);
	if (HP100 < 1) then
		SelectMsg(UID, 2, -1, 1113, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then

		else
		RunGiveItemExchange(UID,657, 1);
		end
	end
end

if (EVENT == 522) then
	HP300 = HowmuchItem(UID, 889320000);
	if (HP300 < 1) then
		SelectMsg(UID, 2, -1, 1116, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,658, 1);
		end
	end
end

if (EVENT == 523) then
	MP100 = HowmuchItem(UID, 889340000);
	if (MP100 < 1) then
		SelectMsg(UID, 2, -1, 1118, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,659, 1);
		end
	end
end

if (EVENT == 524) then
	MP300 = HowmuchItem(UID, 889350000);
	if (MP300 < 1) then
		SelectMsg(UID, 2, -1, 1120, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,660, 1);
		end
	end
end

if (EVENT == 525) then
	HP500 = HowmuchItem(UID, 889330000);
	if (HP500 < 1) then
		SelectMsg(UID, 2, -1, 1669, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,661, 1);
		end
	end
end

if (EVENT == 526) then
	MP500 = HowmuchItem(UID, 889360000);
	if (MP500 < 1) then
		SelectMsg(UID, 2, -1, 1671, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,662, 1);
		end
	end
end

if (EVENT == 527) then
	HP100 = HowmuchItem(UID, 889310000);
	if (HP100 < 1) then
		SelectMsg(UID, 2, -1, 1113, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
		else
		RunGiveItemExchange(UID,657, 1);
		end
	end
end

if (EVENT == 528) then
	HP300 = HowmuchItem(UID, 889320000);
	if (HP300 < 1) then
		SelectMsg(UID, 2, -1, 1116, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,658, 1);
		end
	end
end

if (EVENT == 529) then
	MP100 = HowmuchItem(UID, 889340000);
	if (MP100 < 1) then
		SelectMsg(UID, 2, -1, 1118, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,659, 1);
		end
	end
end

if (EVENT == 530) then
	MP300 = HowmuchItem(UID, 889350000);
	if (MP300 < 1) then
		SelectMsg(UID, 2, -1, 1120, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,660, 1);
		end
	end
end

if (EVENT == 531) then
	HP500 = HowmuchItem(UID, 889330000);
	if (HP500 < 1) then
		SelectMsg(UID, 2, -1, 1669, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then 
		else
		RunGiveItemExchange(UID,661, 1);
		end
	end
end

if (EVENT == 532) then
	MP500 = HowmuchItem(UID, 889360000);
	if (MP500 < 1) then
		SelectMsg(UID, 2, -1, 1671, NPC, 18, 5000);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
		RunGiveItemExchange(UID,662, 1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 200) then
	NCS = HowmuchItem(UID, 800032000);
	if (NCS < 1) then
		SelectMsg(UID, 2, -1, 4454, NPC, 18, 5000);
	else
		SendNameChange(UID);
	end
end

if (EVENT == 300) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST > 0) then
		SelectMsg(UID, 2, -1, 4456, NPC, 4189, 301, 4190, 302);
	else
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	end
end

if (EVENT == 301) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 4456, NPC, 3000, 303, 3005, -1);
	end
end

if (EVENT == 302) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 4456, NPC, 3000, 304, 3005, -1);
	end
end


if (EVENT == 303) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		ResetSkillPoints(UID);
	end
end

if (EVENT == 304) then
	REDIST = HowmuchItem(UID, 700001000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		ResetStatPoints(UID);
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 400) then
	SelectMsg(UID, 2,-1, 4457, NPC, 10, -1);
end

if (EVENT == 500) then
	SelectMsg(UID, 2, -1, 4462, NPC, 10, -1);
end

if (EVENT == 600) then
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

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 700) then
	SelectMsg(UID, 2, -1, 4456, NPC, 10, -1);
end

if (EVENT == 701) then
	REDIST = HowmuchItem(UID, 700008000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		ResetSkillPoints(UID);
		RobItem(UID, 700008000, 1);
	end
end

if (EVENT == 702) then
	REDIST = HowmuchItem(UID, 700008000);
	if (REDIST < 1) then 
		SelectMsg(UID, 2, -1, 4455, NPC, 18, 5000);
	else
		ResetStatPoints(UID);
		RobItem(UID, 700008000, 1);
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 850) then
	SelectMsg(UID, 3, -1, 8973, NPC, 7040, 1007,4504,794,7055, 720,4296,165);
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 721) then
	SelectMsg(UID, 11, -1, 1155, NPC);
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 720) then
	SelectMsg(UID, 3, -1, 4914, NPC, 4285, 721, 4287, 723, 753, 722, 4421, 725, 4420, 724, 4588, 727, 4589, 726, 7243, 729, 7244, 728, 4296, 850);
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1007) then
	SelectMsg(UID, 3, -1, 4901, NPC, 4285, 721, 4287, 1010, 753, 1008, 4421, 1014, 4420, 1012, 4588, 1018, 4589, 1016, 7243, 1022, 7244, 1020, 4296, 850);
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1008) then
	VALARMOR = HowmuchItem(UID, 800180000);
	if (VALARMOR < 1 or VALARMOR == 0) then
		SelectMsg(UID, 2, -1, 4921, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6910,STEP,1);
		end
    end
end

if (EVENT == 1009) then
	VALARMOR = HowmuchItem(UID, 800180000);
	if (VALARMOR < 1 or VALARMOR == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6910,STEP,1);
		end
    end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1010) then
	VALHELMET = HowmuchItem(UID, 800170000);
	if (VALHELMET < 1 or VALHELMET == 0) then
		SelectMsg(UID, 2, -1, 4921, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6882,STEP,1);
		end
    end
end

if (EVENT == 1011) then
	VALHELMET = HowmuchItem(UID, 800170000);
	if (VALHELMET < 1 or VALHELMET == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6882,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1012) then
	GRYPARMOR = HowmuchItem(UID, 800240000);
	if (GRYPARMOR < 1 or GRYPARMOR == 0) then
		SelectMsg(UID, 2, -1, 6488, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6883,STEP,1);
		end
    end
end

if (EVENT == 1013) then
	GRYPARMOR = HowmuchItem(UID, 800240000);
	if (GRYPARMOR < 1 or GRYPARMOR == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6883,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1014) then
	GRYPHELMET = HowmuchItem(UID, 800230000);
	if (GRYPHELMET < 1 or GRYPHELMET == 0) then
		SelectMsg(UID, 2, -1, 6488, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6884,STEP,1);
		end
    end
end

if (EVENT == 1015) then
	GRYPHELMET = HowmuchItem(UID, 800230000);
	if (GRYPHELMET < 1 or GRYPHELMET == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6884,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1016) then
	BAHAMUTARMOR = HowmuchItem(UID, 800270000);
	if (BAHAMUTARMOR < 1 or BAHAMUTARMOR == 0) then
		SelectMsg(UID, 2, -1, 1126, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6885,STEP,1);
		end
    end
end

if (EVENT == 1017) then
	BAHAMUTARMOR = HowmuchItem(UID, 800270000);
	if (BAHAMUTARMOR < 1 or BAHAMUTARMOR == 0) then
		SelectMsg(UID, 2, -1, 1126, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6885,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1018) then
	BAHAMUTHELMET = HowmuchItem(UID, 800260000);
	if (BAHAMUTHELMET < 1 or BAHAMUTHELMET == 0) then
		SelectMsg(UID, 2, -1, 1126, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6886,STEP,1);
		end
    end
end

if (EVENT == 1019) then
	BAHAMUTHELMET = HowmuchItem(UID, 800260000);
	if (BAHAMUTHELMET < 1 or BAHAMUTHELMET == 0) then
		SelectMsg(UID, 2, -1, 1126, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6886,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1020) then
	YENICERIA = HowmuchItem(UID, 508117000);
	if (YENICERIA < 1 or YENICERIA == 0) then
		SelectMsg(UID, 2, -1, 1116, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6887,STEP,1);
		end
    end
end

if (EVENT == 1021) then
	YENICERIA = HowmuchItem(UID, 508117000);
	if (YENICERIA < 1 or YENICERIA == 0) then
		SelectMsg(UID, 2, -1, 1116, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6887,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 1022) then
	YENICERIH = HowmuchItem(UID, 508116000);
	if (YENICERIH < 1 or YENICERIH == 0) then
		SelectMsg(UID, 2, -1, 1113, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6888,STEP,1);
		end
    end
end

if (EVENT == 1023) then
	YENICERIH = HowmuchItem(UID, 508116000);
	if (YENICERIH < 1 or YENICERIH == 0) then
		SelectMsg(UID, 2, -1, 1113, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6888,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 794) then
	SelectMsg(UID, 2, -1, 748, NPC, 4509,880,4510,882);
end

if (EVENT == 880) then
	PATHOSGLOVEAT = HowmuchItem(UID, 800250000);
	if (PATHOSGLOVEAT < 1 or PATHOSGLOVEAT == 0) then
		SelectMsg(UID, 2, -1, 4921, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6914,STEP,1);
		end
    end
end

if (EVENT == 881) then
	PATHOSGLOVEAT = HowmuchItem(UID, 800250000);
	if (PATHOSGLOVEAT < 1 or PATHOSGLOVEAT == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6914,STEP,1); 
		end
    end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 882) then
	PATHOSGLOVEDEF = HowmuchItem(UID, 800250000);
	if (PATHOSGLOVEDEF < 1 or PATHOSGLOVEDEF == 0) then
		SelectMsg(UID, 2, -1, 4921, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6915,STEP,1);
		end
    end
end

if (EVENT == 883) then
	PATHOSGLOVEDEF = HowmuchItem(UID, 800250000);
	if (PATHOSGLOVEDEF < 1 or PATHOSGLOVEDEF == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6915,STEP,1); 
    end
    end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 722) then
	MINERVAVALARM = HowmuchItem(UID, 508057000);
	if (MINERVAVALARM < 1 or MINERVAVALARM == 0) then
		SelectMsg(UID, 2, -1, 4921, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6916,STEP,1);
		end
    end
end

if (EVENT == 745) then
	MINERVAVALARM = HowmuchItem(UID, 508057000);
	if (MINERVAVALARM < 1 or MINERVAVALARM == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6916,STEP,1);
		end
    end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 723) then
	MINERVAVALHEL = HowmuchItem(UID, 508056000);
	if (MINERVAVALHEL < 1 or MINERVAVALHEL == 0) then
		SelectMsg(UID, 2, -1, 4921, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6917,STEP,1);
		end
    end
end

if (EVENT == 746) then
	MINERVAVALHEL = HowmuchItem(UID, 508056000);
	if (MINERVAVALHEL < 1 or MINERVAVALHEL == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6917,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 724) then
	MINERVAGRYPARM = HowmuchItem(UID, 508057000);
	if (MINERVAGRYPARM < 1 or MINERVAGRYPARM == 0) then
		SelectMsg(UID, 2, -1, 6488, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6918,STEP,1);
		end
    end
end

if (EVENT == 747) then
	MINERVAGRYPARM = HowmuchItem(UID, 508057000);
	if (MINERVAGRYPARM < 1 or MINERVAGRYPARM == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6918,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 725) then
	MINERVAGRYPHEL = HowmuchItem(UID, 508056000);
	if (MINERVAGRYPHEL < 1 or MINERVAGRYPHEL == 0) then
		SelectMsg(UID, 2, -1, 6488, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6919,STEP,1);
		end
    end
end

if (EVENT == 748) then
	MINERVAGRYPHEL = HowmuchItem(UID, 508056000);
	if (MINERVAGRYPHEL < 1 or MINERVAGRYPHEL == 0) then
		SelectMsg(UID, 2, -1, 4902, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6919,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 726) then
	MINERVABAHARM = HowmuchItem(UID, 508057000);
	if (MINERVABAHARM < 1 or MINERVABAHARM == 0) then
		SelectMsg(UID, 2, -1, 1126, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6920,STEP,1);
		end
    end
end

if (EVENT == 749) then
	MINERVABAHARM = HowmuchItem(UID, 508057000);
	if (MINERVABAHARM < 1 or MINERVABAHARM == 0) then
		SelectMsg(UID, 2, -1, 1126, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6920,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 727) then
	MINERVABAHHEL = HowmuchItem(UID, 508056000);
	if (MINERVABAHHEL < 1 or MINERVABAHHEL == 0) then
		SelectMsg(UID, 2, -1, 1126, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6921,STEP,1);
		end
    end
end

if (EVENT == 750) then
	MINERVABAHHEL = HowmuchItem(UID, 508056000);
	if (MINERVABAHHEL < 1 or MINERVABAHHEL == 0) then
		SelectMsg(UID, 2, -1, 1126, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6921,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 728) then
	MINERVAYENAR = HowmuchItem(UID, 508057000);
	if (MINERVAYENAR < 1 or MINERVAYENAR == 0) then
		SelectMsg(UID, 2, -1, 1116, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6922,STEP,1);
		end
    end
end

if (EVENT == 751) then
	MINERVAYENAR = HowmuchItem(UID, 508057000);
	if (MINERVAYENAR < 1 or MINERVAYENAR == 0) then
		SelectMsg(UID, 2, -1, 1116, NPC, 18, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6922,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 729) then
	MINERVAYENHEL = HowmuchItem(UID, 508056000);
	if (MINERVAYENHEL < 1 or MINERVAYENHEL == 0) then
		SelectMsg(UID, 2, -1, 1113, NPC, 10, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then

         else
	RunQuestExchange(UID,6923,STEP,1);
		end
    end
end

if (EVENT == 752) then
	MINERVAYENHEL = HowmuchItem(UID, 508056000);
	if (MINERVAYENHEL < 1 or MINERVAYENHEL == 0) then
		SelectMsg(UID, 2, -1, 1113, NPC, 18, 5000);
	else
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	RunQuestExchange(UID,6923,STEP,1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 650) then
	EVENTKARIBEDIS = HowmuchItem(UID, 810191000);
	if (EVENTKARIBEDIS > 0) then
		SelectMsg(UID, 3, -1, 106, NPC, 4403, 651, 4404, 652, 4479, 653);
	else
		SelectMsg(UID, 2, -1, 106, NPC, 10, -1);
	end
end

if (EVENT == 651) then
	SelectMsg(UID, 2, -1, 938, NPC, 4230, 654, 3005, -1);
end

if (EVENT == 652) then
	SelectMsg(UID, 2, -1, 753, NPC, 4230, 655, 3005, -1);
end

if (EVENT == 653) then
	SelectMsg(UID, 2, -1, 926, NPC, 4230, 656, 3005, -1);
end

if (EVENT == 654) then
	EVENTKARIBEDIS = HowmuchItem(UID, 810191000);
	if (EVENTKARIBEDIS < 1) then
		SelectMsg(UID, 2, -1, 106, NPC, 10, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810191000, 1);
			GiveItem(UID, 700002000, 1);
			SelectMsg(UID, 2, -1 , 1114, NPC, 27, -1);
		end
	end
end
if (EVENT == 655) then
	EVENTKARIBEDIS = HowmuchItem(UID, 810191000);
	if (EVENTKARIBEDIS == 0) then
		SelectMsg(UID, 2, -1, 106, NPC, 10, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 4)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810191000,1);
			GiveItem(UID, 379258000,1);
			GiveItem(UID, 379258000,1);
			GiveItem(UID, 379258000,1);
			GiveItem(UID, 379258000,1);
			SelectMsg(UID, 2, -1 , 1114, NPC, 27, -1);
		end
	end
end

if (EVENT == 656) then
	EVENTKARIBEDIS = HowmuchItem(UID, 810191000);
	if (EVENTKARIBEDIS == 0) then
		SelectMsg(UID, 2, -1, 106, NPC, 10, -1);
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810191000,1);
			GiveItem(UID, 700001000,1);
			SelectMsg(UID, 2, -1 , 1114, NPC, 27, -1);
		end
	end
end

----------------------------------------------------------------------------------------------------------------------

if (EVENT == 5000) then
	ShowMap(UID, 450);
end

----------------------------------------------------------------------------------------------------------------------

----------------------------------------------------------------------------------------------------------------------
-- JOB CHANGE: Job Change (Master) scroll 700112000 or Job Change (No Master) scroll 700113000 (the Master one is used first)

if (EVENT == 2850) then -- sub menu: Job Change / Nation Transfer
	SelectMsg(UID, 3, -1, 4131, NPC, 45486, 2860, 7772, 2899, 4296, 520);
end

if (EVENT == 2860) then
	SelectMsg(UID, 3, -1, 4131, NPC, 45428, 2871, 45429, 2872, 45430, 2873, 45431, 2874, 4296, 520);
end

local function JobChangeTo(uid, job)
	if (HowmuchItem(uid, 700112000) > 0) then
		JobChange(uid, 0, job);
	elseif (HowmuchItem(uid, 700113000) > 0) then
		JobChange(uid, 1, job);
	else
		SelectMsg(uid, 2, -1, 45055, NPC, 18, 5000);
	end
end

if (EVENT == 2871) then JobChangeTo(UID, 1); end -- Warrior
if (EVENT == 2872) then JobChangeTo(UID, 2); end -- Rogue
if (EVENT == 2873) then JobChangeTo(UID, 3); end -- Mage
if (EVENT == 2874) then JobChangeTo(UID, 4); end -- Priest

----------------------------------------------------------------------------------------------------------------------
-- NATION TRANSFER: Nation Transfer scroll 810096000

if (EVENT == 2899) then
	if (HowmuchItem(UID, 810096000) < 1) then
		SelectMsg(UID, 2, -1, 1523, NPC, 18, 5000);
	else
		SelectMsg(UID, 2, -1, 1524, NPC, 72, 2900, 73, -1);
	end
end

if (EVENT == 2900) then
	if (HowmuchItem(UID, 810096000) < 1) then
		SelectMsg(UID, 2, -1, 1523, NPC, 10, -1);
	else
		SendNationTransfer(UID);
	end
end
