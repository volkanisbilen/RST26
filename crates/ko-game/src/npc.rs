//! NPC/Monster runtime types and packet builders.
//! - `GameServer/Npc.h` — CNpc class (runtime NPC instance)
//! - `GameServer/NpcTable.h` — CNpcTable (static template data)
//! - `GameServer/Npc.cpp:155-164` — GetInOut()
//! - `GameServer/Npc.cpp:246-401` — GetNpcInfo()

use ko_protocol::{Opcode, Packet};

/// NPC appearing in a region.
pub const NPC_IN: u8 = 0x01;
/// NPC disappearing from a region.
pub const NPC_OUT: u8 = 0x02;

/// Starting runtime ID for NPC instances.
/// NPC runtime IDs start at this value to avoid collision with session IDs.
pub const NPC_BAND: u32 = 10000;

/// NPC type sent for non-monster NPCs in GetNpcInfo.
/// original server sends actual npc_type. Kept only for test/doc reference.
#[allow(dead_code)]
const NPC_SCARECROW: u8 = 171;

/// Unique runtime identifier for an NPC instance.
pub type NpcId = u32;

/// Static NPC/Monster template loaded from the database.
/// Contains all immutable stats and appearance data for an NPC type.
/// Multiple NPC instances may share the same template.
#[derive(Debug, Clone)]
pub struct NpcTemplate {
    /// Template ID (K_NPC/K_MONSTER sSid).
    pub s_sid: u16,
    /// True if this is a monster, false if NPC.
    pub is_monster: bool,
    /// Display name.
    pub name: String,
    /// Picture/model ID for the client.
    pub pid: u16,
    /// Size multiplier (100 = normal).
    pub size: u16,
    /// Primary weapon model ID.
    pub weapon_1: u32,
    /// Secondary weapon model ID.
    pub weapon_2: u32,
    /// Nation/group (0=neutral, 1=Karus, 2=Elmorad).
    pub group: u8,
    /// Action type (0=passive/tender, 1=aggressive/atrocity).
    pub act_type: u8,
    /// NPC functional type (0=monster, 1=gate, 2=guard, etc.).
    pub npc_type: u8,
    /// Family type for group AI behavior.
    pub family_type: u8,
    /// Selling group for vendor NPCs.
    pub selling_group: u32,
    /// Level.
    pub level: u8,
    /// Maximum HP.
    pub max_hp: u32,
    /// Maximum MP.
    pub max_mp: u16,
    /// Attack power.
    pub attack: u16,
    /// Armor class (defense).
    pub ac: u16,
    /// Hit rate.
    ///
    pub hit_rate: u16,
    /// Evasion rate.
    ///
    pub evade_rate: u16,
    /// Damage value.
    ///
    pub damage: u16,
    /// Attack delay in milliseconds.
    ///
    pub attack_delay: u16,
    /// Random movement speed.
    ///
    pub speed_1: u16,
    /// Chase/attack movement speed.
    ///
    pub speed_2: u16,
    /// Stand time (idle delay between actions) in milliseconds.
    ///
    pub stand_time: u16,
    /// Search range for enemy detection.
    ///
    pub search_range: u8,
    /// Attack range (melee distance).
    ///
    pub attack_range: u8,
    /// Direct attack type: 0=melee, 1=long-range, 2=magic.
    ///
    pub direct_attack: u8,
    /// Tracing range (extended chase range).
    ///
    pub tracing_range: u8,
    /// Primary magic skill ID.
    ///
    pub magic_1: u32,
    /// Secondary magic skill ID.
    ///
    pub magic_2: u32,
    /// Healing magic skill ID (used by healer NPCs).
    ///
    pub magic_3: u32,
    /// Magic attack type — controls which class magic the NPC uses in combat.
    ///
    /// Values: 0=none, 2=melee+magic, 3=boss special, 4/5=magic primary, 6=heavy magic
    pub magic_attack: u8,
    /// Fire resistance.
    ///
    pub fire_r: i16,
    /// Cold/ice resistance.
    ///
    pub cold_r: i16,
    /// Lightning resistance.
    ///
    pub lightning_r: i16,
    /// Light magic resistance.
    ///
    pub magic_r: i16,
    /// Disease/curse resistance.
    ///
    pub disease_r: i16,
    /// Poison resistance.
    ///
    pub poison_r: i16,
    /// Experience points awarded on kill.
    ///
    pub exp: u32,
    /// Loyalty (nation points) awarded on kill.
    ///
    pub loyalty: u32,
    /// Gold amount dropped on kill.
    ///
    pub money: u32,
    /// Item drop table index (references monster_item/npc_item table).
    ///
    pub item_table: i16,
    /// Area range for proximity effects (e.g. Santa death rewards).
    ///
    pub area_range: f32,
}

