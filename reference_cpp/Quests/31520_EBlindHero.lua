      --Created By TriLogy----
---TriLogy Tarafından Yazılmıştır.---
  ----------27.06.2020----------
local NPC = 31520;

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
if(NATION == 2)then
SaveEvent(UID, 11409);
end
end

if(EVENT == 1005)then
	SelectMsg(UID, 4, 547, 20408, NPC, 10, 1002,27,-1);
	SaveEvent(UID, 11412);
end

if(EVENT == 1003)then
NATION = CheckNation(UID);
if(NATION == 2)then
SelectMsg(UID, 2, 547, 20408, NPC, 10, 1005);
end
ItemCount0 = HowmuchItem(UID,900012000);
if(NATION == 2)then
if(ItemCount0 < 0)then
SelectMsg(UID, 2, 547, 192, NPC, 10, -1);
else
SelectMsg(UID, 2, 547, 20408, NPC, 10, 1005);
end
end
end

if(EVENT == 1002)then
NATION = CheckNation(UID);
if(NATION == 2)then
	SaveEvent(UID, 11411);
	SaveEvent(UID, 11437);
end
end
if(EVENT == 1100)then
NATION = CheckNation(UID);
ClassCheck = CheckClass(UID)
if(NATION == 2 and ClassCheck == 1 or ClassCheck == 5 or ClassCheck == 6 or ClassCheck == 13 or ClassCheck == 14 or ClassCheck == 15)then
SaveEvent(UID, 11436);
end
ClassCheck = CheckClass(UID)
if(NATION == 2 and ClassCheck == 2 or ClassCheck == 7 or ClassCheck == 8)then
SaveEvent(UID, 11441);
end
ClassCheck = CheckClass(UID)
if(NATION == 2 and ClassCheck == 3 or ClassCheck == 9 or ClassCheck == 10)then
SaveEvent(UID, 11446);
end
ClassCheck = CheckClass(UID)
if(NATION == 2 and ClassCheck == 4 or ClassCheck == 11 or ClassCheck == 12)then
SaveEvent(UID, 11451);
end
end
if(EVENT == 1102)then
NATION = CheckNation(UID);
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 1 or ClassCheck == 5 or ClassCheck == 6 or ClassCheck == 13 or ClassCheck == 14 or ClassCheck == 15)then
SaveEvent(UID, 11437);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 2 or ClassCheck == 7 or ClassCheck == 8)then
SaveEvent(UID, 11442);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 3 or ClassCheck == 9 or ClassCheck == 10)then
SaveEvent(UID, 11447);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 4 or ClassCheck == 11 or ClassCheck == 12)then
SaveEvent(UID, 11452);
end
end
if(EVENT == 1101)then
NATION = CheckNation(UID);
ClassCheck = CheckClass (UID);
if (NATION == 2 and ClassCheck == 1 or ClassCheck == 5 or ClassCheck == 6 or ClassCheck == 13 or ClassCheck == 14 or ClassCheck == 15) then
SelectMsg(UID, 4, 548, 20065, NPC, 22, 1102, 23, -1);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 2 or ClassCheck == 7 or ClassCheck == 8)then
SelectMsg(UID, 4, 548, 20065, NPC, 22, 1102, 23, -1);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 3 or ClassCheck == 9 or ClassCheck == 10)then
SelectMsg(UID, 4, 548, 20065, NPC, 22, 1102, 23, -1);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 4 or ClassCheck == 11 or ClassCheck == 12)then
SelectMsg(UID, 4, 548, 20065, NPC, 22, 1102, 23, -1);
end
end

if (EVENT == 1105) then
	RELICHERO = HowmuchItem(UID, 508108000);
	if (RELICHERO < 5 ) then
	SelectMsg(UID, 2, -1, 21630, NPC, 10, -1)
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
		SelectMsg(UID, 2, 548, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3035); 
	RobItem(UID, 508108000, 5);
	SaveEvent(UID, 11438);
	SaveEvent(UID, 11464);
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
	SaveEvent(UID, 11443);
	SaveEvent(UID, 11464);
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
		SaveEvent(UID, 11448);
		SaveEvent(UID, 11464);
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
	SaveEvent(UID, 11453);
	SaveEvent(UID, 11464);
    end
end


if(EVENT == 1104)then
NATION = CheckNation(UID);
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 1 or ClassCheck == 5 or ClassCheck == 6 or ClassCheck == 13 or ClassCheck == 14 or ClassCheck == 15)then
SaveEvent(UID, 11439);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 2 or ClassCheck == 7 or ClassCheck == 8)then
SaveEvent(UID, 11444);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 3 or ClassCheck == 9 or ClassCheck == 10)then
SaveEvent(UID, 11449);
end
ClassCheck = CheckClass (UID);
if(NATION == 2 and ClassCheck == 4 or ClassCheck == 11 or ClassCheck == 12)then
SaveEvent(UID, 11454);
end
end
if(EVENT == 1200)then
NATION = CheckNation(UID);
if(NATION == 2)then
SaveEvent(UID, 11463);
end
end
if(EVENT == 1202)then
NATION = CheckNation(UID);
if(NATION == 2)then
SaveEvent(UID, 11464);
end
end
if(EVENT == 1201)then
NATION = CheckNation(UID);
if(NATION == 2)then
SelectMsg(UID, 4, 549, 20067, NPC, 22, 1202, 23, -1);
end
end

if (EVENT == 1205) then
	EXHALSPRIT = HowmuchItem(UID, 910228000);
	if (EXHALSPRIT < 1 ) then
		SelectMsg(UID, 2, 549, 21630, NPC, 10, -1);
	else
		SelectMsg(UID, 5, 549, 21624, NPC, 10, 1208, 27,-1);
	end
end

if (EVENT == 1208) then
	EXHALSPRIT = HowmuchItem(UID, 910228000);
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
		SelectMsg(UID, 2, 549, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3039);
	RobItem(UID, 910228000, 1);
	SaveEvent(UID, 11465);
	SaveEvent(UID, 11476);
	SaveEvent(UID, 11478);
    end
end

if(EVENT == 1204)then
NATION = CheckNation(UID);
if(NATION == 2)then
SaveEvent(UID, 11466);
	end
end
if(EVENT == 1300)then
	NATION = CheckNation(UID);
if(NATION == 2)then
	SaveEvent(UID, 11523);
	end
end

if(EVENT == 1303)then
NATION = CheckNation(UID);
if(NATION == 2)then
SelectMsg(UID, 2, -1, 20487, NPC, 10, 1304);
	end
end

if(EVENT == 1304)then
NATION = CheckNation(UID);
if(NATION == 2)then
SelectMsg(UID, 4, 554, 20489, NPC, 10, 1002,27,-1);
SaveEvent(UID, 11526);
	end
end

if(EVENT == 1305)then
NATION = CheckNation(UID);
if(NATION == 2)then
	GiveItem(UID, 910230000,1);
	SelectMsg(UID, 4, 554, 20401, NPC, 10, -1);
SaveEvent(UID, 11525);
SaveEvent(UID, 11536);
SaveEvent(UID, 11538);
	end
end