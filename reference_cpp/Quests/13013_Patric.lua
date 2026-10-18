-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local Ret = 0;
local NPC = 13013;

if (EVENT == 165) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 166, NPC, 10, 168);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 167, NPC);
	else
		EVENT = QuestNum
	end
end

if (EVENT == 168) then
	Ret = 1;
end

if (EVENT == 170) then
	SelectMsg(UID, 2, 60, 170, NPC, 24, 171);
end

if (EVENT == 171) then
	ShowMap(UID, 4);
	SaveEvent(UID, 48);
end

if (EVENT == 172) then
	SelectMsg(UID, 2, 60, 172, NPC, 24, 171);
end

if (EVENT == 175) then
	SelectMsg(UID, 2, 60, 175, NPC,  25, 176, 13, -1);
end

if (EVENT == 176) then
	SelectMsg(UID, 4, 60, 176, NPC, 22, 178, 23, -1);
end

if (EVENT == 178) then
	SaveEvent(UID, 49);
end

if (EVENT == 180) then
	SaveEvent(UID, 51);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 60, 181, NPC, 14, 168);
	else
		SelectMsg(UID, 2, 60, 182, NPC, 14, 168);
	end
end

if (EVENT == 185) then
	MonsterCount = CountMonsterQuestSub(UID, 60, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 60, 186, NPC, 10, 188);
	else
		SelectMsg(UID, 4, 60, 187, NPC, 10, 187, 27, 168);
	end
end

if (EVENT == 188) then
	ShowMap(UID, 1);
end

if (EVENT == 187) then
	QuestStatusCheck = GetQuestStatus(UID, 60) 
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
     RunQuestExchange(UID,5);
	 SaveEvent(UID, 50);
end
end

if (EVENT == 220) then -- 3 Level Hunt Bandicoot
	SelectMsg(UID, 2, 62, 220, NPC, 24, 221, 14, 222);
end

if (EVENT == 221) then
	ShowMap(UID, 4);
	SaveEvent(UID, 63);
end

if (EVENT == 222) then
	SaveEvent(UID, 63);
end

if (EVENT == 223) then
	SelectMsg(UID, 2, 62, 223, NPC, 24, 221, 14, 222);
end

if (EVENT == 225) then
	SelectMsg(UID, 2, 62, 225, NPC,  33, 226);
end

if (EVENT == 226) then
	SelectMsg(UID, 4, 62, 228, NPC, 22, 229, 23, -1);
end

if (EVENT == 229) then
	SaveEvent(UID, 64);
end

if (EVENT == 231) then
	SaveEvent(UID, 66);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 62, 232, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 62, 233, NPC, 14, -1);
	end
end

if (EVENT == 235) then
	MonsterCount = CountMonsterQuestSub(UID, 62, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 62, 237, NPC, 18, 239);
	else
		SelectMsg(UID, 4, 62, 236, NPC, 10, 238, 27, -1);
	end
end

if (EVENT == 239) then
	ShowMap(UID, 7);
end

if (EVENT == 238) then
	QuestStatusCheck = GetQuestStatus(UID, 62) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
    RunQuestExchange(UID,7)
	SaveEvent(UID, 65);   
end
end

if (EVENT == 370) then 
	SelectMsg(UID, 2, 65, 3150, NPC, 24, 371);
end

if (EVENT == 371) then
	ShowMap(UID, 4);
	SaveEvent(UID, 3322);
end

if (EVENT == 372) then
	SelectMsg(UID, 2, 65, 3151, NPC, 24, 371);
end

if (EVENT == 300) then
	SelectMsg(UID, 2, 65, 3152, NPC, 3012, 301, 13, -1);
end

if (EVENT == 301) then
	SelectMsg(UID, 4, 65, 3153, NPC, 22, 302, 23, 308);
end

if (EVENT == 308) then
	SelectMsg(UID, 2, 65, 3155, NPC, 10, -1);
end

if (EVENT == 302) then
	SaveEvent(UID, 3323);
	SelectMsg(UID, 2, 65, 3154, NPC, 10, -1);
end

if (EVENT == 303) then
	SaveEvent(UID, 3325);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 65, 3157, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 65, 3158, NPC, 14, -1);
	end
end

if (EVENT == 305) then
	MonsterCount = CountMonsterQuestSub(UID, 65, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 65, 3156, NPC, 10, 306);
	else
		SelectMsg(UID, 5, 65, 3159, NPC, 10, 309,27, 306);
	end