/// Runtime NPC instance in the game world.
/// Each instance represents a single NPC or monster placed in a zone.
/// Multiple instances may share the same `NpcTemplate` (via `proto_id`).
#[derive(Debug, Clone)]
pub struct NpcInstance {
    /// Unique runtime ID (starts at NPC_BAND).
    pub nid: NpcId,
    /// Template ID — references `NpcTemplate.s_sid`.
    pub proto_id: u16,
    /// True if this is a monster, false if NPC.
    pub is_monster: bool,
    /// Zone this NPC belongs to.
    pub zone_id: u16,
    /// World X coordinate.
    pub x: f32,
    /// World Y coordinate (height).
    pub y: f32,
    /// World Z coordinate.
    pub z: f32,
    /// Facing direction byte sent to the client.
    pub direction: u8,
    /// Region grid X index.
    pub region_x: u16,
    /// Region grid Z index.
    pub region_z: u16,
    /// Gate state: 0=closed, 1=open, 2=open (for gate NPCs).
    pub gate_open: u8,
    /// Object type: 0=normal, 1=special.
    pub object_type: u8,
    /// Nation: 0=neutral for monsters, group value for NPCs.
    pub nation: u8,
    /// Special type from spawn data (e.g., 7 = CycleSpawn).
    ///
    pub special_type: i16,
    /// Trap number for CycleSpawn NPCs (1-4).
    ///
    pub trap_number: i16,
    /// Event room ID (1-based). 0 = not in any event room.
    ///
    /// Monster Stone rooms use `room_id + 1` (1-based).
    pub event_room: u16,
    /// Whether this NPC was spawned as an event NPC.
    ///
    /// Event NPCs get `m_bDelete = true` on death for cleanup.
    pub is_event_npc: bool,
    /// Summon type for event NPCs.
    ///
    /// 0 = none, 1 = Monster Stone boss, 2 = Juraid monster, 3 = FT monster.
    pub summon_type: u8,

    // ── Type 15 (barracks/pet) fields ─────────────────────────────
    /// Owner/user name for type 15 pet NPCs.
    ///
    pub user_name: String,
    /// Pet name for type 15 pet NPCs.
    ///
    pub pet_name: String,
    /// Clan name for type 15 proto 511 barracks NPCs.
    ///
    pub clan_name: String,
    /// Clan ID for type 15 proto 511 barracks NPCs.
    ///
    pub clan_id: u16,
    /// Clan mark version for type 15 proto 511 barracks NPCs.
    ///
    pub clan_mark_version: u16,
}

/// Build a `WIZ_NPC_INOUT` (0x0A) packet.
/// Wire format:
/// ```text
/// [u8 type] [u32 npcId] [GetNpcInfo if type != OUT]
/// ```
pub fn build_npc_inout(inout_type: u8, npc: &NpcInstance, template: &NpcTemplate) -> Packet {
    let mut pkt = Packet::new(Opcode::WizNpcInout as u8);
    pkt.write_u8(inout_type);
    pkt.write_u32(npc.nid);

    if inout_type != NPC_OUT {
        write_npc_info(&mut pkt, npc, template);
    }

    pkt
}

/// Write the GetNpcInfo block into a packet.
/// Dispatches to type-specific serialization for type 15 and type 191 NPCs.
/// All other NPCs use the default 43-byte format.
pub fn write_npc_info(pkt: &mut Packet, npc: &NpcInstance, tmpl: &NpcTemplate) {
    if is_native_moraranker_template(tmpl) {
        write_npc_info_ranker(pkt, npc, tmpl);
        return;
    }

    write_npc_info_base(pkt, npc, tmpl);
}

/// Write the fixed-size/base GetNpcInfo block used inside multi-NPC lists.
///
/// `WIZ_REQ_NPCIN` has no per-NPC length marker, so native MORANKER character
/// extension bytes must not be written there or subsequent NPCs are parsed at
/// the wrong offset by the client.
pub fn write_npc_info_base(pkt: &mut Packet, npc: &NpcInstance, tmpl: &NpcTemplate) {
    match tmpl.npc_type {
        15 => write_npc_info_type15(pkt, npc, tmpl),
        191 => write_npc_info_type191(pkt, npc, tmpl),
        _ => write_npc_info_default(pkt, npc, tmpl),
    }
}

pub fn is_native_moraranker_template(tmpl: &NpcTemplate) -> bool {
    (31_882..=31_887).contains(&tmpl.s_sid) && (b'R'..=b'W').contains(&tmpl.npc_type)
}

/// Write the v2615 native MORANKER extension for NPC types R..W (82..=87).
///
/// The unpacked client reads a normal GetNpcInfo block followed by a character
/// name and the appearance values below. Ranker templates are runtime-only and
/// intentionally store these values in otherwise-unused template fields:
/// group=race, attack=class, act_type=face, exp=hair RGB, magic1..3=body parts,
/// selling_group=gloves and money=boots. The two weapon fields retain their
/// normal meaning.
fn write_npc_info_ranker(pkt: &mut Packet, npc: &NpcInstance, tmpl: &NpcTemplate) {
    write_npc_info_default(pkt, npc, tmpl);
    pkt.write_string(&tmpl.name);
    pkt.write_u8(tmpl.group);
    pkt.write_u16(tmpl.attack);
    pkt.write_u8(tmpl.act_type);
    pkt.write_u32(tmpl.exp);
    pkt.write_u32(tmpl.magic_1); // chest
    pkt.write_u32(tmpl.magic_2); // helmet
    pkt.write_u32(tmpl.magic_3); // trousers
    pkt.write_u32(tmpl.selling_group); // gloves
    pkt.write_u32(tmpl.money); // boots
    pkt.write_u32(tmpl.weapon_1); // right hand
    pkt.write_u32(tmpl.weapon_2); // left hand
}

