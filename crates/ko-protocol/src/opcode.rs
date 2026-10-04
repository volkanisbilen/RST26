//! Opcode definitions for Knight Online packets.
//! Game Server opcodes correspond to `WIZ_*` constants,
//! Login Server opcodes correspond to `LS_*` constants.
//! See  and

/// Enumeration of all known packet opcodes.
/// Opcodes are transmitted as a single byte in the packet header.
/// Values are sourced from the C++ reference implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Opcode {
    // ---------------------------------------------------------------
    // 0x01..0x0F  Core login / character / movement
    // ---------------------------------------------------------------
    /// Account login request/response.
    WizLogin = 0x01,
    /// Create new character.
    WizNewChar = 0x02,
    /// Delete character.
    WizDelChar = 0x03,
    /// Select character to play.
    WizSelChar = 0x04,
    /// Select nation (Karus/Elmorad).
    WizSelNation = 0x05,
    /// Character movement (1-second interpolated).
    WizMove = 0x06,
    /// User enter/leave region notification.
    WizUserInout = 0x07,
    /// Attack packet.
    WizAttack = 0x08,
    /// Character rotation.
    WizRotate = 0x09,
    /// NPC enter/leave region notification.
    WizNpcInout = 0x0A,
    /// NPC movement (1-second interpolated).
    WizNpcMove = 0x0B,
    /// All character info request.
    WizAllcharInfoReq = 0x0C,
    /// Game start signal.
    WizGamestart = 0x0D,
    /// Character detail data (SendMyInfo).
    WizMyInfo = 0x0E,
    /// Player logout / return to character select.
    WizLogout = 0x0F,

    // ---------------------------------------------------------------
    // 0x10..0x1F  Chat, regen, timers, HP/MP/EXP, items
    // ---------------------------------------------------------------
    /// User chatting.
    WizChat = 0x10,
    /// User dead notification.
    WizDead = 0x11,
    /// User regeneration.
    WizRegene = 0x12,
    /// Game timer.
    WizTime = 0x13,
    /// Game weather.
    WizWeather = 0x14,
    /// Region change / nearby user+NPC list.
    WizRegionChange = 0x15,
    /// Client request for unregistered user info.
    WizReqUserIn = 0x16,
    /// Current HP download.
    WizHpChange = 0x17,
    /// Current MP download.
    WizMspChange = 0x18,
    /// Nation chat (also used as item log channel).
    WizNationChat = 0x19,
    /// Current EXP download.
    WizExpChange = 0x1A,
    /// Level change (max HP, MP, SP, weight, exp download).
    WizLevelChange = 0x1B,
    /// NPC region list.
    WizNpcRegion = 0x1C,
    /// Client request for unregistered NPC info.
    WizReqNpcIn = 0x1D,
    /// Warp within same zone.
    WizWarp = 0x1E,
    /// User item move.
    WizItemMove = 0x1F,

    // ---------------------------------------------------------------
    // 0x20..0x2F  NPC events, items, zone, state, loyalty, party
    // ---------------------------------------------------------------
    /// User click NPC event.
    WizNpcEvent = 0x20,
    /// Item trade.
    WizItemTrade = 0x21,
    /// Target HP query (attack result).
    WizTargetHp = 0x22,
    /// Zone item insert (drop).
    WizItemDrop = 0x23,
    /// Zone item list request (bundle open).
    WizBundleOpenReq = 0x24,
    /// Item trade with NPC start.
    WizTradeNpc = 0x25,
    /// Zone item get (pick up).
    WizItemGet = 0x26,
    /// Zone change (cross-zone teleport).
    WizZoneChange = 0x27,
    /// Stat point change (str, sta, dex, int, cha).
    WizPointChange = 0x28,
    /// State change (sit/stand, emotes, visibility).
    WizStateChange = 0x29,
    /// Nation contribution (loyalty) change.
    WizLoyaltyChange = 0x2A,
    /// Client version check.
    WizVersionCheck = 0x2B,
    /// Encryption key exchange.
    WizCryption = 0x2C,
    /// User slot item resource change (look).
    WizUserlookChange = 0x2D,
    /// Update notice alarm.
    WizNotice = 0x2E,
    /// Party related packet. **Dual-purpose in v2600**: also used for charsel
    /// detail data (sub_74E590). Sub=3 sends per-character info to populate the
    /// character selection linked list (AddCharacter / sub_BFEE00).
    WizParty = 0x2F,

    // ---------------------------------------------------------------
    // 0x30..0x3F  Exchange, magic, skills, class, knights
    // ---------------------------------------------------------------
    /// Exchange (player trade) related packet.
    WizExchange = 0x30,
    /// Magic related packet.
    WizMagicProcess = 0x31,
    /// User changed particular skill point.
    WizSkillptChange = 0x32,
    /// Map object event (e.g. bind point setting).
    WizObjectEvent = 0x33,
    /// Class change (level 10+ job advancement).
    WizClassChange = 0x34,
    /// Select private chat target user.
    WizChatTarget = 0x35,
    /// Current game user count.
    WizConcurrentUser = 0x36,
    /// User data periodic DB save request.
    ///
    WizDatasave = 0x37,
    /// Item durability change notification (server→client).
    ///
    WizDuration = 0x38,
    /// NPC repair/tinker shop open (server→client).
    ///
    WizRepairNpc = 0x3A,
    /// Item repair processing.
    WizItemRepair = 0x3B,
    /// Knights (clan) processing.
    WizKnightsProcess = 0x3C,
    /// Item count/stack change notification.
    WizItemCountChange = 0x3D,
    /// All knights list info download.
    WizKnightsList = 0x3E,
    /// Item remove from inventory.
    WizItemRemove = 0x3F,

    // ---------------------------------------------------------------
    // 0x40..0x4F  Operator, speedhack, compress, warehouse, home, friends
    // ---------------------------------------------------------------
    /// Operator authority packet.
    WizOperator = 0x40,
    /// Speed hack detection check.
    WizSpeedhackCheck = 0x41,
    /// Compressed packet wrapper.
    WizCompressPacket = 0x42,
    /// Server status check packet.
    WizServerCheck = 0x43,
    /// Virtual server / channel change request.
    ///
    WizVirtualServer = 0x4C,
    /// Warehouse open, in, out.
    WizWarehouse = 0x45,
    /// Report bug to the manager (no-op in C++).
    ///
    WizReportBug = 0x47,
    /// Come back home (return to town).
    WizHome = 0x48,
    /// Friend list processing.
    WizFriendProcess = 0x49,
    /// Gold change (enemy gold pickup).
    WizGoldChange = 0x4A,
    /// Warp list by NPC or object.
    WizWarpList = 0x4B,
    /// Party wanted bulletin board.
    WizPartyBbs = 0x4F,

    // ---------------------------------------------------------------
    // 0x50..0x5F  Client/map events, weight, NPC say, upgrade, quest event
    // ---------------------------------------------------------------
    /// Client event (for quest).
    WizClientEvent = 0x52,
    /// Map event.
    WizMapEvent = 0x53,
    /// Notify change of weight.
    WizWeightChange = 0x54,
    /// Select event message.
    WizSelectMsg = 0x55,
    /// NPC say event message.
    WizNpcSay = 0x56,
    /// Battle event results (rankings, bulletin board).
    ///
    WizBattleEvent = 0x57,
    /// Authority / fame change broadcast (war commander, clan leader).
    ///
    WizAuthorityChange = 0x58,
    /// Edit box / PPCard (product key redemption) system.
    ///
    WizEditBox = 0x59,
    /// Flying Santa Claus / Angel visual event.
    ///
    WizSanta = 0x5A,
    /// Item upgrade.
    WizItemUpgrade = 0x5B,
    /// Zone ability — status effects (DOT cure, poison cure, etc.).
    ///
    WizZoneability = 0x5E,
    /// Event system.
    WizEvent = 0x5F,
    /// Stealth state reset (server→client).
    ///
    WizStealth = 0x60,

    // ---------------------------------------------------------------
    // 0x60..0x6F  Quest, merchant, shopping, server index, effect, siege
    // ---------------------------------------------------------------
    /// Quest system.
    WizQuest = 0x64,
    /// Merchant (personal shop).
    WizMerchant = 0x68,
    /// Merchant enter/leave notification.
    WizMerchantInout = 0x69,
    /// Shopping mall (cash shop).
    WizShoppingMall = 0x6A,
    /// Server index/identity.
    WizServerIndex = 0x6B,
    /// Visual effect.
    WizEffect = 0x6C,
    /// Siege warfare.
    WizSiege = 0x6D,
    /// Name change system.
    WizNameChange = 0x6E,

    // ---------------------------------------------------------------
    // 0x70..0x7F  Cape, premium, hacktool, rental, pet, king, skill, rank
    // ---------------------------------------------------------------
    /// Cape system.
    WizCape = 0x70,
    /// Premium (VIP) system.
    WizPremium = 0x71,
    /// Hack tool detection (server ignores).
    WizHacktool = 0x72,
    /// Item rental system.
    WizRental = 0x73,
    /// Challenge system.
    WizChallenge = 0x75,
    /// Pet system.
    WizPet = 0x76,
    /// King system.
    WizKing = 0x78,
    /// Skill data (shortcut bar save/load).
    WizSkillData = 0x79,
    /// Program check (anti-cheat).
    WizProgramCheck = 0x7A,
    /// Bifrost / Beef Roast event system.
    WizBifrost = 0x7B,
    /// Sheriff report system (WIZ_REPORT).
    ///
    WizReport = 0x7C,
    /// Logos shout / server-wide announcement with colors.
    ///
    WizLogosshout = 0x7D,

    // ---------------------------------------------------------------
    // 0x80..0x9F  Rank, nation transfer, mining, helmet, VIP warehouse,
    //             gender change, genie, user info, achieve, loading
    // ---------------------------------------------------------------
    /// Ranking system.
    WizRank = 0x80,
    /// Story / intro cutscene (server→client only).
    ///
    WizStory = 0x81,
    /// Nation transfer.
    WizNationTransfer = 0x82,
    /// Terrain effects notification (server→client only).
    ///
    WizTerrainEffects = 0x83,
    /// Moving tower (siege tower boarding/dismounting).
    ///
    WizMovingTower = 0x84,
    /// Mining system.
    WizMining = 0x86,
    /// Helmet visibility toggle.
    WizHelmet = 0x87,
    /// PVP rivalry system.
    ///
    WizPvp = 0x88,
    /// Change hair/face at character select.
    WizChangeHair = 0x89,
    /// VIP warehouse.
    WizVipwarehouse = 0x8B,
    /// Gender change.
    WizGenderChange = 0x8D,
    /// Genie (lamp) system.
    WizGenie = 0x97,
    /// User information (nearby user list).
    WizUserInfo = 0x98,
    /// User achievement system.
    WizUserAchieve = 0x99,
    /// Experience seal system.
    WizExpSeal = 0x9A,
    /// Kurian SP change.
    WizKurianSpChange = 0x9B,
    /// Loading login -- server queue / capacity check.
    WizLoadingLogin = 0x9F,

    // ---------------------------------------------------------------
    // 0xB0+  Vanguard / Wanted event
    // ---------------------------------------------------------------
    /// Area sound effect (server→client).
    ///
    WizSound = 0xB5,
    /// Wanted (Vanguard) event system.
    WizVanguard = 0xB6,

    /// Stat/Skill preset system (save/load stat and skill configurations).
    ///
    WizPreset = 0xB9,
    /// Auto-loot / auto-drop configuration (client-sent, server ignores).
    ///
    WizAutoDrop = 0xBA,

    /// Merchant search/list system — search for items being sold by merchants.
    ///
    WizMerchantList = 0xBD,

    // ---------------------------------------------------------------
    // 0x92+  v2525 new opcodes (not in v2369 C++ reference)
    // ---------------------------------------------------------------
    /// Max HP change notification (v2525 native, S2C inline handler).
    WizMaxHpChange = 0x92,
    /// Seal/binding system (v2525, Table G 9-sub dispatch).
    WizSeal = 0x95,
    /// Continuous packet data (v2525 resource transfer protocol).
    ///
    /// Part of the 0x47/0x9C transfer system: 0x47 initiates, 0x9C sends data.
    /// Sub-opcodes: 0x02/0x03 (compress_handler), 0x04/0x05, 0x06-0x09, 0x0E, 0xF0.
    /// NOT a generic container — cannot be used for ext_hook S2C.
    WizContinousPacketData = 0x9C,
    /// Achievement system set 2 (v2525 native, inline handler).
    WizAchievement2 = 0xA5,
    /// Collection system 1 (v2525, Table G 9-sub dispatch).
    WizCollection1 = 0xA9,
    /// Premium system 2 (v2525 native, inline handler).
    WizPremium2 = 0xAC,
    /// Collection system 2 (v2525, Table G 9-sub dispatch).
    WizCollection2 = 0xB4,
    /// Attendance system (v2525, Table G 9-sub dispatch).
    WizAttendance = 0xB7,
    /// Upgrade notice (v2525 native, inline handler).
    WizUpgradeNotice = 0xB8,

    // ---------------------------------------------------------------
    // 0xC0+  Captcha, daily rank, battle event
    // ---------------------------------------------------------------
    /// Captcha verification (anti-bot check).
    ///
    WizCaptcha = 0xC0,
    /// Daily ranking system.
    WizDailyRank = 0xC2,
    /// Costume system (v2525 native, inline handler).
    WizCostume = 0xC3,
    /// Soul system (v2525 native, inline handler).
    WizSoul = 0xC5,
    /// Daily quest (v2525 native — replaces ext_hook sub 0xD3 for >= v2369).
    ///
    /// v2525 uses this dedicated opcode.
    WizDailyQuest = 0xC7,
    /// Awakening system (v2525 native, inline handler).
    WizAwakening = 0xCB,
    /// Enchant system (v2525 native, inline handler).
    WizEnchant = 0xCC,
    /// Challenge system 2 (v2525 native, inline handler).
    WizChallenge2 = 0xCE,
    /// Ability system (v2525 native, inline handler).
    WizAbility = 0xCF,
    /// Guild bank system (v2525 native, inline handler).
    WizGuildBank = 0xD0,

    // ---------------------------------------------------------------
    // 0xD0+  Clan warehouse
    // ---------------------------------------------------------------
    /// Clan warehouse (shared bank) open, input, output, move.
    ///
    WizClanWarehouse = 0xD1,
    /// Rebirth system (v2525 native, inline handler).
    WizRebirth = 0xD3,
    /// Territory system (v2525 native, inline handler).
    WizTerritory = 0xD5,
    /// World boss system (v2525 native, inline handler).
    WizWorldBoss = 0xD6,
    /// Season system (v2525 native, inline handler — max GameMain opcode).
    WizSeason = 0xD7,
    /// Moradon native nation-ranker statues (R..W) and ranker memo UI.
    WizRanker = 0xD9,
    /// Scrolling notice message (used for merchant wind notice, etc.).
    ///
    /// **WARNING**: Outside v2525 GameMain range (0x06-0xD7), silently dropped.
    WizAddMsg = 0xDB,

    // ---------------------------------------------------------------
    // 0xE0+  Extended opcodes (Cinderella, CSW, Juraid, etc.)
    // ---------------------------------------------------------------
    /// Cinderella event system.
    WizCinderella = 0xE0,
    /// Party HP display.
    ///
    /// **WARNING**: Outside v2525 GameMain range (0x06-0xD7), silently dropped.
    WizPartyHp = 0xE8,
    /// Extended hook — opcode container for perks, death notice, KC, lottery, etc.
    ///
    /// **WARNING**: Outside v2525 GameMain range (0x06-0xD7), silently dropped.
    /// S2C packets should use `Opcode::EXT_HOOK_S2C` (0x9C) instead.
    WizExtHook = 0xE9,

    // ---------------------------------------------------------------
    // Missing opcodes added for completeness (Sprint 511)
    // ---------------------------------------------------------------
    /// Continuous region data packet (stub in C++).
    WizContinousPacket = 0x44,
    /// Server change notification.
    WizServerChange = 0x46,
    /// Battle zone concurrent user count request.
    WizZoneConcurrent = 0x4D,
    /// Corpse name display.
    WizCorpse = 0x4E,
    /// Market bulletin board service (stub in C++).
    WizMarketBbs = 0x50,
    /// Duplicate connection kick notification.
    WizKickout = 0x51,
    /// Clan premium system (stub in C++).
    WizClanPremium = 0x5C,
    /// Player state flag update (S2C only, v2525).
    ///
    /// No C++ server handler — client has real handler at `0x82E1E8`.
    /// Sets a byte flag at `player_object+0xB69` (visual/transform state).
    WizPacket2 = 0x5D,
    /// Room packet processing (stub in C++).
    WizRoomPacketProcess = 0x61,
    /// Room system (stub in C++).
    WizRoom = 0x62,
    /// Clan battle system (stub in C++).
    WizClanBattle = 0x63,
    /// PPCard login verification.
    WizPpCardLogin = 0x65,
    /// Kiss emote system (stub in C++).
    WizKiss = 0x66,
    /// User recommendation (stub in C++).
    WizRecommendUser = 0x67,
    /// Web page link (stub in C++).
    WizWebpage = 0x6F,
    /// Timed item expiration notification.
    WizItemExpiration = 0x74,
    /// China-specific opcode (unused in C++).
    WizChina = 0x77,
    /// Screenshot/capture system (stub in C++).
    WizCapture = 0x85,
    /// Loyalty (NP) shop (stub — no C++ handler).
    WizLoyaltyShop = 0x90,
    /// Clan battle points system.
    WizClanpointsBattle = 0x91,
    /// Kill assist notification.
    WizKillAssist = 0xC8,
    /// Manes Survival shares byte 0xD0 with the v2525 guild-bank route.
    /// Dispatch must therefore use protocol version and sub-opcode context.
    /// Knight Royale event.
    WizKnightRoyale = 0xEF,

    // ---------------------------------------------------------------
    // 0xFA..0xFE  GameGuard protocol (custom, not in original client)
    // ---------------------------------------------------------------
    /// GameGuard challenge response (client → server).
    WizGameguardResp = 0xFA,
    /// GameGuard server challenge (server → client).
    WizGameguardChal = 0xFB,
    /// GameGuard heartbeat (UDP, client → server).
    WizGameguardHb = 0xFC,
    /// GameGuard authentication (client ↔ server).
    WizGameguardAuth = 0xFD,
    /// GameGuard X25519 key exchange (client ↔ server).
    WizGameguardKey = 0xFE,
}