end

if (EVENT == 306) then
	ShowMap(UID, 11);
end

if (EVENT == 309) then
	QuestStatusCheck = GetQuestStatus(UID, 65)
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then	
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		RunQuestExchange(UID,320,STEP,1);
		SaveEvent(UID, 3324);
end
end

if (EVENT == 470) then
	SelectMsg(UID, 2, 67, 170, NPC, 24, 471);
end

if (EVENT == 471) then
	ShowMap(UID, 4);
	SaveEvent(UID, 3332);
end

if (EVENT == 472) then
	SelectMsg(UID, 2, 67, 172, NPC, 24, 471);
end

if (EVENT == 400) then
	SelectMsg(UID, 2, 67, 3160, NPC,  3012, 401, 13, -1);
end

if (EVENT == 401) then
	SelectMsg(UID, 4, 67, 3161, NPC, 22, 402, 23, 408);
end

if (EVENT == 408) then
	SelectMsg(UID, 2, 67, 3163, NPC, 10, -1);
end

if (EVENT == 402) then
	SaveEvent(UID, 3333);
	SelectMsg(UID, 2, 67, 3162, NPC, 10, -1);
end

if (EVENT == 403) then
	SaveEvent(UID, 3335);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 1, 67, 3165, NPC, 14, -1);
	else
		SelectMsg(UID, 1, 67, 3166, NPC, 14, -1);
	end
end

if (EVENT == 405) then
	MonsterCount = CountMonsterQuestSub(UID, 67, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 67, 3164, NPC, 10, 406);
	else
		SelectMsg(UID, 5, 67, 3167, NPC, 10, 409, 27, -1);
	end
end

if (EVENT == 406) then
	ShowMap(UID, 34);
end

if (EVENT == 409) then
	QuestStatusCheck = GetQuestStatus(UID, 67) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
        RunQuestExchange(UID, 321,STEP,1);
		SaveEvent(UID, 3334);
end
end

if (EVENT == 570) then
	SelectMsg(UID, 2, 70, 170, NPC, 24, 571);
end

if (EVENT == 571) then
	ShowMap(UID, 4);
	SaveEvent(UID, 3342);
end

if (EVENT == 572) then
	SelectMsg(UID, 2, 70, 172, NPC, 24, 571);
end

if (EVENT == 500) then
	SelectMsg(UID, 2, 70, 3168, NPC,  3012, 501, 13, -1);
end

if (EVENT == 501) then
	SelectMsg(UID, 4, 70, 3169, NPC, 22, 502, 23, 508);
end

if (EVENT == 508) then
	SelectMsg(UID, 2, 70, 3171, NPC, 10, -1);
end

if (EVENT == 502) then
	SelectMsg(UID, 2, 70, 3170, NPC, 10, -1);
	SaveEvent(UID, 3343);
end

if (EVENT == 503) then
	SaveEvent(UID, 3345);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 70, 3173, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 70, 3174, NPC, 14, -1);
	end
end

if (EVENT == 505) then
	MonsterCount = CountMonsterQuestSub(UID, 70, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 70, 3172, NPC, 10, 506);
	else
		SelectMsg(UID, 5, 70, 3175, NPC, 10, 509, 27, -1);
	end
end

if (EVENT == 506) then
	ShowMap(UID, 325);
end

if (EVENT == 509) then
	QuestStatusCheck = GetQuestStatus(UID, 70)
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
        RunQuestExchange(UID,322,STEP,1);
		SaveEvent(UID, 3344);
end
end

if (EVENT == 670) then
	SelectMsg(UID, 2, 80, 3176, NPC, 3013, 671);
end

if (EVENT == 671) then
	SaveEvent(UID, 3352);
	SelectMsg(UID, 2, 80, 3182, NPC, 10, -1);
end

if (EVENT == 672) then
	SelectMsg(UID, 2, 80, 3177, NPC, 24, 671);
end

if (EVENT == 600) then
	SelectMsg(UID, 2, 80, 3193, NPC, 3017, 601);
end

if (EVENT == 601) then
	SelectMsg(UID, 4, 80, 3195, NPC, 3018, 602, 3019, 608);
end

if (EVENT == 608) then
	SelectMsg(UID, 2, 80, 3197, NPC, 10, -1);
end

if (EVENT == 602) then
	SelectMsg(UID, 2, 80, 3193, NPC, 10, -1);
	SaveEvent(UID, 3353);
end

