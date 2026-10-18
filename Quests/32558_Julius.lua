local NPC = 32558;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 1034, NPC,10,-1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 1034, NPC)
	else
		EVENT = QuestNum
	end
end
--if EVENT == 100 then
	--SelectMsg(UID, 2, -1, 1034, NPC,10,-1);
--end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=191 status=2 n_index=1297
if (EVENT == 101) then
	QuestStatusCheck = GetQuestStatus(UID, 191)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 194)) then
			SaveEvent(UID, 1299);
		end
	end
end

-- [AUTO-GEN] quest=295 status=255 n_index=751
if (EVENT == 102) then
	SaveEvent(UID, 752);
end

-- [AUTO-GEN] quest=295 status=0 n_index=752
if (EVENT == 104) then
	SelectMsg(UID, 4, 295, 1055, NPC, 128, 105, 23, -1);
end

-- [AUTO-GEN] quest=295 status=0 n_index=752
if (EVENT == 105) then
	SaveEvent(UID, 753);
end

-- [AUTO-GEN] quest=295 status=1 n_index=753
if (EVENT == 111) then
	QuestStatusCheck = GetQuestStatus(UID, 295)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 128)) then
			SaveEvent(UID, 754);
		end
	end
end

-- [AUTO-GEN] quest=295 status=1 n_index=753
if (EVENT == 112) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 295, 1055, NPC, 22, 111, 23, -1);
	else
		SelectMsg(UID, 2, 295, 1055, NPC, 18, 113);
	end
end

-- [AUTO-GEN] quest=295 status=1 n_index=753
if (EVENT == 113) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=297 status=0 n_index=774
if (EVENT == 124) then
	SelectMsg(UID, 4, 297, 1055, NPC, 128, 125, 23, -1);
end

-- [AUTO-GEN] quest=297 status=0 n_index=774
if (EVENT == 125) then
	SaveEvent(UID, 775);
end

-- [AUTO-GEN] quest=297 status=1 n_index=775
if (EVENT == 131) then
	QuestStatusCheck = GetQuestStatus(UID, 297)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 128)) then
			SaveEvent(UID, 776);
		end
	end
end

-- [AUTO-GEN] quest=297 status=1 n_index=775
if (EVENT == 132) then
	ItemA = HowmuchItem(UID, 900068000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 297, 1055, NPC, 18, 133);
	else
		SelectMsg(UID, 4, 297, 1055, NPC, 41, 131, 27, -1);
	end
end

-- [AUTO-GEN] quest=297 status=1 n_index=775
if (EVENT == 133) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=299 status=0 n_index=794
if (EVENT == 144) then
	SelectMsg(UID, 4, 299, 1055, NPC, 128, 145, 23, -1);
end

-- [AUTO-GEN] quest=299 status=0 n_index=794
if (EVENT == 145) then
	SaveEvent(UID, 795);
end

-- [AUTO-GEN] quest=299 status=1 n_index=795
if (EVENT == 151) then
	QuestStatusCheck = GetQuestStatus(UID, 299)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 128)) then
			SaveEvent(UID, 796);
		end
	end
end

-- [AUTO-GEN] quest=299 status=1 n_index=795
if (EVENT == 152) then
	ItemA = HowmuchItem(UID, 900068000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 299, 1055, NPC, 18, 153);
	else
		SelectMsg(UID, 4, 299, 1055, NPC, 41, 151, 27, -1);
	end
end

-- [AUTO-GEN] quest=299 status=1 n_index=795
if (EVENT == 153) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=301 status=0 n_index=814
if (EVENT == 164) then
	SelectMsg(UID, 4, 301, 1055, NPC, 128, 165, 23, -1);
end

-- [AUTO-GEN] quest=301 status=0 n_index=814
if (EVENT == 165) then
	SaveEvent(UID, 815);
end

-- [AUTO-GEN] quest=301 status=1 n_index=815
if (EVENT == 171) then
	QuestStatusCheck = GetQuestStatus(UID, 301)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 128)) then
			SaveEvent(UID, 816);
		end
	end
end

-- [AUTO-GEN] quest=301 status=1 n_index=815
if (EVENT == 172) then
	ItemA = HowmuchItem(UID, 900068000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 301, 1055, NPC, 18, 173);
	else
		SelectMsg(UID, 4, 301, 1055, NPC, 41, 171, 27, -1);
	end
end

-- [AUTO-GEN] quest=301 status=1 n_index=815
if (EVENT == 173) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=296 status=0 n_index=764
if (EVENT == 184) then
	SelectMsg(UID, 4, 296, 1074, NPC, 129, 185, 23, -1);
end

-- [AUTO-GEN] quest=296 status=0 n_index=764
if (EVENT == 185) then
	SaveEvent(UID, 765);
