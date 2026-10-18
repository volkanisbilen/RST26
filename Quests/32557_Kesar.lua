local NPC = 32557;

if EVENT == 100 then
   SelectMsg(UID, 2, -1, 1033, NPC,10,-1);
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=191 status=2 n_index=1292
if (EVENT == 101) then
	QuestStatusCheck = GetQuestStatus(UID, 191)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 193)) then
			SaveEvent(UID, 1294);
		end
	end
end

-- [AUTO-GEN] quest=295 status=255 n_index=745
if (EVENT == 102) then
	SaveEvent(UID, 746);
end

-- [AUTO-GEN] quest=295 status=0 n_index=746
if (EVENT == 104) then
	SelectMsg(UID, 4, 295, 1043, NPC, 128, 105, 23, -1);
end

-- [AUTO-GEN] quest=295 status=0 n_index=746
if (EVENT == 105) then
	SaveEvent(UID, 747);
end

-- [AUTO-GEN] quest=295 status=1 n_index=747
if (EVENT == 111) then
	QuestStatusCheck = GetQuestStatus(UID, 295)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 128)) then
			SaveEvent(UID, 748);
		end
	end
end

-- [AUTO-GEN] quest=295 status=1 n_index=747
if (EVENT == 112) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 295, 1043, NPC, 22, 111, 23, -1);
	else
		SelectMsg(UID, 2, 295, 1043, NPC, 18, 113);
	end
end

-- [AUTO-GEN] quest=295 status=1 n_index=747
if (EVENT == 113) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=297 status=0 n_index=769
if (EVENT == 124) then
	SelectMsg(UID, 4, 297, 1043, NPC, 128, 125, 23, -1);
end

-- [AUTO-GEN] quest=297 status=0 n_index=769
if (EVENT == 125) then
	SaveEvent(UID, 770);
end

-- [AUTO-GEN] quest=297 status=1 n_index=770
if (EVENT == 131) then
	QuestStatusCheck = GetQuestStatus(UID, 297)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 128)) then
			SaveEvent(UID, 771);
		end
	end
end

-- [AUTO-GEN] quest=297 status=1 n_index=770
if (EVENT == 132) then
	ItemA = HowmuchItem(UID, 900068000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 297, 1043, NPC, 18, 133);
	else
		SelectMsg(UID, 4, 297, 1043, NPC, 41, 131, 27, -1);
	end
end

-- [AUTO-GEN] quest=297 status=1 n_index=770
if (EVENT == 133) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=299 status=0 n_index=789
if (EVENT == 144) then
	SelectMsg(UID, 4, 299, 1043, NPC, 128, 145, 23, -1);
end

-- [AUTO-GEN] quest=299 status=0 n_index=789
if (EVENT == 145) then
	SaveEvent(UID, 790);
end

-- [AUTO-GEN] quest=299 status=1 n_index=790
if (EVENT == 151) then
	QuestStatusCheck = GetQuestStatus(UID, 299)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 128)) then
			SaveEvent(UID, 791);
		end
	end
end

-- [AUTO-GEN] quest=299 status=1 n_index=790
if (EVENT == 152) then
	ItemA = HowmuchItem(UID, 900068000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 299, 1043, NPC, 18, 153);
	else
		SelectMsg(UID, 4, 299, 1043, NPC, 41, 151, 27, -1);
	end
end

-- [AUTO-GEN] quest=299 status=1 n_index=790
if (EVENT == 153) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=301 status=0 n_index=809
if (EVENT == 164) then
	SelectMsg(UID, 4, 301, 1043, NPC, 128, 165, 23, -1);
end

-- [AUTO-GEN] quest=301 status=0 n_index=809
if (EVENT == 165) then
	SaveEvent(UID, 810);
end

-- [AUTO-GEN] quest=301 status=1 n_index=810
if (EVENT == 171) then
	QuestStatusCheck = GetQuestStatus(UID, 301)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 128)) then
			SaveEvent(UID, 811);
		end
	end
end

-- [AUTO-GEN] quest=301 status=1 n_index=810
if (EVENT == 172) then
	ItemA = HowmuchItem(UID, 900068000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 301, 1043, NPC, 18, 173);
	else
		SelectMsg(UID, 4, 301, 1043, NPC, 41, 171, 27, -1);
	end
end

-- [AUTO-GEN] quest=301 status=1 n_index=810
if (EVENT == 173) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=296 status=0 n_index=759
if (EVENT == 184) then
	SelectMsg(UID, 4, 296, 1062, NPC, 129, 185, 23, -1);
end

-- [AUTO-GEN] quest=296 status=0 n_index=759
if (EVENT == 185) then
	SaveEvent(UID, 760);
end