if (EVENT == 603) then
	SaveEvent(UID, 3355);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 80, 3187, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 80, 3188, NPC, 14, -1);
	end
end

if (EVENT == 605) then
	MonsterCount = CountMonsterQuestSub(UID, 80, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 80, 3197, NPC, 10, 606);
	else
		SelectMsg(UID, 5, 80, 3189, NPC, 10, 609, 27, -1);
	end
end

if (EVENT == 606) then
	ShowMap(UID, 326);
end

if (EVENT == 609) then
	QuestStatusCheck = GetQuestStatus(UID, 80) 
	SlotCheck = CheckGiveSlot(UID, 3)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
    RunQuestExchange(UID,323,STEP,1);
	SaveEvent(UID, 3354);
end
end

if (EVENT == 770) then
	SelectMsg(UID, 2, 83, 3798, NPC, 10, 771);
end

if (EVENT == 771) then
	ShowMap(UID, 4);
	SaveEvent(UID, 3362);
end

if (EVENT == 772) then
	SelectMsg(UID, 2, 83, 3799, NPC, 10, 771);
end

if (EVENT == 700) then
	SelectMsg(UID, 2, 83, 3800, NPC,  3012, 701, 13, -1);
end

if (EVENT == 701) then
	SelectMsg(UID, 4, 83, 3801, NPC, 22, 702, 23, 708);
end

if (EVENT == 708) then
	SelectMsg(UID, 2, 83, 3803, NPC, 10, -1);
end

if (EVENT == 702) then
	SelectMsg(UID, 2, 83, 3802, NPC, 10, -1);
	SaveEvent(UID, 3363);
end

if (EVENT == 703) then
	SaveEvent(UID, 3365);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 83, 3805, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 83, 3806, NPC, 14, -1);
	end
end

if (EVENT == 705) then
	MonsterCount = CountMonsterQuestSub(UID, 83, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 83, 3804, NPC, 10, 706);
	else
		SelectMsg(UID, 5, 83, 3807, NPC, 10, 709, 27, -1);
	end
end

if (EVENT == 706) then
	ShowMap(UID, 12);
end

if (EVENT == 709) then
	QuestStatusCheck = GetQuestStatus(UID, 83) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
        RunQuestExchange(UID,324,STEP,1);
		SaveEvent(UID, 3364);
end
end

if (EVENT == 9200) then
	SelectMsg(UID, 2, 85, 175, NPC, 10, 9201);
end

if (EVENT == 9201) then
	SaveEvent(UID, 5274);
end

if (EVENT == 9202) then
	SelectMsg(UID, 2, 85, 175, NPC, 10, 9201);
end

if (EVENT == 9205) then
	SelectMsg(UID, 2, 85, 175, NPC,  25, 9206, 13, -1);
end


if (EVENT == 9206) then
	SelectMsg(UID, 4, 85, 8667, NPC, 22, 9207, 23, -1);
end

if (EVENT == 9207) then
	SaveEvent(UID, 5275);
end

if (EVENT == 9215) then
	MonsterCount = CountMonsterQuestSub(UID, 85, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 85, 8667, NPC, 10, 9216);
	else
		SelectMsg(UID, 5, 85, 8667, NPC, 10, 9217, 27, -1);
	end
end

if (EVENT == 9216) then
	ShowMap(UID, 54);
end

if (EVENT == 9217) then
	QuestStatusCheck = GetQuestStatus(UID, 85) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	RunQuestExchange(UID,1080,STEP,1);
	SaveEvent(UID, 5276);
end
end

if (EVENT == 870) then
	SelectMsg(UID, 2, 86, 170, NPC, 24, 871);
end

if (EVENT == 871) then
	ShowMap(UID, 4);
	SaveEvent(UID, 3372);
end

if (EVENT == 872) then
	SelectMsg(UID, 2, 86, 172, NPC, 24, 871);
end

if (EVENT == 800) then
	SelectMsg(UID, 2, 86, 3808, NPC,  3012, 801, 13, -1);
end

if (EVENT == 801) then
	SelectMsg(UID, 4, 86, 3809, NPC, 22, 802, 23, 808);
end

if (EVENT == 808) then
	SelectMsg(UID, 2, 86, 3811, NPC, 10, -1);
end

if (EVENT == 802) then
	SelectMsg(UID, 2, 86, 3810, NPC, 10, -1);
	SaveEvent(UID, 3373);
end

if (EVENT == 803) then
	SaveEvent(UID, 3375);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 86, 3813, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 86, 3814, NPC, 14, -1);
	end