impl Opcode {
    /// S2C opcode byte for ext_hook packets.
    ///
    /// **v2525 LIMITATION**: GameMain dispatch range is 0x06-0xD7. This opcode
    /// (0xE9) is OUTSIDE that range and silently dropped by vanilla v2525 clients.
    /// 0x9C was tried as alternative but it's WIZ_CONTINOUS_PACKET (resource
    /// transfer protocol) — ext_hook sub-opcodes don't match its dispatch table.
    ///
    /// Ext_hook S2C features require client-side support (modified client) or
    /// per-feature remapping to v2525 native opcodes with correct packet formats.
    /// For notification-only features, WizChat WAR_SYSTEM_CHAT is used as fallback.
    pub const EXT_HOOK_S2C: u8 = 0xE9;

    /// Try to convert a raw `u8` value into an `Opcode`.
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            // 0x01..0x0F  Core login / character / movement
            0x01 => Some(Self::WizLogin),
            0x02 => Some(Self::WizNewChar),
            0x03 => Some(Self::WizDelChar),
            0x04 => Some(Self::WizSelChar),
            0x05 => Some(Self::WizSelNation),
            0x06 => Some(Self::WizMove),
            0x07 => Some(Self::WizUserInout),
            0x08 => Some(Self::WizAttack),
            0x09 => Some(Self::WizRotate),
            0x0A => Some(Self::WizNpcInout),
            0x0B => Some(Self::WizNpcMove),
            0x0C => Some(Self::WizAllcharInfoReq),
            0x0D => Some(Self::WizGamestart),
            0x0E => Some(Self::WizMyInfo),
            0x0F => Some(Self::WizLogout),
            // 0x10..0x1F  Chat, regen, timers, HP/MP/EXP, items
            0x10 => Some(Self::WizChat),
            0x11 => Some(Self::WizDead),
            0x12 => Some(Self::WizRegene),
            0x13 => Some(Self::WizTime),
            0x14 => Some(Self::WizWeather),
            0x15 => Some(Self::WizRegionChange),
            0x16 => Some(Self::WizReqUserIn),
            0x17 => Some(Self::WizHpChange),
            0x18 => Some(Self::WizMspChange),
            0x19 => Some(Self::WizNationChat),
            0x1A => Some(Self::WizExpChange),
            0x1B => Some(Self::WizLevelChange),
            0x1C => Some(Self::WizNpcRegion),
            0x1D => Some(Self::WizReqNpcIn),
            0x1E => Some(Self::WizWarp),
            0x1F => Some(Self::WizItemMove),
            // 0x20..0x2F  NPC events, items, zone, state, loyalty, party
            0x20 => Some(Self::WizNpcEvent),
            0x21 => Some(Self::WizItemTrade),
            0x22 => Some(Self::WizTargetHp),
            0x23 => Some(Self::WizItemDrop),
            0x24 => Some(Self::WizBundleOpenReq),
            0x25 => Some(Self::WizTradeNpc),
            0x26 => Some(Self::WizItemGet),
            0x27 => Some(Self::WizZoneChange),
            0x28 => Some(Self::WizPointChange),
            0x29 => Some(Self::WizStateChange),
            0x2A => Some(Self::WizLoyaltyChange),
            0x2B => Some(Self::WizVersionCheck),
            0x2C => Some(Self::WizCryption),
            0x2D => Some(Self::WizUserlookChange),
            0x2E => Some(Self::WizNotice),
            0x2F => Some(Self::WizParty),
            // 0x30..0x3F  Exchange, magic, skills, class, knights
            0x30 => Some(Self::WizExchange),
            0x31 => Some(Self::WizMagicProcess),
            0x32 => Some(Self::WizSkillptChange),
            0x33 => Some(Self::WizObjectEvent),
            0x34 => Some(Self::WizClassChange),
            0x35 => Some(Self::WizChatTarget),
            0x36 => Some(Self::WizConcurrentUser),
            0x37 => Some(Self::WizDatasave),
            0x38 => Some(Self::WizDuration),
            0x3A => Some(Self::WizRepairNpc),
            0x3B => Some(Self::WizItemRepair),
            0x3C => Some(Self::WizKnightsProcess),
            0x3D => Some(Self::WizItemCountChange),
            0x3E => Some(Self::WizKnightsList),
            0x3F => Some(Self::WizItemRemove),
            // 0x40..0x4F  Operator, speedhack, compress, warehouse, friends
            0x40 => Some(Self::WizOperator),
            0x41 => Some(Self::WizSpeedhackCheck),
            0x42 => Some(Self::WizCompressPacket),
            0x43 => Some(Self::WizServerCheck),
            0x44 => Some(Self::WizContinousPacket),
            0x45 => Some(Self::WizWarehouse),
            0x46 => Some(Self::WizServerChange),
            0x47 => Some(Self::WizReportBug),
            0x48 => Some(Self::WizHome),
            0x49 => Some(Self::WizFriendProcess),
            0x4A => Some(Self::WizGoldChange),
            0x4B => Some(Self::WizWarpList),
            0x4C => Some(Self::WizVirtualServer),
            0x4D => Some(Self::WizZoneConcurrent),
            0x4E => Some(Self::WizCorpse),
            0x4F => Some(Self::WizPartyBbs),
            0x50 => Some(Self::WizMarketBbs),
            0x51 => Some(Self::WizKickout),
            // 0x50..0x5F  Client/map events, weight, NPC say, upgrade
            0x52 => Some(Self::WizClientEvent),
            0x53 => Some(Self::WizMapEvent),
            0x54 => Some(Self::WizWeightChange),
            0x55 => Some(Self::WizSelectMsg),
            0x56 => Some(Self::WizNpcSay),
            0x57 => Some(Self::WizBattleEvent),
            0x58 => Some(Self::WizAuthorityChange),
            0x59 => Some(Self::WizEditBox),
            0x5A => Some(Self::WizSanta),
            0x5B => Some(Self::WizItemUpgrade),
            0x5C => Some(Self::WizClanPremium),
            0x5D => Some(Self::WizPacket2),
            0x5E => Some(Self::WizZoneability),
            0x5F => Some(Self::WizEvent),
            0x60 => Some(Self::WizStealth),
            0x61 => Some(Self::WizRoomPacketProcess),
            0x62 => Some(Self::WizRoom),
            0x63 => Some(Self::WizClanBattle),
            // 0x64..0x6F  Quest, merchant, shopping, server index, effect, siege
            0x64 => Some(Self::WizQuest),
            0x65 => Some(Self::WizPpCardLogin),
            0x66 => Some(Self::WizKiss),
            0x67 => Some(Self::WizRecommendUser),
            0x68 => Some(Self::WizMerchant),
            0x69 => Some(Self::WizMerchantInout),
            0x6A => Some(Self::WizShoppingMall),
            0x6B => Some(Self::WizServerIndex),
            0x6C => Some(Self::WizEffect),
            0x6D => Some(Self::WizSiege),
            0x6E => Some(Self::WizNameChange),
            0x6F => Some(Self::WizWebpage),
            // 0x70..0x7F  Cape, premium, hacktool, rental, pet, king, skill
            0x70 => Some(Self::WizCape),
            0x71 => Some(Self::WizPremium),
            0x72 => Some(Self::WizHacktool),
            0x73 => Some(Self::WizRental),
            0x74 => Some(Self::WizItemExpiration),
            0x75 => Some(Self::WizChallenge),
            0x76 => Some(Self::WizPet),
            0x77 => Some(Self::WizChina),
            0x78 => Some(Self::WizKing),
            0x79 => Some(Self::WizSkillData),
            0x7A => Some(Self::WizProgramCheck),
            0x7B => Some(Self::WizBifrost),
            0x7C => Some(Self::WizReport),
            0x7D => Some(Self::WizLogosshout),
            // 0x80..0x9F  Rank, story, nation transfer, terrain, tower, mining, etc.
            0x80 => Some(Self::WizRank),
            0x81 => Some(Self::WizStory),
            0x82 => Some(Self::WizNationTransfer),
            0x83 => Some(Self::WizTerrainEffects),
            0x84 => Some(Self::WizMovingTower),
            0x85 => Some(Self::WizCapture),
            0x86 => Some(Self::WizMining),
            0x87 => Some(Self::WizHelmet),
            0x88 => Some(Self::WizPvp),
            0x89 => Some(Self::WizChangeHair),
            0x8B => Some(Self::WizVipwarehouse),
            0x8D => Some(Self::WizGenderChange),
            0x90 => Some(Self::WizLoyaltyShop),
            0x91 => Some(Self::WizClanpointsBattle),
            0x97 => Some(Self::WizGenie),
            0x98 => Some(Self::WizUserInfo),
            0x99 => Some(Self::WizUserAchieve),
            0x9A => Some(Self::WizExpSeal),
            0x9B => Some(Self::WizKurianSpChange),
            0x9F => Some(Self::WizLoadingLogin),
            // 0xB0+  Sound, Vanguard, Preset
            0xB5 => Some(Self::WizSound),
            0xB6 => Some(Self::WizVanguard),
            0xB9 => Some(Self::WizPreset),
            0xBA => Some(Self::WizAutoDrop),
            0xBD => Some(Self::WizMerchantList),
            // v2525 new opcodes
            0x92 => Some(Self::WizMaxHpChange),
            0x95 => Some(Self::WizSeal),
            0x9C => Some(Self::WizContinousPacketData),
            0xA5 => Some(Self::WizAchievement2),
            0xA9 => Some(Self::WizCollection1),
            0xAC => Some(Self::WizPremium2),
            0xB4 => Some(Self::WizCollection2),
            0xB7 => Some(Self::WizAttendance),
            0xB8 => Some(Self::WizUpgradeNotice),
            // 0xC0+  Captcha, daily rank, kill assist
            0xC0 => Some(Self::WizCaptcha),
            0xC2 => Some(Self::WizDailyRank),
            0xC3 => Some(Self::WizCostume),
            0xC5 => Some(Self::WizSoul),
            0xC7 => Some(Self::WizDailyQuest),
            0xC8 => Some(Self::WizKillAssist),
            0xCB => Some(Self::WizAwakening),
            0xCC => Some(Self::WizEnchant),
            0xCE => Some(Self::WizChallenge2),
            0xCF => Some(Self::WizAbility),
            0xD0 => Some(Self::WizGuildBank),
            // 0xD0+  Clan warehouse
            0xD1 => Some(Self::WizClanWarehouse),
            0xD3 => Some(Self::WizRebirth),
            0xD5 => Some(Self::WizTerritory),
            0xD6 => Some(Self::WizWorldBoss),
            0xD7 => Some(Self::WizSeason),
            0xD9 => Some(Self::WizRanker),
            0xDB => Some(Self::WizAddMsg),
            // 0xE0+  Extended
            0xE0 => Some(Self::WizCinderella),
            0xE8 => Some(Self::WizPartyHp),
            0xE9 => Some(Self::WizExtHook),
            0xEF => Some(Self::WizKnightRoyale),
            // GameGuard
            0xFA => Some(Self::WizGameguardResp),
            0xFB => Some(Self::WizGameguardChal),
            0xFC => Some(Self::WizGameguardHb),
            0xFD => Some(Self::WizGameguardAuth),
            0xFE => Some(Self::WizGameguardKey),
            _ => None,
        }
    }
}