end

-- [AUTO-GEN] quest=296 status=1 n_index=765
if (EVENT == 191) then
	QuestStatusCheck = GetQuestStatus(UID, 296)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 127)) then
			SaveEvent(UID, 766);
		end
	end
end

-- [AUTO-GEN] quest=296 status=1 n_index=765
if (EVENT == 192) then
	ItemA = HowmuchItem(UID, 900071000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 296, 1074, NPC, 18, 193);
	else
		SelectMsg(UID, 4, 296, 1074, NPC, 41, 191, 27, -1);
	end
end

-- [AUTO-GEN] quest=296 status=1 n_index=765
if (EVENT == 193) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=298 status=0 n_index=784
if (EVENT == 204) then
	SelectMsg(UID, 4, 298, 1074, NPC, 129, 205, 23, -1);
end

-- [AUTO-GEN] quest=298 status=0 n_index=784
if (EVENT == 205) then
	SaveEvent(UID, 785);
end

-- [AUTO-GEN] quest=298 status=1 n_index=785
if (EVENT == 211) then
	QuestStatusCheck = GetQuestStatus(UID, 298)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 127)) then
			SaveEvent(UID, 786);
		end
	end
end

-- [AUTO-GEN] quest=298 status=1 n_index=785
if (EVENT == 212) then
	ItemA = HowmuchItem(UID, 900071000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 298, 1074, NPC, 18, 213);
	else
		SelectMsg(UID, 4, 298, 1074, NPC, 41, 211, 27, -1);
	end
end

-- [AUTO-GEN] quest=298 status=1 n_index=785
if (EVENT == 213) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=300 status=0 n_index=804
if (EVENT == 224) then
	SelectMsg(UID, 4, 300, 1074, NPC, 129, 225, 23, -1);
end

-- [AUTO-GEN] quest=300 status=0 n_index=804
if (EVENT == 225) then
	SaveEvent(UID, 805);
end

-- [AUTO-GEN] quest=300 status=1 n_index=805
if (EVENT == 231) then
	QuestStatusCheck = GetQuestStatus(UID, 300)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 127)) then
			SaveEvent(UID, 806);
		end
	end
end

-- [AUTO-GEN] quest=300 status=1 n_index=805
if (EVENT == 232) then
	ItemA = HowmuchItem(UID, 900071000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 300, 1074, NPC, 18, 233);
	else
		SelectMsg(UID, 4, 300, 1074, NPC, 41, 231, 27, -1);
	end
end

-- [AUTO-GEN] quest=300 status=1 n_index=805
if (EVENT == 233) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=371 status=0 n_index=1053
if (EVENT == 240) then
	SelectMsg(UID, 4, 371, 1483, NPC, 157, 241, 23, -1);
end

-- [AUTO-GEN] quest=371 status=0 n_index=1053
if (EVENT == 241) then
	SaveEvent(UID, 1054);
end

-- [AUTO-GEN] quest=371 status=1 n_index=1054
if (EVENT == 243) then
	QuestStatusCheck = GetQuestStatus(UID, 371)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1055);
		end
	end
end

-- [AUTO-GEN] quest=371 status=1 n_index=1054
if (EVENT == 244) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 371, 1483, NPC, 18, 245);
	else
		SelectMsg(UID, 4, 371, 1483, NPC, 41, 243, 27, -1);
	end
end

-- [AUTO-GEN] quest=371 status=1 n_index=1054
if (EVENT == 245) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=372 status=0 n_index=1063
if (EVENT == 250) then
	SelectMsg(UID, 4, 372, 1483, NPC, 157, 251, 23, -1);
end

-- [AUTO-GEN] quest=372 status=0 n_index=1063
if (EVENT == 251) then
	SaveEvent(UID, 1064);
end

-- [AUTO-GEN] quest=372 status=1 n_index=1064
if (EVENT == 253) then
	QuestStatusCheck = GetQuestStatus(UID, 372)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1065);
		end
	end
end

-- [AUTO-GEN] quest=372 status=1 n_index=1064
if (EVENT == 254) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 372, 1483, NPC, 18, 255);
	else
		SelectMsg(UID, 4, 372, 1483, NPC, 41, 253, 27, -1);
	end
end

-- [AUTO-GEN] quest=372 status=1 n_index=1064
if (EVENT == 255) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=373 status=0 n_index=1073
if (EVENT == 260) then
	SelectMsg(UID, 4, 373, 1483, NPC, 157, 261, 23, -1);
end

-- [AUTO-GEN] quest=373 status=0 n_index=1073
if (EVENT == 261) then
	SaveEvent(UID, 1074);
end