end

if (EVENT == 805) then
	MonsterCount = CountMonsterQuestSub(UID, 86, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 86, 3812, NPC, 10, 806);
	else
		SelectMsg(UID, 5, 86, 3815, NPC, 10, 809, 27, -1);
	end
end

if (EVENT == 806) then
	ShowMap(UID, 38);
end

if (EVENT == 809) then
	QuestStatusCheck = GetQuestStatus(UID, 86)
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then	
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
    RunQuestExchange(UID,325,STEP,1);
	SaveEvent(UID, 3374);
end
end

if (EVENT == 9220) then
	SelectMsg(UID, 2, 87, 170, NPC, 10, 9221);
end

if (EVENT == 9221) then
	SaveEvent(UID, 5281);
end

if (EVENT == 9222) then
	SelectMsg(UID, 2, 87, 172, NPC, 10, 9221);
end

if (EVENT == 9225) then
	SelectMsg(UID, 2, 87, 3168, NPC, 10, 9226);
end

if (EVENT == 9226) then
	SelectMsg(UID, 4, 87, 3169, NPC, 22, 9227, 23, -1);
end

if (EVENT == 9227) then
	SelectMsg(UID, 2, 87, 3170, NPC, 14, -1);
	SaveEvent(UID, 5282);
end

if (EVENT == 9230) then
	SaveEvent(UID, 5284);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 87, 3173, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 87, 3174, NPC, 14, -1);
	end
end

if (EVENT == 9235) then
	MonsterCount = CountMonsterQuestSub(UID, 87, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 87, 3172, NPC, 10, 9236);
	else
		SelectMsg(UID, 5, 87, 3175, NPC, 10, 9239, 27, -1);
	end 
end

if (EVENT == 9236) then
	ShowMap(UID, 593);
end

if (EVENT == 9239) then
	QuestStatusCheck = GetQuestStatus(UID, 87) 
	SlotCheck = CheckGiveSlot(UID, 3)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
       RunQuestExchange(UID,1081,STEP,1);
	   SaveEvent(UID, 5283);
end
end

if (EVENT == 1070) then
	SelectMsg(UID, 2, 90, 3827, NPC, 3005, 1071);
end

if (EVENT == 1071) then
	ShowMap(UID, 4);
	SaveEvent(UID, 3392);
end

if (EVENT == 1072) then
	SelectMsg(UID, 2, 90, 3828, NPC, 3005, 1071);
end

if (EVENT == 1000) then
	SelectMsg(UID, 2, 90, 3835, NPC,  3012, 1001, 13, -1);
end

if (EVENT == 1001) then
	SelectMsg(UID, 4, 90, 3836, NPC, 22, 1002, 23, 1008);
end

if (EVENT == 1002) then
	SelectMsg(UID, 2, 90, 3838, NPC, 10, -1);
	SaveEvent(UID, 3393);
end

if (EVENT == 1008) then
	SelectMsg(UID, 2, 90, 3839, NPC, 10, -1);
end

if (EVENT == 1003) then
	SaveEvent(UID, 3395);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 90, 3841, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 90, 3842, NPC, 14, -1);
	end
end

if (EVENT == 1005) then
	MonsterCount = CountMonsterQuestSub(UID, 90, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 90, 3840, NPC, 10, 1006);
	else
		SelectMsg(UID, 5, 90, 3843, NPC, 10, 1009, 27, -1);
	end
end

if (EVENT == 1006) then
	ShowMap(UID, 629);
end

if (EVENT == 1009) then
	QuestStatusCheck = GetQuestStatus(UID, 90) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
          RunQuestExchange(UID,327,STEP,1);
		  SaveEvent(UID, 3394);
end
end

if (EVENT == 970) then 
	SelectMsg(UID, 2, 92, 170, NPC, 24, 971);
end

if (EVENT == 971) then
	ShowMap(UID, 4);
	SaveEvent(UID, 3382);
end

if (EVENT == 972) then
	SelectMsg(UID, 2, 92, 172, NPC, 24, 971);
end

if (EVENT == 900) then
	SelectMsg(UID, 2, 92, 3816, NPC,  3012, 901, 13, -1);
end

if (EVENT == 901) then
	SelectMsg(UID, 4, 92, 3817, NPC, 22, 902, 23, 908);
end

if (EVENT == 902) then
	SelectMsg(UID, 2, 92, 3818, NPC, 10, -1);
	SaveEvent(UID, 3383);