-- [AUTO-GEN] quest=296 status=1 n_index=760
if (EVENT == 191) then
	QuestStatusCheck = GetQuestStatus(UID, 296)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 127)) then
			SaveEvent(UID, 761);
		end
	end
end

-- [AUTO-GEN] quest=296 status=1 n_index=760
if (EVENT == 192) then
	ItemA = HowmuchItem(UID, 900071000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 296, 1062, NPC, 18, 193);
	else
		SelectMsg(UID, 4, 296, 1062, NPC, 41, 191, 27, -1);
	end
end

-- [AUTO-GEN] quest=296 status=1 n_index=760
if (EVENT == 193) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=298 status=0 n_index=779
if (EVENT == 204) then
	SelectMsg(UID, 4, 298, 1062, NPC, 129, 205, 23, -1);
end

-- [AUTO-GEN] quest=298 status=0 n_index=779
if (EVENT == 205) then
	SaveEvent(UID, 780);
end

-- [AUTO-GEN] quest=298 status=1 n_index=780
if (EVENT == 211) then
	QuestStatusCheck = GetQuestStatus(UID, 298)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 127)) then
			SaveEvent(UID, 781);
		end
	end
end

-- [AUTO-GEN] quest=298 status=1 n_index=780
if (EVENT == 212) then
	ItemA = HowmuchItem(UID, 900071000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 298, 1062, NPC, 18, 213);
	else
		SelectMsg(UID, 4, 298, 1062, NPC, 41, 211, 27, -1);
	end
end

-- [AUTO-GEN] quest=298 status=1 n_index=780
if (EVENT == 213) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=300 status=0 n_index=799
if (EVENT == 224) then
	SelectMsg(UID, 4, 300, 1062, NPC, 129, 225, 23, -1);
end

-- [AUTO-GEN] quest=300 status=0 n_index=799
if (EVENT == 225) then
	SaveEvent(UID, 800);
end

-- [AUTO-GEN] quest=300 status=1 n_index=800
if (EVENT == 231) then
	QuestStatusCheck = GetQuestStatus(UID, 300)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 127)) then
			SaveEvent(UID, 801);
		end
	end
end

-- [AUTO-GEN] quest=300 status=1 n_index=800
if (EVENT == 232) then
	ItemA = HowmuchItem(UID, 900071000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 300, 1062, NPC, 18, 233);
	else
		SelectMsg(UID, 4, 300, 1062, NPC, 41, 231, 27, -1);
	end
end

-- [AUTO-GEN] quest=300 status=1 n_index=800
if (EVENT == 233) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=371 status=0 n_index=1048
if (EVENT == 240) then
	SelectMsg(UID, 4, 371, 1484, NPC, 157, 241, 23, -1);
end

-- [AUTO-GEN] quest=371 status=0 n_index=1048
if (EVENT == 241) then
	SaveEvent(UID, 1049);
end

-- [AUTO-GEN] quest=371 status=1 n_index=1049
if (EVENT == 243) then
	QuestStatusCheck = GetQuestStatus(UID, 371)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1050);
		end
	end
end

-- [AUTO-GEN] quest=371 status=1 n_index=1049
if (EVENT == 244) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 371, 1484, NPC, 18, 245);
	else
		SelectMsg(UID, 4, 371, 1484, NPC, 41, 243, 27, -1);
	end
end

-- [AUTO-GEN] quest=371 status=1 n_index=1049
if (EVENT == 245) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=372 status=0 n_index=1058
if (EVENT == 250) then
	SelectMsg(UID, 4, 372, 1484, NPC, 157, 251, 23, -1);
end

-- [AUTO-GEN] quest=372 status=0 n_index=1058
if (EVENT == 251) then
	SaveEvent(UID, 1059);
end

-- [AUTO-GEN] quest=372 status=1 n_index=1059
if (EVENT == 253) then
	QuestStatusCheck = GetQuestStatus(UID, 372)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1060);
		end
	end
end

-- [AUTO-GEN] quest=372 status=1 n_index=1059
if (EVENT == 254) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 372, 1484, NPC, 18, 255);
	else
		SelectMsg(UID, 4, 372, 1484, NPC, 41, 253, 27, -1);
	end
end

-- [AUTO-GEN] quest=372 status=1 n_index=1059
if (EVENT == 255) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=373 status=0 n_index=1068
if (EVENT == 260) then
	SelectMsg(UID, 4, 373, 1484, NPC, 157, 261, 23, -1);
end

-- [AUTO-GEN] quest=373 status=0 n_index=1068
if (EVENT == 261) then
	SaveEvent(UID, 1069);
end

-- [AUTO-GEN] quest=373 status=1 n_index=1069
if (EVENT == 263) then
	QuestStatusCheck = GetQuestStatus(UID, 373)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1070);
		end
	end