-- [AUTO-GEN] quest=373 status=1 n_index=1074
if (EVENT == 263) then
	QuestStatusCheck = GetQuestStatus(UID, 373)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1075);
		end
	end
end

-- [AUTO-GEN] quest=373 status=1 n_index=1074
if (EVENT == 264) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 373, 1483, NPC, 18, 265);
	else
		SelectMsg(UID, 4, 373, 1483, NPC, 41, 263, 27, -1);
	end
end

-- [AUTO-GEN] quest=373 status=1 n_index=1074
if (EVENT == 265) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=374 status=0 n_index=1083
if (EVENT == 270) then
	SelectMsg(UID, 4, 374, 1483, NPC, 157, 271, 23, -1);
end

-- [AUTO-GEN] quest=374 status=0 n_index=1083
if (EVENT == 271) then
	SaveEvent(UID, 1084);
end

-- [AUTO-GEN] quest=374 status=1 n_index=1084
if (EVENT == 273) then
	QuestStatusCheck = GetQuestStatus(UID, 374)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1085);
		end
	end
end

-- [AUTO-GEN] quest=374 status=1 n_index=1084
if (EVENT == 274) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 374, 1483, NPC, 18, 275);
	else
		SelectMsg(UID, 4, 374, 1483, NPC, 41, 273, 27, -1);
	end
end

-- [AUTO-GEN] quest=374 status=1 n_index=1084
if (EVENT == 275) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=375 status=0 n_index=1093
if (EVENT == 280) then
	SelectMsg(UID, 4, 375, 1483, NPC, 157, 281, 23, -1);
end

-- [AUTO-GEN] quest=375 status=0 n_index=1093
if (EVENT == 281) then
	SaveEvent(UID, 1094);
end

-- [AUTO-GEN] quest=375 status=1 n_index=1094
if (EVENT == 283) then
	QuestStatusCheck = GetQuestStatus(UID, 375)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1095);
		end
	end
end

-- [AUTO-GEN] quest=375 status=1 n_index=1094
if (EVENT == 284) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 375, 1483, NPC, 18, 285);
	else
		SelectMsg(UID, 4, 375, 1483, NPC, 41, 283, 27, -1);
	end
end

-- [AUTO-GEN] quest=375 status=1 n_index=1094
if (EVENT == 285) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=376 status=0 n_index=1103
if (EVENT == 290) then
	SelectMsg(UID, 4, 376, 1483, NPC, 157, 291, 23, -1);
end

-- [AUTO-GEN] quest=376 status=0 n_index=1103
if (EVENT == 291) then
	SaveEvent(UID, 1104);
end

-- [AUTO-GEN] quest=376 status=1 n_index=1104
if (EVENT == 293) then
	QuestStatusCheck = GetQuestStatus(UID, 376)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1105);
		end
	end
end

-- [AUTO-GEN] quest=376 status=1 n_index=1104
if (EVENT == 294) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 376, 1483, NPC, 18, 295);
	else
		SelectMsg(UID, 4, 376, 1483, NPC, 41, 293, 27, -1);
	end
end

-- [AUTO-GEN] quest=376 status=1 n_index=1104
if (EVENT == 295) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=377 status=0 n_index=1113
if (EVENT == 300) then
	SelectMsg(UID, 4, 377, 1483, NPC, 157, 301, 23, -1);
end

-- [AUTO-GEN] quest=377 status=0 n_index=1113
if (EVENT == 301) then
	SaveEvent(UID, 1114);
end

-- [AUTO-GEN] quest=377 status=1 n_index=1114
if (EVENT == 303) then
	QuestStatusCheck = GetQuestStatus(UID, 377)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1115);
		end
	end
end

-- [AUTO-GEN] quest=377 status=1 n_index=1114
if (EVENT == 304) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 377, 1483, NPC, 18, 305);
	else
		SelectMsg(UID, 4, 377, 1483, NPC, 41, 303, 27, -1);
	end
end

-- [AUTO-GEN] quest=377 status=1 n_index=1114
if (EVENT == 305) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=191 status=0 n_index=1295
if (EVENT == 400) then
	SelectMsg(UID, 4, 191, 8882, NPC, 174, 401, 23, -1);
end

-- [AUTO-GEN] quest=191 status=0 n_index=1295
if (EVENT == 401) then
	SaveEvent(UID, 1296);
end

-- [AUTO-GEN] quest=191 status=1 n_index=1296
if (EVENT == 420) then
	QuestStatusCheck = GetQuestStatus(UID, 191)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 194)) then
			SaveEvent(UID, 1297);
		end
	end
end

-- [AUTO-GEN] quest=1756 status=0 n_index=14670
if (EVENT == 1200) then
	SelectMsg(UID, 4, 1756, 45528, NPC, 3577, 1201, 23, -1);