/// Login Server opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LoginOpcode {
    /// Client version check request (old launcher protocol).
    LsVersionReq = 0x01,
    /// Patch download info request.
    LsDownloadInfoReq = 0x02,
    /// Launcher keep-alive ping.
    LsKoreakoLauncherPing = 0x03,
    /// v2600 handshake: account + version_int (first packet from game client).
    LsHandshake = 0x51,
    /// Encryption key exchange.
    LsCryption = 0xF2,
    /// Account login request.
    LsLoginReq = 0xF3,
    /// Server list request/response (echo pattern). C++ LS_SERVERLIST.
    LsServerList = 0xF5,
    /// News/notice (echo pattern). C++ LS_NEWS = 0xF6.
    /// S→C: "INotice" + notice text. Name kept for backwards compat.
    LsVersionCheck = 0xF6,
    /// Unknown (returns u16(0)).
    LsUnkF7 = 0xF7,
    /// Server redirect (S→C only).
    LsServerRedirect = 0xA1,
    /// Server select (C→S only, client connects to game server).
    LsServerSelect = 0xA6,
    /// Password login / post-failure reconnect (client-specific, plaintext).
    LsPasswordLogin = 0xEA,
    /// OTP (one-time password) verification.
    LsOtp = 0xFA,
    /// OTP sync (echo pattern). C++ LS_OTP_SYNC = 0xFD.
    /// Name kept for backwards compat.
    LsNews = 0xFD,
}