end

-- [AUTO-GEN] quest=373 status=1 n_index=1069
if (EVENT == 264) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 373, 1484, NPC, 18, 265);
	else
		SelectMsg(UID, 4, 373, 1484, NPC, 41, 263, 27, -1);
	end
end

-- [AUTO-GEN] quest=373 status=1 n_index=1069
if (EVENT == 265) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=374 status=0 n_index=1078
if (EVENT == 270) then
	SelectMsg(UID, 4, 374, 1484, NPC, 157, 271, 23, -1);
end

-- [AUTO-GEN] quest=374 status=0 n_index=1078
if (EVENT == 271) then
	SaveEvent(UID, 1079);
end

-- [AUTO-GEN] quest=374 status=1 n_index=1079
if (EVENT == 273) then
	QuestStatusCheck = GetQuestStatus(UID, 374)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1080);
		end
	end
end

-- [AUTO-GEN] quest=374 status=1 n_index=1079
if (EVENT == 274) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 374, 1484, NPC, 18, 275);
	else
		SelectMsg(UID, 4, 374, 1484, NPC, 41, 273, 27, -1);
	end
end

-- [AUTO-GEN] quest=374 status=1 n_index=1079
if (EVENT == 275) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=375 status=0 n_index=1088
if (EVENT == 280) then
	SelectMsg(UID, 4, 375, 1484, NPC, 157, 281, 23, -1);
end

-- [AUTO-GEN] quest=375 status=0 n_index=1088
if (EVENT == 281) then
	SaveEvent(UID, 1089);
end

-- [AUTO-GEN] quest=375 status=1 n_index=1089
if (EVENT == 283) then
	QuestStatusCheck = GetQuestStatus(UID, 375)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1090);
		end
	end
end

-- [AUTO-GEN] quest=375 status=1 n_index=1089
if (EVENT == 284) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 375, 1484, NPC, 18, 285);
	else
		SelectMsg(UID, 4, 375, 1484, NPC, 41, 283, 27, -1);
	end
end

-- [AUTO-GEN] quest=375 status=1 n_index=1089
if (EVENT == 285) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=376 status=0 n_index=1098
if (EVENT == 290) then
	SelectMsg(UID, 4, 376, 1484, NPC, 157, 291, 23, -1);
end

-- [AUTO-GEN] quest=376 status=0 n_index=1098
if (EVENT == 291) then
	SaveEvent(UID, 1099);
end

-- [AUTO-GEN] quest=376 status=1 n_index=1099
if (EVENT == 293) then
	QuestStatusCheck = GetQuestStatus(UID, 376)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1100);
		end
	end
end

-- [AUTO-GEN] quest=376 status=1 n_index=1099
if (EVENT == 294) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 376, 1484, NPC, 18, 295);
	else
		SelectMsg(UID, 4, 376, 1484, NPC, 41, 293, 27, -1);
	end
end

-- [AUTO-GEN] quest=376 status=1 n_index=1099
if (EVENT == 295) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=377 status=0 n_index=1108
if (EVENT == 300) then
	SelectMsg(UID, 4, 377, 1484, NPC, 157, 301, 23, -1);
end

-- [AUTO-GEN] quest=377 status=0 n_index=1108
if (EVENT == 301) then
	SaveEvent(UID, 1109);
end

-- [AUTO-GEN] quest=377 status=1 n_index=1109
if (EVENT == 303) then
	QuestStatusCheck = GetQuestStatus(UID, 377)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 182)) then
			SaveEvent(UID, 1110);
		end
	end
end

-- [AUTO-GEN] quest=377 status=1 n_index=1109
if (EVENT == 304) then
	ItemA = HowmuchItem(UID, 900019000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 377, 1484, NPC, 18, 305);
	else
		SelectMsg(UID, 4, 377, 1484, NPC, 41, 303, 27, -1);
	end
end

-- [AUTO-GEN] quest=377 status=1 n_index=1109
if (EVENT == 305) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=191 status=0 n_index=1290
if (EVENT == 400) then
	SelectMsg(UID, 4, 191, 8881, NPC, 173, 401, 23, -1);
end

-- [AUTO-GEN] quest=191 status=0 n_index=1290
if (EVENT == 401) then
	SaveEvent(UID, 1291);
end

-- [AUTO-GEN] quest=191 status=1 n_index=1291
if (EVENT == 420) then
	QuestStatusCheck = GetQuestStatus(UID, 191)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 193)) then
			SaveEvent(UID, 1292);
		end
	end
end

-- [AUTO-GEN] quest=1755 status=0 n_index=14665
if (EVENT == 1200) then
	SelectMsg(UID, 4, 1755, 45524, NPC, 3577, 1201, 23, -1);
end

