-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC =13015;

if (EVENT == 165) then
Level = CheckLevel(UID)
if (Level > 34 and Level < 60) then---35 levelden 59 level arası
        SelectMsg(UID, 2, -1, 4134, NPC, 4075, 180, 4076, -1);
elseif (Level > 59 and Level < 84) then---60 levelden 83 level arası
        SelectMsg(UID, 2, -1, 4133, NPC, 4075, 190, 4076, -1); 
else
        SelectMsg(UID, 2, -1, 4135, NPC, 10, -1);
end
end

if (EVENT == 180) then----1 Level ile 59 Level Arası Forgetten Temple
Time = CheckMonsterChallengeTime(UID)
if (Time == 9) then---saaat 9 ise
SelectMsg(UID, 2, -1, 4133, NPC, 4075, 181, 4076, -1); 
elseif (Time == 19) then----saat 19 ise
SelectMsg(UID, 2, -1, 4133, NPC, 4075, 182, 4076, -1); 
else--Zamanı gelmedi ise
SelectMsg(UID, 2, -1, 4140, NPC, 10, -1);
end
end

if EVENT == 181 then --1 ile 59 Arası Forgetten Temple Saat 9 ise
      Count = CheckMonsterChallengeUserCount(UID)
      if (Count < 33) then
         ItemA = HowmuchItem(UID, 900000000); 
         if (ItemA > 100000) then
            GoldLose(UID, 100000)
            ZoneChange(UID, 55, 150, 150)
         else
            SelectMsg(UID, 2, -1, 4136, NPC, 10, -1);
         end
      else
         SelectMsg(UID, 2, -1, 4137, NPC, 10, -1);
   end
end

if EVENT == 182 then --1 ile 59 Arası Forgetten Temple Saat 19 ise
      Count = CheckMonsterChallengeUserCount(UID)
      if (Count < 33) then
         ItemA = HowmuchItem(UID, 900000000); 
         if (ItemA > 100000) then
            GoldLose(UID, 100000)
            ZoneChange(UID, 55, 150, 150)
         else
            SelectMsg(UID, 2, -1, 4136, NPC, 10, -1);
         end
      else
         SelectMsg(UID, 2, -1, 4137, NPC, 10, -1);
   end
end


if (EVENT == 190) then----60 Level ile 83 Level Arası Forgetten Temple
Time = CheckMonsterChallengeTime(UID)
if (Time == 3) then---saaat 3 ise
SelectMsg(UID, 2, -1, 4133, NPC, 4075, 191, 4076, -1); 
elseif (Time == 22) then----saat 22 ise
SelectMsg(UID, 2, -1, 4133, NPC, 4075, 192, 4076, -1); 
else--Zamanı gelmedi ise
SelectMsg(UID, 2, -1, 4138, NPC, 10, -1);
end
end


if EVENT == 191 then --60 ile 83 Arası Forgetten Temple Saat 3 ise
      Count = CheckMonsterChallengeUserCount(UID)
      if (Count < 33) then
         ItemA = HowmuchItem(UID, 900000000); 
         if (ItemA > 100000) then
            GoldLose(UID, 100000)
            ZoneChange(UID, 55, 150, 150)
         else
            SelectMsg(UID, 2, -1, 4136, NPC, 10, -1);
         end
      else
         SelectMsg(UID, 2, -1, 4137, NPC, 10, -1);
   end
end

if EVENT == 192 then --60 ile 83 Arası Forgetten Temple Saat 22 ise
      Count = CheckMonsterChallengeUserCount(UID)
      if (Count < 33) then
         ItemA = HowmuchItem(UID, 900000000); 
         if (ItemA > 100000) then
            GoldLose(UID, 100000)
            ZoneChange(UID, 55, 150, 150)
         else
            SelectMsg(UID, 2, -1, 4136, NPC, 10, -1);
         end
      else
         SelectMsg(UID, 2, -1, 4137, NPC, 10, -1);
   end
end
