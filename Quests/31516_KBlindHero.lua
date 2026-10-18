local NPC = 31516;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 20864, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 20864, NPC)
	else
		EVENT = QuestNum
	end
end

if(EVENT == 1000)then
NATION = CheckNation(UID);
if(NATION == 1)then
SaveEvent(UID, 11403);
end
end

if(EVENT == 1005)then
	SelectMsg(UID, 4, 547, 20404, NPC, 10, 1002,27,-1);
	SaveEvent(UID, 11406);
end

if(EVENT == 1003)then
NATION = CheckNation(UID);
if(NATION == 1)then
SelectMsg(UID, 2, 547, 20404, NPC, 10, 1005);
end
ItemCount0 = HowmuchItem(UID,900012000);
if(NATION == 2)then
if(ItemCount0 < 0)then
SelectMsg(UID, 2, 547, 192, NPC, 10, -1);
else
SelectMsg(UID, 2, 547, 20404, NPC, 10, 1005);
end
end
end

if(EVENT == 1002)then
NATION = CheckNation(UID);
if(NATION == 1)then
	SaveEvent(UID, 11405);
	SaveEvent(UID, 11416);
end
end
if(EVENT == 1100)then
NATION = CheckNation(UID);
ClassCheck = CheckClass(UID)
if(NATION == 1 and ClassCheck == 1 or ClassCheck == 5 or ClassCheck == 6 or ClassCheck == 13 or ClassCheck == 14 or ClassCheck == 15)then
SaveEvent(UID, 11415);
end
ClassCheck = CheckClass(UID)
if(NATION == 1 and ClassCheck == 2 or ClassCheck == 7 or ClassCheck == 8)then
SaveEvent(UID, 11420);
end
ClassCheck = CheckClass(UID)
if(NATION == 1 and ClassCheck == 3 or ClassCheck == 9 or ClassCheck == 10)then
SaveEvent(UID, 11425);
end
ClassCheck = CheckClass(UID)
if(NATION == 1 and ClassCheck == 4 or ClassCheck == 11 or ClassCheck == 12)then
SaveEvent(UID, 11430);
end
end
if(EVENT == 1102)then
NATION = CheckNation(UID);
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 1 or ClassCheck == 5 or ClassCheck == 6 or ClassCheck == 13 or ClassCheck == 14 or ClassCheck == 15)then
SaveEvent(UID, 11416);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 2 or ClassCheck == 7 or ClassCheck == 8)then
SaveEvent(UID, 11421);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 3 or ClassCheck == 9 or ClassCheck == 10)then
SaveEvent(UID, 11426);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 4 or ClassCheck == 11 or ClassCheck == 12)then
SaveEvent(UID, 11431);
end
end
if(EVENT == 1101)then
NATION = CheckNation(UID);
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 1 or ClassCheck == 5 or ClassCheck == 6 or ClassCheck == 13 or ClassCheck == 14 or ClassCheck == 15)then
SelectMsg(UID, 4, 548, 20064, NPC, 22, 1102, 23, -1);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 2 or ClassCheck == 7 or ClassCheck == 8)then
SelectMsg(UID, 4, 548, 20064, NPC, 22, 1102, 23, -1);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 3 or ClassCheck == 9 or ClassCheck == 10)then
SelectMsg(UID, 4, 548, 20064, NPC, 22, 1102, 23, -1);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 4 or ClassCheck == 11 or ClassCheck == 12)then
SelectMsg(UID, 4, 548, 20064, NPC, 22, 1102, 23, -1);
end
end

if (EVENT == 1105) then
	RELICHERO = HowmuchItem(UID, 508108000);
	if (RELICHERO < 5) then
		SelectMsg(UID, 2, -1, 11380, NPC, 18, -1);
	else
		Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SelectMsg(UID, 5, 548, 749, NPC, 4006, 880, 4005, -1);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SelectMsg(UID, 5, 548, 749, NPC, 4006, 881, 4005, -1);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SelectMsg(UID, 5, 548, 749, NPC, 4006, 882, 4005, -1);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SelectMsg(UID, 5, 548, 749, NPC, 4006, 883, 4005, -1);
    end
end
end

if (EVENT == 880) then
	RELICHERO = HowmuchItem(UID, 508108000);
	SlotCheck = CheckGiveSlot(UID, 3)
	if SlotCheck == false then
	elseif (RELICHERO < 5) then
		SelectMsg(UID, 5, 548, 21624, NPC, 10, -1);
    else
	RunQuestExchange(UID,3035);
	RobItem(UID, 508108000, 5);
	SaveEvent(UID, 11417);
	SaveEvent(UID, 11458);
    end
end


if (EVENT == 881) then
	RELICHERO = HowmuchItem(UID, 508108000);
	SlotCheck = CheckGiveSlot(UID, 3)
	if SlotCheck == false then
	elseif (RELICHERO < 5) then
		SelectMsg(UID, 2, 548, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3036);
	RobItem(UID, 508108000, 5);
	SaveEvent(UID, 11422);
	SaveEvent(UID, 11458);
    end
