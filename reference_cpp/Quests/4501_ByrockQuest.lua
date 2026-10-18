local UserClass;

local QuestNum;

local Ret = 0;

local NPC =4501;





if EVENT == 500 then

   SelectMsg(UID, 3, -1, 45311, NPC, 45543, 143,45544, 144,45545, 145,45546, 146,45547, 147,45548, 148,45549, 149,45550, 150,45551, 151,45552, 152,45553, 153); 

end

--45310 yazan ilk açılışta bilgilendirme mesajı örnek olarak buraya bilgil vermek istediğimiz olayı yazabiliriz.
--45539 ise quest_menu_us tbl de bulunanlar butonlar 

if EVENT == 101 then

	SelectMsg(UID, 3, -1, 4834, NPC, 4262, 101);

end

if (EVENT == 143) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 100 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 100);
	GiveItem(UID, 800605000, 1); -- infarrow
		end
	end
end
if (EVENT == 144) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 100 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 100);
	GiveItem(UID, 750680000, 1); --otoloot
		end
	end
end
if (EVENT == 145) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 200 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 200);
	GiveItem(UID, 800440000, 1); -- extrainv
		end
	end
end
if (EVENT == 146) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 300 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 300);
	GiveItem(UID, 800610000, 1); -- otomingn
		end
	end
end
if (EVENT == 147) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 600 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 600);
	GiveItem(UID, 511000000, 1); -- otoloot ve mining
		end
	end
end
if (EVENT == 148) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 700 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 700);
	GiveItem(UID, 379258000, 1); -- karivdis
		end
	end
end
if (EVENT == 149) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 700 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 700);
	GiveItem(UID, 700002000, 1); -- trina
		end
	end
end
if (EVENT == 150) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 700 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 700);
	GiveItem(UID, 354000000, 1); -- accesorytrina
		end
	end
end
if (EVENT == 151) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 800 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 800);
	GiveItem(UID, 800387000, 1); -- peri
		end
	end
end
if (EVENT == 152) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 800 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 800);
	GiveItem(UID, 811164000, 1); -- warwing
		end
	end
end
if (EVENT == 153) then
	QUESTITEM = HowmuchItem(UID, 389960000);
	if (QUESTITEM < 800 or QUESTITEM == 0) then
		SelectMsg(UID, 2, -1, 45313, NPC, 45554, 5000);
	else
	SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
    else   
	RobItem(UID, 389960000, 800);
	GiveItem(UID, 810926000, 1); -- tattoo
		end
	end
end

if (EVENT == 5000) then
	SelectMsg(UID, 2, -1, 45312, NPC, 45555, -1);
end