/// Write GetNpcInfo for **type 15** (barracks / pets).
/// - Proto 511 (Barracks): includes clan name + NPC name strings, clan ID + mark version.
/// - Other type 15 (Pets): SByte mode, includes user name + pet name strings.
fn write_npc_info_type15(pkt: &mut Packet, npc: &NpcInstance, tmpl: &NpcTemplate) {
    if tmpl.s_sid == 511 {
        // === Type 15, Proto 511: Barracks ===
        pkt.write_u16(tmpl.s_sid);
        pkt.write_u8(2); // always NPC
        pkt.write_u16(tmpl.pid);
        pkt.write_u32(tmpl.selling_group);
        pkt.write_u8(15); // actual type, NOT NPC_SCARECROW
        pkt.write_u32(0);
        pkt.write_u16(tmpl.size);
        pkt.write_u32(tmpl.weapon_1);
        pkt.write_u32(tmpl.weapon_2);
        // Clan name (empty if no clan) — C++ uses Packet << string (u16 length prefix)
        pkt.write_string(&npc.clan_name);
        // NPC name — C++ uses Packet << GetName()
        pkt.write_string(&tmpl.name);
        pkt.write_u8(npc.nation);
        pkt.write_u8(tmpl.level);
        pkt.write_u16((npc.x * 10.0) as u16);
        pkt.write_u16((npc.z * 10.0) as u16);
        pkt.write_u16(0);
        pkt.write_u32(npc.gate_open as u32);
        pkt.write_u8(npc.object_type);
        // If clan exists: clan ID + mark version; else: u16(0) + u16(0)
        pkt.write_u16(npc.clan_id);
        pkt.write_u16(npc.clan_mark_version);
        pkt.write_i16(npc.direction as i16);
    } else {
        // === Type 15, non-511: Pets/Summons ===
        //
        // C++ calls `pkt.SByte()` which sets mode to u8-length-prefix for strings.
        // SByte() writes NOTHING — it only sets the mode flag (m_doubleByte=false).
        // Numeric writes (u16, u8, u32) are NOT affected by SByte mode.
        // 2614/2615 client requires the real template identity.
        // SID=0 creates the pet UI/NID but does not create a render object.
        pkt.write_u16(tmpl.s_sid);
        pkt.write_u8(2); // NPC/type-15 pet
        pkt.write_u16(tmpl.pid);
        pkt.write_u32(0x00);
        pkt.write_u8(15); // actual type
        pkt.write_u32(0);
        pkt.write_u16(tmpl.size);
        pkt.write_u32(0x00); // weapon1 = 0 for pets
        pkt.write_u32(0x00); // weapon2 = 0 for pets
                             // Owner name + Pet name — SByte mode: u8 length prefix (NOT u16)
        pkt.write_sbyte_string(&npc.user_name);
        pkt.write_sbyte_string(&npc.pet_name);
        // The 2625 client reads one more u32 here for type 15 (NPC-in parser, after the pet name
        // string); 0 means "no skin". Without it every later field shifts by 4 bytes and the pet
        // is created at the map origin with a wrong nation/level.
        pkt.write_u32(0);
        pkt.write_u8(npc.nation);
        pkt.write_u8(tmpl.level);
        pkt.write_u16((npc.x * 10.0) as u16);
        pkt.write_u16((npc.z * 10.0) as u16);
        pkt.write_u16(0);
        pkt.write_u32(0x00); // gate_open = 0
        pkt.write_u8(0x00); // object_type = 0
        pkt.write_u16(0x00);
        pkt.write_u16(0x00);
        pkt.write_i16(npc.direction as i16);
    }
}

/// Write GetNpcInfo for **type 191** (Guard Towers).
/// Same binary layout as default (43 bytes) but always sends:
/// - `uint8(2)` for isMonster flag (always NPC)
/// - `GetType()` (191) instead of NPC_SCARECROW
fn write_npc_info_type191(pkt: &mut Packet, npc: &NpcInstance, tmpl: &NpcTemplate) {
    pkt.write_u16(tmpl.s_sid);
    pkt.write_u8(2); // always NPC
    pkt.write_u16(tmpl.pid);
    pkt.write_u32(tmpl.selling_group);
    pkt.write_u8(191); // actual type, NOT NPC_SCARECROW
    pkt.write_u32(0);
    pkt.write_u16(tmpl.size);
    pkt.write_u32(tmpl.weapon_1);
    pkt.write_u32(tmpl.weapon_2);
    pkt.write_u8(npc.nation);
    pkt.write_u8(tmpl.level);
    pkt.write_u16((npc.x * 10.0) as u16);
    pkt.write_u16((npc.z * 10.0) as u16);
    pkt.write_u16(0);
    pkt.write_u32(npc.gate_open as u32);
    pkt.write_u8(npc.object_type);
    pkt.write_u16(0);
    pkt.write_u16(0);
    pkt.write_i16(npc.direction as i16);
}

/// Guard summon proto ID (`Define.h:481` — `#define GUARD_SUMMON 8850`).
/// Guard summons are special monster NPCs that retain their nation in packets,
/// unlike regular monsters which always send nation=0.
const GUARD_SUMMON: u16 = 8850;

use crate::npc_type_constants::{NPC_DESTROYED_ARTIFACT, NPC_GATE};

/// Check if an NPC is a CSW door.
/// CSW doors are proto IDs 561, 562, 563 with NPC_GATE type.
fn is_csw_door(proto_id: u16, npc_type: u8) -> bool {
    matches!(proto_id, 561..=563) && npc_type == NPC_GATE
}