impl LoginOpcode {
    /// Try to convert a raw `u8` value into a `LoginOpcode`.
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Self::LsVersionReq),
            0x02 => Some(Self::LsDownloadInfoReq),
            0x03 => Some(Self::LsKoreakoLauncherPing),
            0x51 => Some(Self::LsHandshake),
            0xF2 => Some(Self::LsCryption),
            0xF3 => Some(Self::LsLoginReq),
            0xF5 => Some(Self::LsServerList),
            0xF6 => Some(Self::LsVersionCheck),
            0xF7 => Some(Self::LsUnkF7),
            0xA1 => Some(Self::LsServerRedirect),
            0xA6 => Some(Self::LsServerSelect),
            0xEA => Some(Self::LsPasswordLogin),
            0xFA => Some(Self::LsOtp),
            0xFD => Some(Self::LsNews),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_roundtrip() {
        assert_eq!(Opcode::from_byte(0x01), Some(Opcode::WizLogin));
        assert_eq!(Opcode::from_byte(0x2C), Some(Opcode::WizCryption));
        // The enum uses the v2525 name for the shared, version-dependent byte.
        assert_eq!(Opcode::from_byte(0xD0), Some(Opcode::WizGuildBank));
        assert_eq!(Opcode::from_byte(0xFF), None);
    }

    #[test]
    fn test_opcode_as_u8() {
        assert_eq!(Opcode::WizLogin as u8, 0x01);
        assert_eq!(Opcode::WizCryption as u8, 0x2C);
        assert_eq!(Opcode::WizVersionCheck as u8, 0x2B);
    }

    #[test]
    fn test_new_opcodes_roundtrip() {
        // Spot-check a selection of newly added opcodes
        let cases: &[(u8, Opcode)] = &[
            (0x10, Opcode::WizChat),
            (0x11, Opcode::WizDead),
            (0x12, Opcode::WizRegene),
            (0x13, Opcode::WizTime),
            (0x14, Opcode::WizWeather),
            (0x17, Opcode::WizHpChange),
            (0x18, Opcode::WizMspChange),
            (0x19, Opcode::WizNationChat),
            (0x1A, Opcode::WizExpChange),
            (0x1B, Opcode::WizLevelChange),
            (0x1F, Opcode::WizItemMove),
            (0x20, Opcode::WizNpcEvent),
            (0x21, Opcode::WizItemTrade),
            (0x23, Opcode::WizItemDrop),
            (0x24, Opcode::WizBundleOpenReq),
            (0x25, Opcode::WizTradeNpc),
            (0x26, Opcode::WizItemGet),
            (0x28, Opcode::WizPointChange),
            (0x2A, Opcode::WizLoyaltyChange),
            (0x2D, Opcode::WizUserlookChange),
            (0x2E, Opcode::WizNotice),
            (0x2F, Opcode::WizParty),
            (0x30, Opcode::WizExchange),
            (0x31, Opcode::WizMagicProcess),
            (0x32, Opcode::WizSkillptChange),
            (0x33, Opcode::WizObjectEvent),
            (0x34, Opcode::WizClassChange),
            (0x35, Opcode::WizChatTarget),
            (0x36, Opcode::WizConcurrentUser),
            (0x37, Opcode::WizDatasave),
            (0x38, Opcode::WizDuration),
            (0x3A, Opcode::WizRepairNpc),
            (0x3B, Opcode::WizItemRepair),
            (0x3D, Opcode::WizItemCountChange),
            (0x3E, Opcode::WizKnightsList),
            (0x3F, Opcode::WizItemRemove),
            (0x40, Opcode::WizOperator),
            (0x43, Opcode::WizServerCheck),
            (0x45, Opcode::WizWarehouse),
            (0x4C, Opcode::WizVirtualServer),
            (0x48, Opcode::WizHome),
            (0x4A, Opcode::WizGoldChange),
            (0x4B, Opcode::WizWarpList),
            (0x4F, Opcode::WizPartyBbs),
            (0x52, Opcode::WizClientEvent),
            (0x53, Opcode::WizMapEvent),
            (0x54, Opcode::WizWeightChange),
            (0x55, Opcode::WizSelectMsg),
            (0x56, Opcode::WizNpcSay),
            (0x57, Opcode::WizBattleEvent),
            (0x58, Opcode::WizAuthorityChange),
            (0x59, Opcode::WizEditBox),
            (0x5A, Opcode::WizSanta),
            (0x5B, Opcode::WizItemUpgrade),
            (0x5E, Opcode::WizZoneability),
            (0x5F, Opcode::WizEvent),
            (0x60, Opcode::WizStealth),
            (0x64, Opcode::WizQuest),
            (0x68, Opcode::WizMerchant),
            (0x69, Opcode::WizMerchantInout),
            (0x6C, Opcode::WizEffect),
            (0x6D, Opcode::WizSiege),
            (0x6E, Opcode::WizNameChange),
            (0x70, Opcode::WizCape),
            (0x71, Opcode::WizPremium),
            (0x75, Opcode::WizChallenge),
            (0x76, Opcode::WizPet),
            (0x78, Opcode::WizKing),
            (0x7A, Opcode::WizProgramCheck),
            (0x7B, Opcode::WizBifrost),
            (0x7C, Opcode::WizReport),
            (0x7D, Opcode::WizLogosshout),
            (0x47, Opcode::WizReportBug),
            (0x80, Opcode::WizRank),
            (0x81, Opcode::WizStory),
            (0x82, Opcode::WizNationTransfer),
            (0x83, Opcode::WizTerrainEffects),
            (0x84, Opcode::WizMovingTower),
            (0x86, Opcode::WizMining),
            (0x88, Opcode::WizPvp),
            (0x8B, Opcode::WizVipwarehouse),
            (0x8D, Opcode::WizGenderChange),
            (0x9A, Opcode::WizExpSeal),
            (0x9B, Opcode::WizKurianSpChange),
            (0xB5, Opcode::WizSound),
            (0xB6, Opcode::WizVanguard),
            (0xB9, Opcode::WizPreset),
            (0xBA, Opcode::WizAutoDrop),
            (0xBD, Opcode::WizMerchantList),
            (0xC2, Opcode::WizDailyRank),
            (0xD1, Opcode::WizClanWarehouse),
            (0xDB, Opcode::WizAddMsg),
            (0xE0, Opcode::WizCinderella),
            (0xE8, Opcode::WizPartyHp),
            (0xE9, Opcode::WizExtHook),
            // Sprint 511 additions
            (0x44, Opcode::WizContinousPacket),
            (0x46, Opcode::WizServerChange),
            (0x4D, Opcode::WizZoneConcurrent),
            (0x4E, Opcode::WizCorpse),
            (0x50, Opcode::WizMarketBbs),
            (0x51, Opcode::WizKickout),
            (0x5C, Opcode::WizClanPremium),
            (0x5D, Opcode::WizPacket2),
            (0x61, Opcode::WizRoomPacketProcess),
            (0x62, Opcode::WizRoom),
            (0x63, Opcode::WizClanBattle),
            (0x65, Opcode::WizPpCardLogin),
            (0x66, Opcode::WizKiss),
            (0x67, Opcode::WizRecommendUser),
            (0x6F, Opcode::WizWebpage),
            (0x74, Opcode::WizItemExpiration),
            (0x77, Opcode::WizChina),
            (0x85, Opcode::WizCapture),
            (0x90, Opcode::WizLoyaltyShop),
            (0x91, Opcode::WizClanpointsBattle),
            (0xC8, Opcode::WizKillAssist),
            (0xEF, Opcode::WizKnightRoyale),
            // v2525 new opcodes
            (0x92, Opcode::WizMaxHpChange),
            (0x95, Opcode::WizSeal),
            (0x9C, Opcode::WizContinousPacketData),
            (0xA5, Opcode::WizAchievement2),
            (0xA9, Opcode::WizCollection1),
            (0xAC, Opcode::WizPremium2),
            (0xB4, Opcode::WizCollection2),
            (0xB7, Opcode::WizAttendance),
            (0xB8, Opcode::WizUpgradeNotice),
            (0xC3, Opcode::WizCostume),
            (0xC5, Opcode::WizSoul),
            (0xC7, Opcode::WizDailyQuest),
            (0xCB, Opcode::WizAwakening),
            (0xCC, Opcode::WizEnchant),
            (0xCE, Opcode::WizChallenge2),
            (0xCF, Opcode::WizAbility),
            (0xD0, Opcode::WizGuildBank),
            (0xD3, Opcode::WizRebirth),
            (0xD5, Opcode::WizTerritory),
            (0xD6, Opcode::WizWorldBoss),
            (0xD7, Opcode::WizSeason),
            (0xD9, Opcode::WizRanker),
        ];
        for &(byte, expected) in cases {
            assert_eq!(
                Opcode::from_byte(byte),
                Some(expected),
                "from_byte(0x{:02X}) failed",
                byte
            );
            assert_eq!(expected as u8, byte, "{:?} as u8 failed", expected);
        }
    }

    #[test]
    fn test_wiz_pvp_opcode_value() {
        assert_eq!(Opcode::WizPvp as u8, 0x88);
        assert_eq!(Opcode::from_byte(0x88), Some(Opcode::WizPvp));
    }

    #[test]
    fn test_ext_hook_s2c_constant() {
        // EXT_HOOK_S2C = 0xE9 (same as WizExtHook).
        // v2525 vanilla clients drop this opcode (outside 0x06-0xD7 range).
        // Kept for modified client compatibility; vanilla clients use WizChat fallback.
        assert_eq!(Opcode::EXT_HOOK_S2C, 0xE9);
        assert_eq!(Opcode::EXT_HOOK_S2C, Opcode::WizExtHook as u8);
    }

    #[test]
    fn test_login_opcode_roundtrip() {
        assert_eq!(
            LoginOpcode::from_byte(0x01),
            Some(LoginOpcode::LsVersionReq)
        );
        assert_eq!(LoginOpcode::from_byte(0xF2), Some(LoginOpcode::LsCryption));
        assert_eq!(LoginOpcode::from_byte(0xF3), Some(LoginOpcode::LsLoginReq));
        assert_eq!(
            LoginOpcode::from_byte(0xF5),
            Some(LoginOpcode::LsServerList)
        );
        assert_eq!(
            LoginOpcode::from_byte(0xF6),
            Some(LoginOpcode::LsVersionCheck)
        );
        assert_eq!(LoginOpcode::from_byte(0xFD), Some(LoginOpcode::LsNews));
        assert_eq!(LoginOpcode::from_byte(0x42), None);
    }

    #[test]
    fn test_login_opcode_as_u8() {
        assert_eq!(LoginOpcode::LsVersionReq as u8, 0x01);
        assert_eq!(LoginOpcode::LsCryption as u8, 0xF2);
        assert_eq!(LoginOpcode::LsLoginReq as u8, 0xF3);
        assert_eq!(LoginOpcode::LsServerList as u8, 0xF5);
    }
}