-- [AUTO-GEN] quest=1755 status=0 n_index=14665
if (EVENT == 1201) then
	SaveEvent(UID, 14666);
end

-- [AUTO-GEN] quest=1755 status=1 n_index=14666
if (EVENT == 1202) then
	QuestStatusCheck = GetQuestStatus(UID, 1755)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16267)) then
			SaveEvent(UID, 14667);
		end
	end
end

-- [AUTO-GEN] quest=1755 status=1 n_index=14666
if (EVENT == 1203) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1755, 45524, NPC, 22, 1202, 23, -1);
	else
		SelectMsg(UID, 2, 1755, 45524, NPC, 18, 1204);
	end
end

-- [AUTO-GEN] quest=1755 status=1 n_index=14666
if (EVENT == 1204) then
	ShowMap(UID, 71);
end


-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=1768 status=0 n_index=14755
if (EVENT == 2000) then
	SelectMsg(UID, 4, 1768, 45639, NPC, 3579, 2001, 23, -1);
end

-- [AUTO-GEN] quest=1768 status=0 n_index=14755
if (EVENT == 2001) then
	SaveEvent(UID, 14756);
end

-- [AUTO-GEN] quest=1768 status=1 n_index=14756
if (EVENT == 2004) then
	QuestStatusCheck = GetQuestStatus(UID, 1768)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16299)) then
			SaveEvent(UID, 14757);
		end
	end
end

-- [AUTO-GEN] quest=1768 status=1 n_index=14756
if (EVENT == 2005) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1768, 45639, NPC, 22, 2004, 23, -1);
	else
		SelectMsg(UID, 2, 1768, 45639, NPC, 18, 2006);
	end
end

-- [AUTO-GEN] quest=1768 status=1 n_index=14756
if (EVENT == 2006) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=1770 status=0 n_index=14765
if (EVENT == 2010) then
	SelectMsg(UID, 4, 1770, 45640, NPC, 3580, 2011, 23, -1);
end

-- [AUTO-GEN] quest=1770 status=0 n_index=14765
if (EVENT == 2011) then
	SaveEvent(UID, 14766);
end

-- [AUTO-GEN] quest=1770 status=1 n_index=14766
if (EVENT == 2014) then
	QuestStatusCheck = GetQuestStatus(UID, 1770)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16300)) then
			SaveEvent(UID, 14767);
		end
	end
end

-- [AUTO-GEN] quest=1770 status=1 n_index=14766
if (EVENT == 2015) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1770, 45640, NPC, 22, 2014, 23, -1);
	else
		SelectMsg(UID, 2, 1770, 45640, NPC, 18, 2016);
	end
end

-- [AUTO-GEN] quest=1770 status=1 n_index=14766
if (EVENT == 2016) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=1772 status=0 n_index=14775
if (EVENT == 2020) then
	SelectMsg(UID, 4, 1772, 45641, NPC, 3581, 2021, 23, -1);
end

-- [AUTO-GEN] quest=1772 status=0 n_index=14775
if (EVENT == 2021) then
	SaveEvent(UID, 14776);
end

-- [AUTO-GEN] quest=1772 status=1 n_index=14776
if (EVENT == 2024) then
	QuestStatusCheck = GetQuestStatus(UID, 1772)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16301)) then
			SaveEvent(UID, 14777);
		end
	end
end

-- [AUTO-GEN] quest=1772 status=1 n_index=14776
if (EVENT == 2025) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1772, 45641, NPC, 22, 2024, 23, -1);
	else
		SelectMsg(UID, 2, 1772, 45641, NPC, 18, 2026);
	end
end

-- [AUTO-GEN] quest=1772 status=1 n_index=14776
if (EVENT == 2026) then
	ShowMap(UID, 71);
end

-- [AUTO-GEN] quest=1774 status=0 n_index=14785
if (EVENT == 2030) then
	SelectMsg(UID, 4, 1774, 45642, NPC, 3582, 2031, 23, -1);
end

-- [AUTO-GEN] quest=1774 status=0 n_index=14785
if (EVENT == 2031) then
	SaveEvent(UID, 14786);
end

-- [AUTO-GEN] quest=1774 status=1 n_index=14786
if (EVENT == 2034) then
	QuestStatusCheck = GetQuestStatus(UID, 1774)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16302)) then
			SaveEvent(UID, 14787);
		end
	end
end

-- [AUTO-GEN] quest=1774 status=1 n_index=14786
if (EVENT == 2035) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1774, 45642, NPC, 22, 2034, 23, -1);
	else
		SelectMsg(UID, 2, 1774, 45642, NPC, 18, 2036);
	end
end

-- [AUTO-GEN] quest=1774 status=1 n_index=14786
if (EVENT == 2036) then
	ShowMap(UID, 71);
end