/// Write GetNpcInfo for **default** NPC types (all except 15 and 191).
/// Wire format (43 bytes):
/// ```text
/// [u16 protoId] [u8 1=monster/2=npc] [u16 pictureId] [u32 sellingGroup]
/// [u8 npcType] [u32 0] [u16 size] [u32 weapon1] [u32 weapon2]
/// [u8 nation] [u8 level] [u16 posX*10] [u16 posZ*10]
/// [u16 0] [u32 gateOpen] [u8 objectType] [u16 0] [u16 0] [i16 direction]
/// ```
fn write_npc_info_default(pkt: &mut Packet, npc: &NpcInstance, tmpl: &NpcTemplate) {
    // Proto ID
    pkt.write_u16(tmpl.s_sid);

    // isMonster: 1 = monster, 2 = NPC
    pkt.write_u8(if tmpl.is_monster { 1 } else { 2 });

    // Picture ID
    pkt.write_u16(tmpl.pid);

    // Selling group
    pkt.write_u32(tmpl.selling_group);

    // NPC type — sniffer verified: original server sends actual npc_type for ALL NPCs.
    //   guards=11, merchants=22, event=46, blacksmith=77, innhost=31, etc.
    // The npc_type determines client interaction menu (shop, quest, dialog, etc.)
    //
    pkt.write_u8(tmpl.npc_type);

    // Reserved u32
    pkt.write_u32(0);

    // Size
    pkt.write_u16(tmpl.size);

    // Weapons
    pkt.write_u32(tmpl.weapon_1);
    pkt.write_u32(tmpl.weapon_2);

    // Nation determination
    //
    // ```cpp
    // if (isGuardSummon())
    //     bNation = GetNation();
    // else
    //     bNation = uint8(isMonster() ? 0 : GetNation());
    //
    // if ((GetType() == NPC_DESTROYED_ARTIFACT || isCswDoors()))
    // {
    //     bNation = 0;
    //     if (userClanID && g_pMain->pSiegeWar.sMasterKnights == userClanID)
    //         bNation = 3;
    // }
    // ```
    //
    // Guard summons (proto 8850) retain their nation even though they are monsters.
    // Regular monsters always send nation=0. NPCs keep their template nation.
    // CSW doors / destroyed artifacts override nation to 0 (or 3 for siege master).
    let nation =
        if tmpl.npc_type == NPC_DESTROYED_ARTIFACT || is_csw_door(tmpl.s_sid, tmpl.npc_type) {
            // CSW override: nation=0 in broadcast path (userClanID=0).
            // Per-user packets in req_npcin would need user_clan_id for nation=3.
            0u8
        } else if tmpl.s_sid == GUARD_SUMMON {
            // Guard summons keep their nation even as monsters: the caster's nation when
            // summoned by a skill, otherwise the template group.
            if npc.nation != 0 {
                npc.nation
            } else {
                tmpl.group
            }
        } else if tmpl.is_monster {
            // Regular monsters always send nation=0.
            0u8
        } else {
            // NPCs keep their nation.
            npc.nation
        };

    pkt.write_u8(nation);

    // Level
    pkt.write_u8(tmpl.level);

    // Position: GetSPosX/Z = uint16(pos * 10)
    pkt.write_u16((npc.x * 10.0) as u16);
    pkt.write_u16((npc.z * 10.0) as u16);

    // Reserved u16
    pkt.write_u16(0);

    // Gate open status
    pkt.write_u32(npc.gate_open as u32);

    // Object type
    pkt.write_u8(npc.object_type);

    // Reserved u16, u16
    pkt.write_u16(0);
    pkt.write_u16(0);

    // IDA-verified: direction(u8) + nation2(u8) — two separate fields
    pkt.write_u8(npc.direction as u8);
    pkt.write_u8(nation); // nation2 — used for NPC color comparison with player nation
}

#[cfg(test)]
mod tests {
    use super::*;
    use ko_protocol::PacketReader;

    /// Create a test NPC template.
    fn test_template() -> NpcTemplate {
        NpcTemplate {
            s_sid: 150,
            is_monster: true,
            name: "Kecoon".to_string(),
            pid: 150,
            size: 100,
            weapon_1: 0,
            weapon_2: 0,
            group: 0,
            act_type: 1,
            npc_type: 0,
            family_type: 0,
            selling_group: 0,
            level: 80,
            max_hp: 1000,
            max_mp: 0,
            attack: 1,
            ac: 0,
            hit_rate: 100,
            evade_rate: 50,
            damage: 10,
            attack_delay: 1500,
            speed_1: 1000,
            speed_2: 500,
            stand_time: 3000,
            search_range: 30,
            attack_range: 3,
            direct_attack: 0,
            tracing_range: 30,
            magic_1: 0,
            magic_2: 0,
            magic_3: 0,
            magic_attack: 0,
            fire_r: 0,
            cold_r: 0,
            lightning_r: 0,
            magic_r: 0,
            disease_r: 0,
            poison_r: 0,
            exp: 100,
            loyalty: 0,
            money: 0,
            item_table: 0,
            area_range: 0.0,
        }
    }

    /// Create a test NPC instance.
    fn test_instance() -> NpcInstance {
        NpcInstance {
            nid: 10001,
            proto_id: 150,
            is_monster: true,
            zone_id: 21,
            x: 616.0,
            y: 0.0,
            z: 341.0,
            direction: 0,
            region_x: 12,
            region_z: 7,
            gate_open: 0,
            object_type: 0,
            nation: 0,
            special_type: 0,
            trap_number: 0,
            event_room: 0,
            is_event_npc: false,
            summon_type: 0,
            user_name: String::new(),
            pet_name: String::new(),
            clan_name: String::new(),
            clan_id: 0,
            clan_mark_version: 0,
        }
    }

    #[test]
    fn test_npc_inout_in_packet_format() {
        let tmpl = test_template();
        let npc = test_instance();
        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);

        assert_eq!(pkt.opcode, Opcode::WizNpcInout as u8);

        let mut r = PacketReader::new(&pkt.data);