end

if (EVENT == 908) then
	SelectMsg(UID, 2, 92, 3819, NPC, 10, -1);
end

if (EVENT == 903) then
	SaveEvent(UID, 3385);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 92, 3821, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 92, 3822, NPC, 14, -1);
	end
end

if (EVENT == 905) then
	MonsterCount = CountMonsterQuestSub(UID, 92, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, 92, 3820, NPC, 10, 906);
	else
		SelectMsg(UID, 5, 92, 3823, NPC, 10, 909, 27, -1);
	end
end

if (EVENT == 906) then
	ShowMap(UID, 60);
end

if (EVENT == 909) then
	QuestStatusCheck = GetQuestStatus(UID, 92) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
        RunQuestExchange(UID,326,STEP,1);
		SaveEvent(UID, 3384);
end
end

if (EVENT == 9240) then
	SelectMsg(UID, 2, 96, 3798, NPC, 10, 9241);
end

if (EVENT == 9241) then
	SaveEvent(UID, 5288);
end

if (EVENT == 9242) then
	SelectMsg(UID, 2, 96, 3799, NPC, 10, 9241);
end

if (EVENT == 9245) then
	SelectMsg(UID, 2, 96, 3800, NPC, 10, 9246);
end

if (EVENT == 9246) then
	SelectMsg(UID, 4, 96, 3801, NPC, 22, 9247, 23, -1);
end

if (EVENT == 9247) then
	SelectMsg(UID, 2, 96, 3802, NPC, 14, -1);
	SaveEvent(UID, 5289);
end

if (EVENT == 9250) then
	SaveEvent(UID, 5291);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 96, 3805, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 96, 3806, NPC, 14, -1);
	end
end

if (EVENT == 9255) then
	MonsterCount = CountMonsterQuestSub(UID, 96, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 96, 3804, NPC, 10, 9256);
	else
		SelectMsg(UID, 4, 96, 3807, NPC, 10, 9259, 27, -1);
	end 
end

if (EVENT == 9256) then
	ShowMap(UID, 594);
end

if (EVENT == 9259) then
	QuestStatusCheck = GetQuestStatus(UID, 96) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	RunQuestExchange(UID,1082)
	SaveEvent(UID, 5290);
end
end

if (EVENT == 9431) then 
	SelectMsg(UID, 2, 97, 3798, NPC, 10, 9432);
end

if (EVENT == 9432) then
	SaveEvent(UID, 5295);
end

if (EVENT == 9433) then
	SelectMsg(UID, 2, 97, 3799, NPC, 10, 9432);
end

if (EVENT == 9434) then
	SelectMsg(UID, 2, 97, 8671, NPC, 10, 9435);
end

if (EVENT == 9435) then
	SelectMsg(UID, 4, 97, 8672, NPC, 22, 9436, 23, -1);
end

if (EVENT == 9436) then
	SelectMsg(UID, 2, 97, 3802, NPC, 14, -1);
	SaveEvent(UID, 5296);
end

if (EVENT == 9437) then
	SaveEvent(UID, 5298);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 97, 8671, NPC, 14, -1);
	else
		SelectMsg(UID, 2, 97, 8672, NPC, 14, -1);
	end
end

if (EVENT == 9439) then
	ITEMWH = HowmuchItem(UID, 810418000);
	if (ITEMWH < 5) then
		SelectMsg(UID, 2, 97, 8671, NPC, 10, 9440);
	else
		SelectMsg(UID, 4, 97, 8672, NPC, 10, 9441, 27, 9440);
	end 
end

if (EVENT == 9440) then
	ShowMap(UID, 523);
end

if (EVENT == 9441) then
	QuestStatusCheck = GetQuestStatus(UID, 97) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
    RunQuestExchange(UID,1083);
	SaveEvent(UID, 5297);
end
end

if (EVENT == 9260) then 
	SelectMsg(UID, 2, 98, 3168, NPC, 10, 9261);
end

if (EVENT == 9261) then
	SaveEvent(UID, 5302);
end

if (EVENT == 9262) then
	SelectMsg(UID, 2, 98, 3168, NPC, 10, 9261);
end   

if (EVENT == 9265) then
	SelectMsg(UID, 2, 98, 3168, NPC,  3012, 9266, 13, -1);
end

if (EVENT == 9266) then
	SelectMsg(UID, 4, 98, 3169, NPC, 22, 9267, 23, 9268);
end

