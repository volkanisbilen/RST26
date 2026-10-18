local UserClass;

local QuestNum;

local Ret = 0;

local NPC =4500;





if EVENT == 500 then

   SelectMsg(UID, 3, -1, 45310, NPC, 45539, 101,45540, 102,45541, 103,45542, 104); 
end

--45310 yazan ilk açılışta bilgilendirme mesajı örnek olarak buraya bilgil vermek istediğimiz olayı yazabiliriz.
--45539 ise quest_menu_us tbl de bulunanlar butonlar

if EVENT == 101 then

	SelectMsg(UID, 3, -1, 45319, NPC, 45555, -1,45561,500);

end
if EVENT == 102 then

	SelectMsg(UID, 3, -1, 45321, NPC, 45555, -1,45561,500);

end

if EVENT == 103 then

	SelectMsg(UID, 3, -1, 45320, NPC, 45555, -1,45561,500);

end
if EVENT == 104 then

	SelectMsg(UID, 3, -1, 45322, NPC, 45555, -1,45561,500);

end