        // Type
        assert_eq!(r.read_u8(), Some(NPC_IN));
        // NPC ID
        assert_eq!(r.read_u32(), Some(10001));
        // GetNpcInfo (43 bytes)
        // Proto ID
        assert_eq!(r.read_u16(), Some(150));
        // isMonster
        assert_eq!(r.read_u8(), Some(1));
        // Picture ID
        assert_eq!(r.read_u16(), Some(150));
        // Selling group
        assert_eq!(r.read_u32(), Some(0));
        // NPC type
        assert_eq!(r.read_u8(), Some(0));
        // Reserved
        assert_eq!(r.read_u32(), Some(0));
        // Size
        assert_eq!(r.read_u16(), Some(100));
        // Weapons
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        // Nation
        assert_eq!(r.read_u8(), Some(0));
        // Level
        assert_eq!(r.read_u8(), Some(80));
        // Position: 616.0 * 10 = 6160, 341.0 * 10 = 3410
        assert_eq!(r.read_u16(), Some(6160));
        assert_eq!(r.read_u16(), Some(3410));
        // Reserved
        assert_eq!(r.read_u16(), Some(0));
        // Gate open
        assert_eq!(r.read_u32(), Some(0));
        // Object type
        assert_eq!(r.read_u8(), Some(0));
        // Reserved
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        // Direction + nation2
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u8(), Some(0));

        // Should have consumed all data (5 + 43 = 48 bytes)
        assert_eq!(r.remaining(), 0);
        assert_eq!(pkt.data.len(), 48);
    }

    #[test]
    fn test_npc_inout_out_packet_format() {
        let tmpl = test_template();
        let npc = test_instance();
        let pkt = build_npc_inout(NPC_OUT, &npc, &tmpl);

        assert_eq!(pkt.opcode, Opcode::WizNpcInout as u8);

        let mut r = PacketReader::new(&pkt.data);
        assert_eq!(r.read_u8(), Some(NPC_OUT));
        assert_eq!(r.read_u32(), Some(10001));
        // No GetNpcInfo for OUT
        assert_eq!(r.remaining(), 0);
        assert_eq!(pkt.data.len(), 5);
    }

    #[test]
    fn test_default_npc_preserves_byte_direction() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 7;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 1;
        npc.direction = 130;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);

        assert_eq!(pkt.data.len(), 48);
        assert_eq!(pkt.data[46], 130);
        assert_eq!(pkt.data[47], 1);
    }

    #[test]
    fn test_native_ranker_extension_packet_format() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.s_sid = 31_882;
        tmpl.name = "RankOne".to_string();
        tmpl.npc_type = b'R';
        tmpl.group = 12; // race
        tmpl.attack = 106; // class
        tmpl.act_type = 3; // face
        tmpl.exp = 0x0011_2233; // hair RGB
        tmpl.magic_1 = 210_000_000; // chest
        tmpl.magic_2 = 210_001_000; // helmet
        tmpl.magic_3 = 210_002_000; // trousers
        tmpl.selling_group = 210_003_000; // gloves
        tmpl.money = 210_004_000; // boots
        tmpl.weapon_1 = 180_000_000;
        tmpl.weapon_2 = 170_000_000;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 1;
        npc.direction = 130;
        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);

        assert_eq!(pkt.data[46], 130);
        assert_eq!(pkt.data[47], 1);

        let mut reader = PacketReader::new(&pkt.data[48..]);
        assert_eq!(reader.read_string().as_deref(), Some("RankOne"));
        assert_eq!(reader.read_u8(), Some(12));
        assert_eq!(reader.read_u16(), Some(106));
        assert_eq!(reader.read_u8(), Some(3));
        assert_eq!(reader.read_u32(), Some(0x0011_2233));
        assert_eq!(reader.read_u32(), Some(210_000_000));
        assert_eq!(reader.read_u32(), Some(210_001_000));
        assert_eq!(reader.read_u32(), Some(210_002_000));
        assert_eq!(reader.read_u32(), Some(210_003_000));
        assert_eq!(reader.read_u32(), Some(210_004_000));
        assert_eq!(reader.read_u32(), Some(180_000_000));
        assert_eq!(reader.read_u32(), Some(170_000_000));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_native_ranker_base_info_stays_fixed_size_for_req_npcin() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.s_sid = 31_882;
        tmpl.npc_type = b'R';

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 1;
        npc.direction = 130;

        let mut pkt = Packet::new(Opcode::WizReqNpcIn as u8);
        pkt.write_u16(1);
        pkt.write_u32(npc.nid);
        write_npc_info_base(&mut pkt, &npc, &tmpl);

        assert_eq!(pkt.data.len(), 2 + 4 + 43);
        assert_eq!(pkt.data[47], 130);
        assert_eq!(pkt.data[48], 1);
    }

    #[test]
    fn test_get_npc_info_size_43_bytes() {
        let tmpl = test_template();
        let npc = test_instance();
        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);

        // Total data: 1 (type) + 4 (npcId) + 43 (GetNpcInfo) = 48
        assert_eq!(pkt.data.len(), 48);
        // GetNpcInfo alone = 43 bytes
        let npc_info_size = pkt.data.len() - 5;
        assert_eq!(npc_info_size, 43);
    }

    #[test]
    fn test_npc_template_is_npc_flag() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.group = 2; // Elmorad

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 2;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // type
        r.read_u32(); // npcId
        r.read_u16(); // protoId

        // isMonster flag should be 2 (NPC, not monster)
        assert_eq!(r.read_u8(), Some(2));
    }

    #[test]
    fn test_npc_scarecrow_type_for_non_monsters() {
        // Sniffer verified: original server sends ACTUAL npc_type, not NPC_SCARECROW (171).
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 7; // actual NPC type from template

        let mut npc = test_instance();
        npc.is_monster = false;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId
        r.read_u16(); // protoId
        r.read_u8(); // isMonster
        r.read_u16(); // pictureId
        r.read_u32(); // sellingGroup

        // NPC type = actual template value (sniffer verified)
        assert_eq!(r.read_u8(), Some(7));
    }

    #[test]
    fn test_monster_uses_actual_type() {
        // Monsters should use their actual type, not NPC_SCARECROW
        let mut tmpl = test_template();
        tmpl.is_monster = true;
        tmpl.npc_type = 5;

        let npc = test_instance();

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId
        r.read_u16(); // protoId
        r.read_u8(); // isMonster
        r.read_u16(); // pictureId
        r.read_u32(); // sellingGroup

        // Monster should use its actual type
        assert_eq!(r.read_u8(), Some(5));
    }

    #[test]
    fn test_npc_instance_event_fields_defaults() {
        let npc = test_instance();
        assert_eq!(npc.event_room, 0);
        assert!(!npc.is_event_npc);
        assert_eq!(npc.summon_type, 0);
    }

    #[test]
    fn test_npc_instance_event_fields_set() {
        let mut npc = test_instance();
        npc.event_room = 42; // 1-based room ID
        npc.is_event_npc = true;
        npc.summon_type = 1; // Monster Stone boss

        assert_eq!(npc.event_room, 42);
        assert!(npc.is_event_npc);
        assert_eq!(npc.summon_type, 1);
    }

    #[test]
    fn test_type191_guard_tower_sends_actual_type() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 191;
        tmpl.s_sid = 32701;
        tmpl.pid = 30311;
        tmpl.level = 70;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 1; // Karus

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId

        // Proto ID
        assert_eq!(r.read_u16(), Some(32701));
        // isMonster = 2 (always NPC for type 191)
        assert_eq!(r.read_u8(), Some(2));
        // Picture ID
        assert_eq!(r.read_u16(), Some(30311));
        // Selling group
        assert_eq!(r.read_u32(), Some(0));
        // NPC type = 191 (NOT 171/NPC_SCARECROW!)
        assert_eq!(r.read_u8(), Some(191));
        // Reserved
        assert_eq!(r.read_u32(), Some(0));
        // Size
        assert_eq!(r.read_u16(), Some(100));
        // Weapons
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));
        // Nation
        assert_eq!(r.read_u8(), Some(1));
        // Level
        assert_eq!(r.read_u8(), Some(70));
        // Position
        assert_eq!(r.read_u16(), Some(6160));
        assert_eq!(r.read_u16(), Some(3410));
        // Reserved
        assert_eq!(r.read_u16(), Some(0));
        // Gate open
        assert_eq!(r.read_u32(), Some(0));
        // Object type
        assert_eq!(r.read_u8(), Some(0));
        // Reserved
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        // Direction
        assert_eq!(r.read_u16(), Some(0));

        // 43 bytes info + 5 bytes header = 48 total
        assert_eq!(r.remaining(), 0);
        assert_eq!(pkt.data.len(), 48);
    }

    #[test]
    fn test_type15_barracks_proto511_packet_format() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 15;
        tmpl.s_sid = 511;
        tmpl.pid = 9854;
        tmpl.name = "<Encampment>".to_string();

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 1;
        npc.clan_name = "TestClan".to_string();
        npc.clan_id = 42;
        npc.clan_mark_version = 3;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId

        assert_eq!(r.read_u16(), Some(511)); // proto
        assert_eq!(r.read_u8(), Some(2)); // isMonster = NPC
        assert_eq!(r.read_u16(), Some(9854)); // pid
        assert_eq!(r.read_u32(), Some(0)); // selling group
        assert_eq!(r.read_u8(), Some(15)); // actual type
        assert_eq!(r.read_u32(), Some(0)); // reserved
        assert_eq!(r.read_u16(), Some(100)); // size
        assert_eq!(r.read_u32(), Some(0)); // weapon1
        assert_eq!(r.read_u32(), Some(0)); // weapon2

        // Clan name string (u16 length prefix)
        let clan_name = r.read_string().unwrap();
        assert_eq!(clan_name, "TestClan");
        // NPC name string
        let npc_name = r.read_string().unwrap();
        assert_eq!(npc_name, "<Encampment>");

        assert_eq!(r.read_u8(), Some(1)); // nation
        assert_eq!(r.read_u8(), Some(80)); // level
        assert_eq!(r.read_u16(), Some(6160)); // posX
        assert_eq!(r.read_u16(), Some(3410)); // posZ
        assert_eq!(r.read_u16(), Some(0)); // reserved
        assert_eq!(r.read_u32(), Some(0)); // gate_open
        assert_eq!(r.read_u8(), Some(0)); // object_type
        assert_eq!(r.read_u16(), Some(42)); // clan_id
        assert_eq!(r.read_u16(), Some(3)); // mark_version
        assert_eq!(r.read_u16(), Some(0)); // direction

        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_type15_barracks_no_clan_empty_string() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 15;
        tmpl.s_sid = 511;
        tmpl.name = "<Encampment>".to_string();

        let mut npc = test_instance();
        npc.is_monster = false;
        // No clan: clan_name is empty, clan_id = 0
        npc.clan_name = String::new();
        npc.clan_id = 0;
        npc.clan_mark_version = 0;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId
        r.read_u16(); // proto
        r.read_u8(); // isMonster
        r.read_u16(); // pid
        r.read_u32(); // selling_group
        r.read_u8(); // type = 15
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        // Empty clan name
        let clan_name = r.read_string().unwrap();
        assert_eq!(clan_name, "");
        // NPC name still present
        let npc_name = r.read_string().unwrap();
        assert_eq!(npc_name, "<Encampment>");

        // Tail: nation, level, pos, reserved, gate, objtype, clan_id=0, mark=0, dir
        r.read_u8(); // nation
        r.read_u8(); // level
        r.read_u16(); // posX
        r.read_u16(); // posZ
        r.read_u16(); // reserved
        r.read_u32(); // gate_open
        r.read_u8(); // object_type
        assert_eq!(r.read_u16(), Some(0)); // clan_id
        assert_eq!(r.read_u16(), Some(0)); // mark_version
        r.read_u16(); // direction

        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_type15_pet_packet_format() {
        // C++ SByte() sets mode flag (writes nothing), then strings use u8-length-prefix.
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 15;
        tmpl.s_sid = 1000; // Not 511 → pet path
        tmpl.pid = 5000;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.user_name = "PlayerOne".to_string();
        npc.pet_name = "Fluffy".to_string();
        npc.nation = 2; // Elmorad

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId

        // C++ pkt.SByte() writes nothing — just sets mode.
        // 2614/2615 requires the real template SID and NPC flag.
        assert_eq!(r.read_u16(), Some(1000));
        assert_eq!(r.read_u8(), Some(2));
        // pid
        assert_eq!(r.read_u16(), Some(5000));
        // u32(0)
        assert_eq!(r.read_u32(), Some(0));
        // type = 15
        assert_eq!(r.read_u8(), Some(15));
        // reserved
        assert_eq!(r.read_u32(), Some(0));
        // size
        assert_eq!(r.read_u16(), Some(100));
        // weapon1, weapon2 = 0 for pets
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u32(), Some(0));

        // User name — SByte mode: u8 length prefix (NOT u16!)
        let user_name = r.read_sbyte_string().unwrap();
        assert_eq!(user_name, "PlayerOne");
        // Pet name — SByte mode
        let pet_name = r.read_sbyte_string().unwrap();
        assert_eq!(pet_name, "Fluffy");

        // 2625 client reads one extra u32 (pet skin, 0 = none) after the pet name
        assert_eq!(r.read_u32(), Some(0));

        // nation
        assert_eq!(r.read_u8(), Some(2));
        // level
        assert_eq!(r.read_u8(), Some(80));
        // pos
        assert_eq!(r.read_u16(), Some(6160));
        assert_eq!(r.read_u16(), Some(3410));
        // reserved
        assert_eq!(r.read_u16(), Some(0));
        // gate, objtype, reserved, reserved, direction
        assert_eq!(r.read_u32(), Some(0));
        assert_eq!(r.read_u8(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(0));
        assert_eq!(r.read_u16(), Some(0));

        assert_eq!(r.remaining(), 0);
    }

    // ── Sprint 380: CSW door/artifact nation override tests ─────────

    #[test]
    fn test_destroyed_artifact_nation_override_to_zero() {
        // (when userClanID == 0, i.e., broadcast path)
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 61; // NPC_DESTROYED_ARTIFACT
        tmpl.s_sid = 541;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 2; // Elmorad — should be overridden to 0

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId
        r.read_u16(); // protoId
        r.read_u8(); // isMonster
        r.read_u16(); // pictureId
        r.read_u32(); // sellingGroup
        r.read_u8(); // npcType (NPC_SCARECROW since is_monster=false)
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        // Nation should be 0 (overridden), NOT 2
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_csw_door_561_nation_override_to_zero() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 50; // NPC_GATE
        tmpl.s_sid = 561;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 1; // Karus — should be overridden to 0

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId
        r.read_u16(); // protoId
        r.read_u8(); // isMonster
        r.read_u16(); // pictureId
        r.read_u32(); // sellingGroup
        r.read_u8(); // npcType
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        // Nation should be 0 (CSW door override)
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_csw_door_562_nation_override_to_zero() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 50; // NPC_GATE
        tmpl.s_sid = 562;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 2;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout
        r.read_u32(); // npcId
        r.read_u16(); // proto
        r.read_u8(); // isMonster
        r.read_u16(); // pid
        r.read_u32(); // selling
        r.read_u8(); // type
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_csw_door_563_nation_override_to_zero() {
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 50; // NPC_GATE
        tmpl.s_sid = 563;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 1;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout
        r.read_u32(); // npcId
        r.read_u16(); // proto
        r.read_u8(); // isMonster
        r.read_u16(); // pid
        r.read_u32(); // selling
        r.read_u8(); // type
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_non_csw_gate_keeps_nation() {
        // A normal gate NPC (not proto 561-563) should keep its nation
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 50; // NPC_GATE
        tmpl.s_sid = 100; // NOT a CSW door

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 2;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout
        r.read_u32(); // npcId
        r.read_u16(); // proto
        r.read_u8(); // isMonster
        r.read_u16(); // pid
        r.read_u32(); // selling
        r.read_u8(); // type
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        // Nation should remain 2 (not a CSW door)
        assert_eq!(r.read_u8(), Some(2));
    }

    #[test]
    fn test_normal_monster_keeps_nation_zero() {
        // Regular monsters always have nation=0 (set in npc.nation)
        let tmpl = test_template(); // is_monster=true, npc_type=0
        let npc = test_instance(); // nation=0

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout
        r.read_u32(); // npcId
        r.read_u16(); // proto
        r.read_u8(); // isMonster
        r.read_u16(); // pid
        r.read_u32(); // selling
        r.read_u8(); // type
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_normal_npc_keeps_nation() {
        // Regular NPCs keep their nation
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 7; // some non-gate, non-artifact type

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 1; // Karus

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout
        r.read_u32(); // npcId
        r.read_u16(); // proto
        r.read_u8(); // isMonster
        r.read_u16(); // pid
        r.read_u32(); // selling
        r.read_u8(); // type
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        // Nation should remain 1
        assert_eq!(r.read_u8(), Some(1));
    }

    #[test]
    fn test_is_csw_door_helper() {
        // Proto 561-563 with NPC_GATE (50) are CSW doors
        assert!(is_csw_door(561, 50));
        assert!(is_csw_door(562, 50));
        assert!(is_csw_door(563, 50));

        // Proto 561 with wrong type is NOT a CSW door
        assert!(!is_csw_door(561, 61));
        assert!(!is_csw_door(561, 0));

        // Proto 560 (out of range) with NPC_GATE is NOT a CSW door
        assert!(!is_csw_door(560, 50));
        assert!(!is_csw_door(564, 50));

        // Normal gate NPC
        assert!(!is_csw_door(100, 50));
    }

    #[test]
    fn test_destroyed_artifact_packet_size_unchanged() {
        // Verify the CSW override doesn't change packet size (still 43 bytes info)
        let mut tmpl = test_template();
        tmpl.is_monster = false;
        tmpl.npc_type = 61;
        tmpl.s_sid = 541;

        let mut npc = test_instance();
        npc.is_monster = false;
        npc.nation = 2;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        // 1 (type) + 4 (npcId) + 43 (GetNpcInfo) = 48
        assert_eq!(pkt.data.len(), 48);
    }

    // ── Sprint 380: Guard summon nation tests ───────────────────────

    #[test]
    fn test_guard_summon_keeps_nation_karus() {
        // Guard summon (proto 8850) is a monster but retains its nation.
        let mut tmpl = test_template();
        tmpl.is_monster = true;
        tmpl.s_sid = 8850; // GUARD_SUMMON
        tmpl.npc_type = 0; // monster type
        tmpl.group = 1; // Karus

        let mut npc = test_instance();
        npc.is_monster = true;
        npc.proto_id = 8850;
        npc.nation = 0; // spawn sets monsters to 0, but guard summon should use tmpl.group

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout type
        r.read_u32(); // npcId
        r.read_u16(); // protoId
        r.read_u8(); // isMonster
        r.read_u16(); // pictureId
        r.read_u32(); // sellingGroup
        r.read_u8(); // npcType
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        // Guard summon should send nation=1 (Karus) from template group, NOT 0
        assert_eq!(r.read_u8(), Some(1));
    }

    #[test]
    fn test_guard_summon_keeps_nation_elmorad() {
        // Guard summon (proto 8850) with Elmorad nation
        let mut tmpl = test_template();
        tmpl.is_monster = true;
        tmpl.s_sid = 8850;
        tmpl.npc_type = 0;
        tmpl.group = 2; // Elmorad

        let mut npc = test_instance();
        npc.is_monster = true;
        npc.proto_id = 8850;
        npc.nation = 0;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout
        r.read_u32(); // npcId
        r.read_u16(); // proto
        r.read_u8(); // isMonster
        r.read_u16(); // pid
        r.read_u32(); // selling
        r.read_u8(); // type
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        // Guard summon should send nation=2 (Elmorad)
        assert_eq!(r.read_u8(), Some(2));
    }

    #[test]
    fn test_guard_summon_packet_size_43_bytes() {
        // Guard summon uses default format — 43 bytes info
        let mut tmpl = test_template();
        tmpl.is_monster = true;
        tmpl.s_sid = 8850;
        tmpl.group = 1;

        let mut npc = test_instance();
        npc.is_monster = true;
        npc.proto_id = 8850;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        // 1 (type) + 4 (npcId) + 43 (GetNpcInfo) = 48
        assert_eq!(pkt.data.len(), 48);
    }

    #[test]
    fn test_regular_monster_still_gets_nation_zero() {
        // Non-guard-summon monsters should still get nation=0 (regression test)
        let mut tmpl = test_template();
        tmpl.is_monster = true;
        tmpl.s_sid = 150; // regular monster, NOT 8850
        tmpl.npc_type = 0;
        tmpl.group = 1; // has a group, but should be ignored for regular monsters

        let mut npc = test_instance();
        npc.is_monster = true;
        npc.nation = 0;

        let pkt = build_npc_inout(NPC_IN, &npc, &tmpl);
        let mut r = PacketReader::new(&pkt.data);

        r.read_u8(); // inout
        r.read_u32(); // npcId
        r.read_u16(); // proto
        r.read_u8(); // isMonster
        r.read_u16(); // pid
        r.read_u32(); // selling
        r.read_u8(); // type
        r.read_u32(); // reserved
        r.read_u16(); // size
        r.read_u32(); // weapon1
        r.read_u32(); // weapon2

        // Regular monster should still send nation=0
        assert_eq!(r.read_u8(), Some(0));
    }

    #[test]
    fn test_guard_summon_constant_value() {
        // Verify the GUARD_SUMMON constant matches C++ Define.h:481
        assert_eq!(GUARD_SUMMON, 8850);
    }
}