if (EVENT == 9267) then
	SelectMsg(UID, 2, 98, 3170, NPC, 10, -1);
	SaveEvent(UID, 5303);
end

if (EVENT == 9268) then
	SelectMsg(UID, 2, 98, 3171, NPC, 10, -1);
end

if (EVENT == 9270) then
	SaveEvent(UID, 5305);
	SelectMsg(UID, 2, 98, 3190, NPC, 14, -1);
end

if (EVENT == 9275) then
	MonsterCount = CountMonsterQuestSub(UID, 98, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 98, 173, NPC, 10, 9276);
	else
		SelectMsg(UID, 4, 98, 3169, NPC, 10, 9277, 27, -1);
	end
end

if (EVENT == 9276) then
	ShowMap(UID, 595);
end

if (EVENT == 9277) then
	QuestStatusCheck = GetQuestStatus(UID, 98) 
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	RunQuestExchange(UID,1084);
	SaveEvent(UID, 5304);
end
end

if (EVENT == 9280) then 
	SelectMsg(UID, 2, 99, 3168, NPC, 10, 9281);
end

if (EVENT == 9281) then
	SaveEvent(UID, 5309);
end

if (EVENT == 9282) then
	SelectMsg(UID, 2, 99, 3168, NPC, 10, 9281);
end   

if (EVENT == 9285) then
	SelectMsg(UID, 2, 99, 3800, NPC,  3012, 9286, 13, -1);
end

if (EVENT == 9286) then
	SelectMsg(UID, 4, 99, 3801, NPC, 22, 9287, 23, 9288);
end

if (EVENT == 9287) then
	SelectMsg(UID, 2, 99, 3802, NPC, 10, -1);
	SaveEvent(UID, 5310);
end

if (EVENT == 9288) then
	SelectMsg(UID, 2, 99, 3803, NPC, 10, -1);
end

if (EVENT == 9290) then
	SaveEvent(UID, 5312);
	SelectMsg(UID, 2, 99, 3805, NPC, 14, -1);
end

if (EVENT == 9295) then
	MonsterCount = CountMonsterQuestSub(UID, 99, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 99, 3804, NPC, 10, 9296);
	else
		SelectMsg(UID, 5, 99, 3815, NPC, 10, 9297, 27, -1);
	end
end

if (EVENT == 9296) then
	ShowMap(UID, 596);
end

if (EVENT == 9297) then
	QuestStatusCheck = GetQuestStatus(UID, 99) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	RunQuestExchange(UID,1085,STEP,1);
	SaveEvent(UID, 5311);
end
end

local savenum = 111;

if (EVENT == 8630) then
	SelectMsg(UID, 2, savenum, 8090, NPC, 10, 8631);
end

if (EVENT == 8631) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8145);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8150);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8155);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8160);
	end
end

if (EVENT == 8632) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8090, NPC, 22, 8633, 23, 8634);
	else
		SelectMsg(UID, 2, savenum, 8090, NPC, 10, 168);
	end
end

if (EVENT == 8633) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8146);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8151);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8156);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8161);
	end
end

if (EVENT == 8634) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8149);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8154);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8159);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8164);
	end
end

if (EVENT == 8640) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8148);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8153);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8158);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8163);
	end
end

if (EVENT == 8636) then
	MonsterCount = CountMonsterQuestSub(UID, 111, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8090, NPC, 21, 8637);
	else
		SelectMsg(UID, 5, savenum, 8090, NPC, 41, 8638, 23, -1);
	end
end

if (EVENT == 8637) then
	ShowMap(UID, 597);
end

if (EVENT == 8638) then
	QuestStatusCheck = GetQuestStatus(UID, 111) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then 
    RunQuestExchange(UID,840,STEP,1);
		SaveEvent(UID, 8147);
	elseif (Class == 2 or Class == 7 or Class == 8) then 
    RunQuestExchange(UID,841,STEP,1);
		SaveEvent(UID, 8152);
	elseif (Class == 3 or Class == 9 or Class == 10) then 
    RunQuestExchange(UID,842,STEP,1);
		SaveEvent(UID, 8157);
	elseif (Class == 4 or Class == 11 or Class == 12) then 
    RunQuestExchange(UID,843,STEP,1);
		SaveEvent(UID, 8162);
end
end
end

if (EVENT == 1101) then
	SelectMsg(UID, 2, -1, 43652, NPC, 10, -1);
	GiveItem(UID, 900600000,1);
	SaveEvent(UID, 7336);
end