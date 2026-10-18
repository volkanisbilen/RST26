#pragma once

class Packet;
class Unit;
struct _MAGIC_TABLE;
struct _MAGIC_TYPE4;

class CMagicProcess
{
public:
	static void MagicPacket(Packet& pkt, Unit* pCaster = nullptr);
	static void MagicPacketNpc(Packet& pkt, Unit* pCaster = nullptr);
	static void MagicPacketBot(Packet& pkt, Unit* pCaster = nullptr);
	static void RemoveStealth(Unit* pTarget, InvisibilityType bInvisibilityType);
	static bool UserRegionCheck(Unit* pSkillCaster, Unit* pSkillTarget, _MAGIC_TABLE pSkill, int radius, short mousex = 0, short mousez = 0);
	static bool GrantType4Buff(_MAGIC_TABLE pSkill = _MAGIC_TABLE(), _MAGIC_TYPE4* pType = nullptr, Unit* pCaster = nullptr, Unit* pTarget = nullptr, bool bIsRecastingSavedMagic = false);
	static bool RemoveType4Buff(uint8 byBuffType, Unit* pTarget, bool bRemoveSavedMagic = true, bool bRecastSavedMagic = false);
	static bool IsBuff(_MAGIC_TYPE4* pType);
};