end

-- [AUTO-GEN] quest=1756 status=0 n_index=14670
if (EVENT == 1201) then
	SaveEvent(UID, 14671);
end

-- [AUTO-GEN] quest=1756 status=1 n_index=14671
if (EVENT == 1202) then
	QuestStatusCheck = GetQuestStatus(UID, 1756)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16268)) then
			SaveEvent(UID, 14672);
		end
	end
end

-- [AUTO-GEN] quest=1756 status=1 n_index=14671
if (EVENT == 1203) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1756, 45528, NPC, 22, 1202, 23, -1);
	else
		SelectMsg(UID, 2, 1756, 45528, NPC, 18, 1204);
	end
end

-- [AUTO-GEN] quest=1756 status=1 n_index=14671
if (EVENT == 1204) then
	ShowMap(UID, 71);
end


-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=1769 status=0 n_index=14760
if (EVENT == 2000) then
	SelectMsg(UID, 4, 1769, 45645, NPC, 3583, 2001, 23, -1);
end

-- [AUTO-GEN] quest=1769 status=0 n_index=14760
if (EVENT == 2001) then
	SaveEvent(UID, 14761);
end

-- [AUTO-GEN] quest=1769 status=1 n_index=14761
if (EVENT == 2004) then
	QuestStatusCheck = GetQuestStatus(UID, 1769)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16303)) then
			SaveEvent(UID, 14762);
		end
	end
end

-- [AUTO-GEN] quest=1769 status=1 n_index=14761
if (EVENT == 2005) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1769, 45645, NPC, 22, 2004, 23, -1);
	else
		SelectMsg(UID, 2, 1769, 45645, NPC, 18, 2006);
	end
end

-- [AUTO-GEN] quest=1769 status=1 n_index=14761
if (EVENT == 2006) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=1771 status=0 n_index=14770
if (EVENT == 2010) then
	SelectMsg(UID, 4, 1771, 45646, NPC, 3584, 2011, 23, -1);
end

-- [AUTO-GEN] quest=1771 status=0 n_index=14770
if (EVENT == 2011) then
	SaveEvent(UID, 14771);
end

-- [AUTO-GEN] quest=1771 status=1 n_index=14771
if (EVENT == 2014) then
	QuestStatusCheck = GetQuestStatus(UID, 1771)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16304)) then
			SaveEvent(UID, 14772);
		end
	end
end

-- [AUTO-GEN] quest=1771 status=1 n_index=14771
if (EVENT == 2015) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1771, 45646, NPC, 22, 2014, 23, -1);
	else
		SelectMsg(UID, 2, 1771, 45646, NPC, 18, 2016);
	end
end

-- [AUTO-GEN] quest=1771 status=1 n_index=14771
if (EVENT == 2016) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=1773 status=0 n_index=14780
if (EVENT == 2020) then
	SelectMsg(UID, 4, 1773, 45647, NPC, 3585, 2021, 23, -1);
end

-- [AUTO-GEN] quest=1773 status=0 n_index=14780
if (EVENT == 2021) then
	SaveEvent(UID, 14781);
end

-- [AUTO-GEN] quest=1773 status=1 n_index=14781
if (EVENT == 2024) then
	QuestStatusCheck = GetQuestStatus(UID, 1773)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16305)) then
			SaveEvent(UID, 14782);
		end
	end
end

-- [AUTO-GEN] quest=1773 status=1 n_index=14781
if (EVENT == 2025) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1773, 45647, NPC, 22, 2024, 23, -1);
	else
		SelectMsg(UID, 2, 1773, 45647, NPC, 18, 2026);
	end
end

-- [AUTO-GEN] quest=1773 status=1 n_index=14781
if (EVENT == 2026) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=1775 status=0 n_index=14790
if (EVENT == 2030) then
	SelectMsg(UID, 4, 1775, 45648, NPC, 3586, 2031, 23, -1);
end

-- [AUTO-GEN] quest=1775 status=0 n_index=14790
if (EVENT == 2031) then
	SaveEvent(UID, 14791);
end

-- [AUTO-GEN] quest=1775 status=1 n_index=14791
if (EVENT == 2034) then
	QuestStatusCheck = GetQuestStatus(UID, 1775)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16306)) then
			SaveEvent(UID, 14792);
		end
	end
end

-- [AUTO-GEN] quest=1775 status=1 n_index=14791
if (EVENT == 2035) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1775, 45648, NPC, 22, 2034, 23, -1);
	else
		SelectMsg(UID, 2, 1775, 45648, NPC, 18, 2036);
	end
end

-- [AUTO-GEN] quest=1775 status=1 n_index=14791
if (EVENT == 2036) then
	ShowMap(UID, 71);
end