end


if (EVENT == 882) then
	RELICHERO = HowmuchItem(UID, 508108000);
	SlotCheck = CheckGiveSlot(UID, 3)
	if SlotCheck == false then
	elseif (RELICHERO < 5) then
		SelectMsg(UID, 2, 548, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3037);
	RobItem(UID, 508108000, 5);
	SaveEvent(UID, 11427);
	SaveEvent(UID, 11458);
    end
end


if (EVENT == 883) then
	RELICHERO = HowmuchItem(UID, 508108000);
	SlotCheck = CheckGiveSlot(UID, 3)
	if SlotCheck == false then
	elseif (RELICHERO < 5) then
		SelectMsg(UID, 2, 548, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3038);
	RobItem(UID, 508108000, 5);
	SaveEvent(UID, 11432);
	SaveEvent(UID, 11458);
    end
end


if(EVENT == 1104)then
NATION = CheckNation(UID);
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 1 or ClassCheck == 5 or ClassCheck == 6 or ClassCheck == 13 or ClassCheck == 14 or ClassCheck == 15)then
SaveEvent(UID, 11418);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 2 or ClassCheck == 7 or ClassCheck == 8)then
SaveEvent(UID, 11423);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 3 or ClassCheck == 9 or ClassCheck == 10)then
SaveEvent(UID, 11428);
end
ClassCheck = CheckClass (UID);
if(NATION == 1 and ClassCheck == 4 or ClassCheck == 11 or ClassCheck == 12)then
SaveEvent(UID, 11433);
end
end
if(EVENT == 1200)then
NATION = CheckNation(UID);
if(NATION == 1)then
SaveEvent(UID, 11457);
end
end
if(EVENT == 1202)then
NATION = CheckNation(UID);
if(NATION == 1)then
SaveEvent(UID, 11458);
end
end
if(EVENT == 1201)then
NATION = CheckNation(UID);
if(NATION == 1)then
SelectMsg(UID, 4, 549, 20066, NPC, 22, 1202, 23, -1);
end
end

if (EVENT == 1205) then
	EXHALSPRIT = HowmuchItem(UID, 910228000);
	if (EXHALSPRIT < 1) then
		SelectMsg(UID, 2, 549, 21624, NPC, 10, -1);
	else
		SelectMsg(UID, 5, 549, 20066, NPC, 10, 1208, 27, -1);
	end
end

if (EVENT == 1208) then
	EXHALSPRIT = HowmuchItem(UID, 910228000);
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif (EXHALSPRIT < 1) then
		SelectMsg(UID, 2, 549, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3039);
	RobItem(UID, 910228000, 1);
	SaveEvent(UID, 11459);
	SaveEvent(UID, 11470);
	SaveEvent(UID, 11472);
    end
end

if(EVENT == 1204)then
NATION = CheckNation(UID);
if(NATION == 1)then
SaveEvent(UID, 11460);
end
end
if(EVENT == 1300)then
	NATION = CheckNation(UID);
if(NATION == 1)then
	SaveEvent(UID, 11517);
	end
end

if(EVENT == 1303)then
NATION = CheckNation(UID);
if(NATION == 1)then
SelectMsg(UID, 2, -1, 20482, NPC, 10, 1304);
	end
end

if(EVENT == 1304)then
NATION = CheckNation(UID);
if(NATION == 1)then
SelectMsg(UID, 4, 554, 20484, NPC, 10, 1002,27,-1);
SaveEvent(UID, 11520);
	end
end

if(EVENT == 1305)then
NATION = CheckNation(UID);
if(NATION == 1)then
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck then
		GiveItem(UID, 910230000,1);
	end
	SelectMsg(UID, 4, 554, 20484, NPC, 10, -1);
SaveEvent(UID, 11519);
SaveEvent(UID, 11530);
SaveEvent(UID, 11532);
	end
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=548 status=2 n_index=11417
if (EVENT == 101) then
	QuestStatusCheck = GetQuestStatus(UID, 548)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SelectMsg(UID, 5, 548, 749, NPC, 4006, 880, 4005, -1);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SelectMsg(UID, 5, 548, 749, NPC, 4006, 881, 4005, -1);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SelectMsg(UID, 5, 548, 749, NPC, 4006, 882, 4005, -1);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SelectMsg(UID, 5, 548, 749, NPC, 4006, 883, 4005, -1);
		end
	end
end

-- [AUTO-GEN] quest=547 status=2 n_index=11405
if (EVENT == 193) then
	QuestStatusCheck = GetQuestStatus(UID, 547)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3034);
		SaveEvent(UID, 11407);
	end
end

-- [AUTO-GEN] quest=554 status=0 n_index=11517
if (EVENT == 1302) then
	SelectMsg(UID, 4, 554, 20076, NPC, 3076, 1303, 23, -1);
end

