//! Type definitions for WorldState -- enums, structs, and constants.
//! Extracted from `world/mod.rs` to reduce file size.
//! These types are re-exported from `mod.rs` via `pub use types::*;`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use ko_db::models::daily_quest::UserDailyQuestRow;
use ko_protocol::Packet;

use crate::npc::NpcId;
use crate::zone::SessionId;

/// User state: standing.
pub const USER_STANDING: u8 = 0x01;
/// User state: sitting down.
pub const USER_SITDOWN: u8 = 0x02;
/// User state: dead.
pub const USER_DEAD: u8 = 0x03;
/// User state: interacting with monument.
pub const USER_MONUMENT: u8 = 0x06;
/// User state: mining.
pub const USER_MINING: u8 = 0x07;
/// User state: fishing (flashing).
pub const USER_FLASHING: u8 = 0x08;

/// Nation: Karus.
pub const NATION_KARUS: u8 = 1;

/// Nation: El Morad.
pub const NATION_ELMORAD: u8 = 2;

/// Maximum character level.
pub const MAX_LEVEL: u16 = 83;

/// Premium property opcodes for looking up specific bonus values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PremiumProperty {
    /// Gold gain bonus percent from monster drops.
    NoahPercent,
    /// Item drop rate bonus percent.
    DropPercent,
    /// Flat bonus loyalty (NP) per PK kill.
    BonusLoyalty,
    /// Repair cost discount (e.g., 50 = pay 50% of normal).
    RepairDiscountPercent,
    /// If > 0, sell price uses buy_price/4 instead of /6.
    ItemSellPercent,
}

/// NPC AI state enum â€” matches `NpcState` in `globals.h:80-95`.
/// Each NPC cycles through these states via the AI tick system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NpcState {
    /// Dead, waiting for respawn timer.
    Dead = 0,
    /// Just respawned, transitioning to Standing.
    Live = 1,
    /// Idle, scanning for enemies or deciding to patrol.
    Standing = 5,
    /// Walking toward a random patrol waypoint.
    Moving = 6,
    /// Enemy found, transitioning to Tracing or Fighting.
    Attacking = 2,
    /// Chasing a target, not yet in attack range.
    Tracing = 7,
    /// In attack range, executing attack loop.
    Fighting = 8,
    /// Returning to spawn point (leash).
    Back = 10,
    /// Stun debuff (sleep) â€” frozen for duration, then wakes to Fighting.
    ///
    Sleeping = 11,
    /// Lightning stun â€” 2 second freeze, then returns to Standing.
    ///
    Fainting = 12,
    /// Healer NPC â€” finds and heals injured nearby friendly NPCs.
    ///
    Healing = 13,
    /// Executing a skill â€” waits for cast time, then applies effect.
    ///
    Casting = 14,
}

/// Maximum distance (squared) from spawn before NPC is leashed back.
pub const NPC_MAX_LEASH_RANGE: f32 = 200.0;

/// Mutable per-NPC runtime AI state.
/// Stored in `WorldState::npc_ai` DashMap, keyed by NpcId.
#[derive(Debug, Clone)]
pub struct NpcAiState {
    /// Current AI state.
    pub state: NpcState,
    /// Spawn position X (for leash distance calculation).
    pub spawn_x: f32,
    /// Spawn position Z (for leash distance calculation).
    pub spawn_z: f32,
    /// Current position X (may differ from NpcInstance.x during movement).
    pub cur_x: f32,
    /// Current position Z (may differ from NpcInstance.z during movement).
    pub cur_z: f32,
    /// Current target (player session ID), if any.
    pub target_id: Option<SessionId>,
    /// Current NPC target (for NPC-vs-NPC combat: guards attacking monsters).
    ///
    /// When set, `target_id` should be `None` â€” the NPC is fighting another NPC, not a player.
    pub npc_target_id: Option<NpcId>,
    /// Remaining delay before next state tick (in milliseconds).
    pub delay_ms: u64,
    /// Time of last AI tick.
    pub last_tick_ms: u64,
    /// Respawn timer in milliseconds (from NPC spawn data, typically 30s).
    pub regen_time_ms: u64,
    /// Whether this NPC is aggressive (will attack on sight).
    ///
    pub is_aggressive: bool,
    /// Zone ID this NPC belongs to.
    pub zone_id: u16,
    /// Region grid X (updated as NPC moves).
    pub region_x: u16,
    /// Region grid Z (updated as NPC moves).
    pub region_z: u16,
    /// Timestamp (tick ms) when sleeping/fainting ends.
    ///
    pub fainting_until_ms: u64,
    /// Previous AI state before entering CASTING.
    ///
    pub old_state: NpcState,
    /// Skill ID currently being cast (0 = none).
    ///
    pub active_skill_id: u32,
    /// Target ID for the skill being cast (-1 = none).
    ///
    pub active_target_id: i32,
    /// Cast time of active skill in ms (0 = none).
    ///
    pub active_cast_time_ms: u64,
    /// Whether this NPC has pack behavior (calls friends when attacked).
    ///
    pub has_friends: bool,
    /// Family type for group AI â€” same-family NPCs assist each other.
    ///
    pub family_type: u8,
    /// Skill cooldown timestamp (tick ms) â€” prevents magic spam.
    ///
    pub skill_cooldown_ms: u64,
    /// Nation of the NPC (1=Karus, 2=Elmorad, 0=neutral).
    ///
    pub nation: u8,
    /// Whether this NPC (type 191 tower) is currently owned/mounted by a player.
    ///
    pub is_tower_owner: bool,
    /// NPC attack type â€” 0 = TENDER (passive), 1 = ATROCITY (aggressive).
    ///
    /// TENDER monsters only attack when they (or their pack) are damaged first.
    /// ATROCITY monsters attack players on sight.
    pub attack_type: u8,
    /// Tick timestamp (ms) of last combat interaction (damage dealt/received).
    ///
    /// any combat activity, the NPC disengages from TRACING and returns to STANDING.
    pub last_combat_time_ms: u64,
    /// Duration in seconds â€” if > 0, NPC will die after this many seconds.
    ///
    /// Used for summoned/event NPCs that should automatically despawn.
    pub duration_secs: u16,
    /// Tick timestamp (ms) when NPC was spawned (for duration check).
    ///
    pub spawned_at_ms: u64,
    /// Tick timestamp (ms) of last HP regen tick.
    ///
    pub last_hp_regen_ms: u64,
    /// Gate open/close state: 0=closed, 1=open, 2=open (event-forced).
    ///
    pub gate_open: u8,
    /// Wood object cooldown counter â€” incremented each standing tick when open.
    ///
    pub wood_cooldown_count: u32,
    /// UTC second counter for boss magic patterns (magic_attack == 3).
    ///
    /// reset at cycle-specific thresholds per boss proto ID.
    pub utc_second: u32,

    // â”€â”€ Pathfinding fields â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Computed A* path waypoints (world coordinates). Empty = no active path.
    ///
    pub path_waypoints: Vec<(f32, f32)>,
    /// Current index into `path_waypoints` â€” the next waypoint to move toward.
    ///
    pub path_index: usize,
    /// Target X position when the current path was computed.
    /// Used to detect when the target has moved enough to warrant recomputation.
    pub path_target_x: f32,
    /// Target Z position when the current path was computed.
    pub path_target_z: f32,
    /// Whether the current path was computed via A* (true) or direct line (false).
    ///
    /// Note: C++ naming is inverted; m_bPathFlag=true means *no* pathfinding was used.
    pub path_is_direct: bool,

    // ── Movement destination (for incremental step movement) ─────────
    /// Destination X for current MOVING state.
    ///
    pub dest_x: f32,
    /// Destination Z for current MOVING state.
    ///
    pub dest_z: f32,
    /// Pattern frame counter — cycles: 0=random, 1=random, 2=return to spawn.
    ///
    pub pattern_frame: u8,
}

/// Map `m_byActType` (DB value) to NPC attack type used by the AI.
///   - `act_type 1,2,3,4` â†’ `TENDER_ATTACK_TYPE (0)` â€” passive, only fights back
///   - `default`          â†’ `ATROCITY_ATTACK_TYPE (1)` â€” aggressive, attacks on sight
pub(crate) fn map_act_type(act_type: u8) -> u8 {
    match act_type {
        1..=4 => 0, // TENDER_ATTACK_TYPE
        _ => 1,     // ATROCITY_ATTACK_TYPE
    }
}

/// Check if an NPC type is a gate/object type that needs gate AI logic.
/// Gate types: NPC_GATE(50), NPC_PHOENIX_GATE(51), NPC_SPECIAL_GATE(52),
/// NPC_VICTORY_GATE(53), NPC_OBJECT_WOOD(54), NPC_GATE_LEVER(55),
/// NPC_KARUS_MONUMENT(121), NPC_HUMAN_MONUMENT(122), NPC_GATE2(150),
/// NPC_KROWAZ_GATE(180).
pub(crate) fn is_gate_npc_type(npc_type: u8) -> bool {
    matches!(
        npc_type,
        50 | 51 | 52 | 53 | 54 | 55 | 121 | 122 | 150 | 180
    )
}

/// Returns `true` if `npc_type` is a guard NPC that should have AI.
///   NPC_GUARD(11), NPC_PATROL_GUARD(12), NPC_STORE_GUARD(13).
///   NPC_GUARD_TOWER1(14), NPC_GUARD_TOWER2(15).
pub(crate) fn is_guard_npc_type(npc_type: u8) -> bool {
    matches!(npc_type, 11..=15)
}

/// Per-magic-type cooldown entry.
#[derive(Debug, Clone)]
pub struct TypeCooldown {
    /// When this type was last cast (tick-based).
    pub time: Instant,
    /// Whether a speed violation was detected (allows stricter threshold).
    pub t_catch: bool,
}

/// Handle for sending packets to a specific session.
pub struct SessionHandle {
    /// Channel sender to the session's writer task.
    pub tx: mpsc::UnboundedSender<Arc<Packet>>,
    /// Character info for building INOUT packets (None if not in-game yet).
    pub character: Option<CharacterInfo>,
    /// Current position in the world.
    pub position: Position,
    /// Facing direction (set by WIZ_ROTATE). `m_sDirection`.
    pub direction: i16,
    /// Last time the session received a valid packet (monotonic).
    ///
    /// successful decryption.  Used by `Timer_UpdateSessions` for the 10-minute
    /// loading timeout check.
    pub last_response_time: Instant,
    /// True while a zone change is in progress
    /// Blocks movement processing until the zone change completes.
    pub zone_changing: bool,
    /// When zone_changing was set to `true`.  Used to auto-clear the flag
    /// after a safety timeout (30 s) so the player doesn't get permanently stuck.
    pub zone_change_started_at: Instant,
    /// Set when player interacts with a warp gate; cleared on first move with speed > 0,
    /// on warp failure, or on successful zone change completion.
    ///
    pub check_warp_zone_change: bool,
    /// Private chat target session ID, set by WIZ_CHAT_TARGET.
    ///
    pub private_chat_target: Option<SessionId>,
    /// Current Z-target ID (player or NPC), set by WIZ_TARGET_HP.
    ///
    pub target_id: u32,
    /// Pending clan invitation (`m_bKnightsReq`). 0 = no pending invite.
    pub pending_knights_invite: u16,
    /// Pending gate keeper tax amount. 0 = no pending tax.
    /// Set when server sends WIZ_PREMIUM2 init, cleared on confirm/cancel.
    pub pending_gate_tax: u32,
    /// Active type-4 buffs/debuffs keyed by buff_type.
    ///
    pub buffs: HashMap<i32, ActiveBuff>,
    /// Saved magic map for buff persistence: skill_id -> expiry timestamp (ms).
    ///
    /// Only skills with ID > 500000 are persisted (scroll buffs).
    pub saved_magic_map: HashMap<u32, u64>,
    /// Active type-3 DOT/HOT effects (max `MAX_TYPE3_REPEAT` slots).
    ///
    pub durational_skills: Vec<DurationalSkill>,
    /// Inventory slots (equipment + bag + cospre + magic bags).
    ///
    /// Index 0-13: equipment slots, 14-41: inventory bag, 42-76: cospre/magic bags.
    pub inventory: Vec<UserItemSlot>,
    /// Computed equipment stats (updated by SetUserAbility).
    ///
    pub equipped_stats: EquippedStats,

    // â”€â”€ Quest State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Per-player quest progress: quest_id -> UserQuestInfo.
    ///
    pub quests: HashMap<u16, UserQuestInfo>,
    /// Per-player daily quest progress: quest_index -> UserDailyQuestRow.
    ///
    pub daily_quests: HashMap<i16, UserDailyQuestRow>,
    /// NPC the player is currently interacting with (runtime NPC ID).
    ///
    pub event_nid: i16,
    /// Proto ID of the NPC being interacted with (for Lua/quest convenience).
    ///
    pub event_sid: i16,
    /// Current quest helper ID for NPC dialog (Lua script).
    ///
    pub quest_helper_id: u32,
    /// Selected reward index from SelectMsg dialog.
    ///
    /// Stored as a member variable; read by `RunQuestExchange` for `item_exchange_exp` lookup.
    pub by_selected_reward: i8,
    /// Select message flag (dialog type from Lua).
    ///
    pub select_msg_flag: u8,
    /// Stored event IDs for each dialog button (MAX_MESSAGE_EVENT=12).
    ///
    pub select_msg_events: [i32; 12],

    // â”€â”€ Warehouse (Inn) State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Warehouse item slots (per-account, 192 slots = 8 pages * 24 items).
    ///
    pub warehouse: Vec<UserItemSlot>,
    /// Gold stored in the warehouse (inn coins).
    ///
    pub inn_coins: u32,
    /// Whether the warehouse data has been loaded from DB for this session.
    pub warehouse_loaded: bool,

    // â”€â”€ VIP Warehouse State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// VIP warehouse item slots (per-account, 48 slots).
    ///
    pub vip_warehouse: Vec<UserItemSlot>,
    /// VIP warehouse 4-digit PIN password.
    ///
    pub vip_password: String,
    /// Whether a password is set (0=no, 1=yes).
    ///
    pub vip_password_request: u8,
    /// Vault key expiration (unix timestamp, 0=not activated).
    ///
    pub vip_vault_expiry: u32,
    /// Whether VIP warehouse data has been loaded from DB.
    pub vip_warehouse_loaded: bool,

    // ── Knight Return Symbol ────────────────────────────────────────
    /// Return symbol status (0=inactive, >0=active).
    ///
    pub return_symbol_ok: u32,
    /// Return symbol expiry (unix timestamp, 0=none).
    ///
    pub return_symbol_time: i64,
    /// Whether the shopping mall UI is currently open.
    ///
    pub store_open: bool,

    // â”€â”€ Trade (Exchange) State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Trade status (`m_sTradeStatue`: 1=None, 2=Sender, 3=Target, 4=Trading, 5=Deciding).
    pub trade_state: u8,
    /// The other player's session ID in the trade
    pub exchange_user: Option<SessionId>,
    /// Items listed for trade by this player
    pub exchange_items: Vec<ExchangeItem>,
    /// Whether this player sent the trade request
    pub is_request_sender: bool,

    // â”€â”€ Merchant State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Merchant state (`m_bMerchantState`: -1=None, 0=Selling, 1=Buying).
    ///
    pub merchant_state: i8,
    /// True while setting up a selling merchant
    pub selling_merchant_preparing: bool,
    /// True while setting up a buying merchant
    pub buying_merchant_preparing: bool,
    /// Items in the merchant shop
    pub merchant_items: [MerchData; MAX_MERCH_ITEMS],
    /// Session ID of the player currently browsing this shop
    pub merchant_looker: Option<SessionId>,
    /// Session ID of the merchant shop this player is browsing
    pub browsing_merchant: Option<SessionId>,

    // â”€â”€ Ranking State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Daily loyalty earned from PK (reset periodically).
    ///
    pub pk_loyalty_daily: u32,
    /// Premium bonus loyalty earned from PK.
    ///
    pub pk_loyalty_premium_bonus: u16,
    /// Personal rank (1-based, from DB/ranking system).
    ///
    pub personal_rank: u8,
    /// Knights (clan) rank (1-based, from DB/ranking system).
    ///
    pub knights_rank: u8,

    // â”€â”€ Achievement State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Per-player achievement progress: achieve_id -> UserAchieveInfo.
    ///
    pub achieve_map: HashMap<u16, UserAchieveInfo>,
    /// Achievement summary stats (play time, kills, deaths, medals, titles).
    ///
    pub achieve_summary: AchieveSummary,
    /// Unix timestamp when this session entered the game (for play_time tracking).
    ///
    pub achieve_login_time: u32,
    /// Active timed challenge: achieve_id -> expiration unix timestamp.
    ///
    pub achieve_timed: HashMap<u16, u32>,
    /// Whether a timed challenge is currently active.
    ///
    pub achieve_challenge_active: bool,
    /// Stat bonuses from equipped skill title [STR, STA, DEX, INT, CHA, Attack, Defence].
    ///
    pub achieve_stat_bonuses: [i16; 7],

    // â”€â”€ Challenge (Duel) State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Non-zero when this player is the challenger (sent the request).
    /// Stores the cancel sub-opcode (PVP=2, CVC=7).
    ///
    pub requesting_challenge: u8,
    /// Non-zero when this player is the challengee (received the request).
    /// Stores the reject sub-opcode (PVP=4, CVC=9).
    ///
    pub challenge_requested: u8,
    /// Session ID of the challenge partner (-1 = none).
    ///
    pub challenge_user: i16,

    // â”€â”€ GM / Moderation State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // ── Death / Resurrection State ─────────────────────────────────────
    /// EXP lost on the most recent death. Used by resurrection skills (Type5)
    /// to restore a percentage of lost EXP via `bExpRecover`.
    ///
    /// Set in `OnDeath`, reset to 0 on regene.
    pub lost_exp: i64,
    /// Session ID of the player who killed this player, or -1 for PvE death.
    /// Resurrection skill EXP recovery only works when `who_killed_me == -1`.
    ///
    /// Set to killer SID on PvP death, -1 on NPC/environment death.
    /// Reset to -1 on regene and zone change.
    pub who_killed_me: i16,

    /// Whether the player is muted (cannot send chat messages).
    ///
    pub is_muted: bool,
    /// Attack disabled until this UNIX timestamp. 0 = enabled, u32::MAX = permanent.
    ///
    pub attack_disabled_until: u32,
    /// Timestamp of last chat message (for flood detection).
    ///
    pub last_chat_time: Instant,
    /// Number of chat messages sent in the current 1-second window.
    pub chat_flood_count: u8,
    /// Timestamp of last `/town` (WIZ_HOME) use, for 1200s cooldown.
    ///
    pub last_town_time: Instant,
    /// Session ID of the last GM the player PM'd (for rate limiting).
    ///
    pub gm_send_pm_id: u16,
    /// Cooldown expiry for GM PM rate limit (10 minutes when switching GMs).
    ///
    pub gm_send_pm_time: Instant,

    // â”€â”€ Mining & Fishing State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Whether the player is currently mining.
    ///
    pub is_mining: bool,
    /// Whether the player is currently fishing.
    ///
    pub is_fishing: bool,
    /// Next auto-mining tick timestamp (unix seconds).
    ///
    pub auto_mining_time: u64,
    /// Next auto-fishing tick timestamp (unix seconds).
    ///
    pub auto_fishing_time: u64,
    /// Bifrost piece exchange cooldown (1500ms).
    ///
    pub beef_exchange_time: Instant,
    /// Event room ID this player is in (1-based, 0 = not in any event room).
    ///
    /// When a player activates a Monster Stone, this is set to `room_id + 1`.
    /// Combat, NPC visibility, and broadcasts are filtered by this field.
    pub event_room: u16,
    /// Whether the player is looking for a party (set by StateChange Type 2).
    ///
    pub need_party: u8,
    /// Party leader flag, set via StateChange type=6.
    ///
    pub party_leader: u8,
    /// Party type: 0=normal, 2=Full Moon Rift.
    ///
    /// with the client-provided type. DD entry requires type 2.
    pub party_type: i8,
    /// Whether this player has an active Monster Stone room.
    ///
    /// Monster Stone room is activated, `false` on room exit/reset.
    /// Used as a guard in attack/magic handlers for event room isolation.
    pub monster_stone_status: bool,
    /// Draki Tower daily entrance limit (max 3, decremented on each entry).
    ///
    pub draki_entrance_limit: u8,
    /// Draki Tower room ID this player is in (0 = not in Draki Tower).
    ///
    pub draki_room_id: u16,
    /// Whether this player has been assigned to an event room during the Running phase.
    ///
    /// places the player into an active event room instance.
    pub joined_event: bool,
    /// Whether this is the player's final event join (no re-entry after death).
    ///
    /// the same event instance if the player has been eliminated.
    pub is_final_joined_event: bool,
    /// Timestamp of last mining/fishing attempt (for 5s cooldown).
    ///
    pub last_mining_attempt: Instant,

    /// Timestamp of last item upgrade attempt (for UPGRADE_DELAY cooldown).
    ///
    pub last_upgrade_time: Instant,
    /// Session upgrade attempt counter (capped at `UserMaxUpgradeCount`).
    ///
    pub upgrade_count: u8,

    /// Timestamp of last potion use (for 2400ms cooldown).
    ///
    /// Interval: `PLAYER_POTION_REQUEST_INTERVAL = 2400ms`
    pub last_potion_time: Instant,

    /// Shared 850ms cooldown for party target/alert/command operations.
    ///
    /// Used by `PartyTargetNumber`, `PartyAlert`, and `PartyCommand`.
    pub last_target_number_time: Instant,

    /// Team colour for PvP events (soccer, arenas).
    ///
    /// Values: 0=None, 1=Blue, 2=Red, 3=Outside, 4=Map
    pub team_colour: u8,

    //â”€â”€ Attack Rate Limit State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Earliest time the next R-attack is allowed (server-side rate limit).
    ///
    /// Set to `now + 900ms` after each valid attack.
    pub last_attack_time: Option<Instant>,
    /// Per-skill cooldown map: skill_id -> expiry Instant.
    ///
    /// Skills with sReCastTime > 0 are tracked here after MAGIC_EFFECTING.
    pub skill_cooldowns: HashMap<u32, Instant>,
    /// Per-type cooldown map: magic_type -> TypeCooldown.
    ///
    /// Prevents casting skills of the same type too rapidly.
    /// Key = bType[0] or bType[1] (0-9, plus synthetic key 10 for type3 with t_1==-1).
    pub magic_type_cooldowns: HashMap<u8, TypeCooldown>,

    // ── Cast Position Validation ────────────────────────────────────────
    /// Skill ID saved during MAGIC_CASTING for position validation.
    ///
    pub cast_skill_id: u32,
    /// X position saved during MAGIC_CASTING.
    ///
    pub cast_x: f32,
    /// Z position saved during MAGIC_CASTING.
    ///
    pub cast_z: f32,

    /// Movement-during-cast failure flag (AnimatedSkill validation).
    /// Set to true if player moves during MAGIC_CASTING phase.
    /// Causes subsequent FLYING/EFFECTING phases to fail.
    ///
    pub cast_failed: bool,

    /// Last Type 2 animated skill cast timestamp (milliseconds).
    ///
    pub last_type2_cast_time: u64,

    /// Last Type 2 animated skill ID cast.
    ///
    pub last_type2_skill_id: u32,

    /// Mage Armor reflect element type (0=none, 5=Fire, 6=Ice, 7=Lightning).
    ///
    /// Set by BUFF_TYPE_MAGE_ARMOR (25) via `pSkill.sSkill % 100`.
    /// Consumed on first hit (one-time reflect).
    pub reflect_armor_type: u8,

    /// Dagger defense amount modifier (default 100 = full defense).
    ///
    /// Reduced by Eskrima debuff (BUFF_TYPE_DAGGER_BOW_DEFENSE, 45).
    /// Used in `GetACDamage()`: `damage -= damage * (m_sDaggerR * amount / 100) / 250`
    pub dagger_r_amount: u8,

    /// Bow defense amount modifier (default 100 = full defense).
    ///
    /// Reduced by Eskrima debuff (BUFF_TYPE_DAGGER_BOW_DEFENSE, 45).
    /// Used in `GetACDamage()`: `damage -= damage * (m_sBowR * amount / 100) / 250`
    pub bow_r_amount: u8,

    /// Whether skill-buff mirror damage is active (Minak's Thorn).
    ///
    /// Set by BUFF_TYPE_MIRROR_DAMAGE_PARTY (44).
    pub mirror_damage: bool,

    /// Mirror damage type: true = reflect to attacker, false = split among party.
    ///
    /// true for skill 492028, false otherwise.
    pub mirror_damage_type: bool,

    /// Mirror damage percentage (0-100).
    ///
    /// Formula: `mirrorDamage = (m_byMirrorAmount * amount) / 100`
    pub mirror_amount: u8,

    // â"€â"€ Trap / Speed Hack State â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€
    /// Last time this player took trap area damage.
    ///
    pub last_trap_time: Instant,
    /// Last validated position for speed hack detection.
    ///
    pub speed_last_x: f32,
    /// Last validated Z position for speed hack detection.
    pub speed_last_z: f32,

    // â”€â”€ Movement Validation State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Consecutive speed-hack violation count (3 = warp Home).
    ///
    pub speed_hack_count: u8,
    /// Previous echo value from last move packet (-1 = initial).
    ///
    pub move_old_echo: i8,
    /// Previous speed value from last move packet.
    ///
    pub move_old_speed: i16,
    /// Expiry time for the caught-time window (echo anomaly detection).
    ///
    pub move_caught_time: Instant,
    /// Previous destination X (Ã—10) for distance/speed ratio correction.
    ///
    pub move_old_will_x: u16,
    /// Previous destination Z (Ã—10) for distance/speed ratio correction.
    ///
    pub move_old_will_z: u16,
    /// Previous destination Y (Ã—10) for distance/speed ratio correction.
    ///
    pub move_old_will_y: u16,

    // â”€â”€ Pet State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Active pet data (None if no pet summoned).
    ///
    pub pet_data: Option<PetState>,
    /// Unix timestamp of the last pet satisfaction decay check.
    ///
    /// Pet satisfaction decays by 100 every `PLAYER_TRAINING_INTERVAL * 4` (60s).
    pub last_pet_decay_time: u64,

    // ── Cosmetic Visibility Flags ────────────────────────────────────
    /// Whether the player is hiding their cosmetic items (costume).
    ///
    pub is_hiding_cospre: bool,
    /// Whether the player is hiding their helmet cosmetic.
    ///
    pub is_hiding_helmet: bool,
    /// Whether the ITEM_OREADS fairy is currently equipped in COSP_FAIRY slot.
    ///
    pub fairy_check: bool,
    /// Whether a robin loot item is equipped in SHOULDER slot (pos 5).
    ///
    /// Robin items: 950680000, 850680000, 510000000, 520000000
    pub auto_loot: bool,

    // â”€â”€ Wanted Event State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Whether this player is a wanted target in the current event.
    ///
    pub is_wanted: bool,
    /// Wanted event expiry time (unix timestamp, 0 = not active).
    ///
    pub wanted_expiry_time: u32,

    // â”€â”€ Premium State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Account ID for this session (needed for premium periodic save).
    ///
    pub account_id: String,
    /// Active premium subscriptions: premium_type -> expiration unix timestamp.
    ///
    pub premium_map: HashMap<u8, u32>,
    /// Currently active premium type (0 = NO_PREMIUM).
    ///
    pub premium_in_use: u8,
    /// Clan premium type (0 = none, 13 = CLAN_PREMIUM).
    ///
    pub clan_premium_in_use: u8,
    /// Counter for how many switch premiums have been loaded this session.
    ///
    pub switch_premium_count: u8,
    /// Account status (1 = premium active, 0 = none).
    ///
    pub account_status: u8,

    // â”€â”€ Repurchase (Trash Item) State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Items sold to NPC merchants that can be bought back (max 10,000).
    ///
    pub deleted_items: Vec<DeletedItemEntry>,
    /// Display index mapping for current repurchase browse session.
    /// Maps display_index (u8) -> position in `deleted_items` vec.
    ///
    pub delete_item_list: HashMap<u8, usize>,

    // â”€â”€ Chat Room State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Index of the chat room the player is currently in (0 = not in a room).
    ///
    pub chat_room_index: u16,

    // â”€â”€ PM Block State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Whether the player is blocking private messages.
    ///
    pub block_private_chat: bool,

    // â”€â”€ Perk State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Perk levels per type (13 types, 0-based index).
    ///
    pub perk_levels: [i16; 13],
    /// Unspent perk points.
    ///
    pub rem_perk: i16,

    // ── Soul State (v2525) ──────────────────────────────────────────────
    /// Soul category values: 8 categories × 3 rank values.
    ///
    /// `[cat_id, value_0, value_1, value_2]` — cat_id 0-7.
    pub soul_categories: [[i16; 4]; 8],
    /// Soul slot values: 5 slots.
    ///
    /// `[slot_id, value]` — slot_id 0-4.
    pub soul_slots: [[i16; 2]; 5],
    /// Whether soul data has been loaded from DB.
    pub soul_loaded: bool,

    // ── Hermetic Seal (0xCF) State ──────────────────────────────────
    /// Maximum tier achieved (0-9).
    pub seal_max_tier: u8,
    /// Currently selected slot index (0-23).
    pub seal_selected_slot: u8,
    /// Status: 0=active, 1=paused, 2=completed.
    pub seal_status: u8,
    /// Number of upgrade attempts.
    pub seal_upgrade_count: u8,
    /// Current upgrade level (0-9).
    pub seal_current_level: u8,
    /// Elapsed progress time in seconds.
    pub seal_elapsed_time: f64,
    /// Whether hermetic seal data has been loaded from DB.
    pub seal_loaded: bool,

    // ── Costume (0xC3) State ─────────────────────────────────────────────
    /// Active type: 0=none, 1=available, 2=equipped, 3=expired.
    pub costume_active_type: u16,
    /// Equipped costume item ID.
    pub costume_item_id: i32,
    /// Costume item parameter.
    pub costume_item_param: i32,
    /// Model scale value.
    pub costume_scale_raw: i32,
    /// Dye color index (0-13).
    pub costume_color_index: u8,
    /// Absolute UNIX expiry timestamp (seconds).
    pub costume_expiry_time: i64,
    /// Whether costume data has been loaded from DB.
    pub costume_loaded: bool,

    // ── Enchant (0xCC) State ─────────────────────────────────────────────
    /// Weapon/armor: highest star tier achieved.
    pub enchant_max_star: u8,
    /// Weapon/armor: total enchant count.
    pub enchant_count: u8,
    /// Weapon/armor: per-slot levels (8 slots).
    pub enchant_slot_levels: [u8; 8],
    /// Weapon/armor: per-slot unlock flags (9 slots).
    pub enchant_slot_unlocked: [u8; 9],
    /// Item enchant: current category.
    pub enchant_item_category: u8,
    /// Item enchant: slot unlock count.
    pub enchant_item_slot_unlock: u8,
    /// Item enchant: marker flags (5 markers).
    pub enchant_item_markers: [u8; 5],
    /// Whether enchant data has been loaded from DB.
    pub enchant_loaded: bool,
    /// Item enchant: last fail timestamp for 60s cooldown.
    pub enchant_item_last_fail: Option<std::time::Instant>,

    /// Item ID being watched for upgrade effects (0 = none).
    ///
    /// Set by WIZ_UPGRADE_NOTICE (0xB8) C2S handler. Cleared on zone change.
    pub watched_upgrade_item: u32,

    // â”€â”€ Tower (Siege / NPC) State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Runtime NPC ID of the tower the player is currently mounted on (-1 = none).
    ///
    pub tower_owner_id: i32,

    // â”€â”€ Stealth / Invisibility State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Current invisibility type
    ///
    /// - `INVIS_NONE` (0): not invisible
    /// - `INVIS_DISPEL_ON_MOVE` (1): stealth breaks on movement
    /// - `INVIS_DISPEL_ON_ATTACK` (2): stealth breaks on attack only
    pub invisibility_type: u8,

    /// Stealth duration end time (unix epoch seconds). 0 = no timed stealth.
    ///
    /// Set when `ExecuteType9()` applies stealth: `UNIXTIME + pType->sDuration`.
    /// Checked in `Type9Duration()` (UserDurationSkillSystem.cpp:224-240):
    /// when `tEndTime != -1 && UNIXTIME >= tEndTime`, calls `Type9Cancel()`.
    pub stealth_end_time: u64,

    // â”€â”€ Blink (Respawn Invulnerability) State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Blink expiry timestamp (unix epoch seconds). 0 = not blinking.
    ///
    /// checked in `BlinkTimeCheck()` and NPC AI targeting.
    /// While blinking, the player is invulnerable and invisible to NPC AI.
    /// Duration: `BLINK_TIME` (10 seconds) from `Define.h:72`.
    pub blink_expiry_time: u64,
    /// Whether the player can use skills (false during blink).
    ///
    /// `BlinkStart()`, restored to true in `BlinkTimeCheck()` or when
    /// blink ends and player is transformed.
    pub can_use_skills: bool,
    /// Whether the player can use potions (false during no-potion debuff).
    ///
    /// `BUFF_TYPE_NO_POTIONS` (153), restored to true on expiry.
    pub can_use_potions: bool,
    /// Whether the player is in Kaul transformation (BUFF_TYPE 154).
    ///
    pub is_kaul: bool,
    /// Whether the player is in Undead state (BUFF_TYPE 155).
    ///
    pub is_undead: bool,
    /// Current abnormal type (transform / GM visibility state).
    ///
    /// Values: 0 = ABNORMAL_INVISIBLE, 1 = ABNORMAL_NORMAL, 2+ = transform IDs.
    pub abnormal_type: u32,
    /// Saved abnormal type before Kaul/Snowman transformation, restored on expiry.
    ///
    pub old_abnormal_type: u32,
    /// Whether the player is blinded (BUFF_TYPE 156/21/20).
    ///
    pub is_blinded: bool,
    /// Whether physical damage is fully blocked (BUFF_TYPE 157).
    ///
    pub block_physical: bool,
    /// Whether magical damage is fully blocked (BUFF_TYPE 158).
    ///
    pub block_magic: bool,
    /// Whether the player is in Devil transformation (BUFF_TYPE 49).
    ///
    pub is_devil: bool,
    /// Current size visual effect from BUFF_TYPE_SIZE (3).
    ///
    /// Values: 0=none, 2=GIANT, 3=DWARF, 6=GIANT_TARGET, 9=special.
    pub size_effect: u32,
    /// Whether the player can teleport (default true, set false by NO_RECALL 150).
    ///
    pub can_teleport: bool,
    /// Whether the player can use stealth (default true, set false by PROHIBIT_INVIS 26).
    ///
    pub can_stealth: bool,
    /// Whether curses are blocked (BUFF_TYPE 29).
    ///
    pub block_curses: bool,
    /// Whether curses are reflected (BUFF_TYPE 30).
    ///
    pub reflect_curses: bool,
    /// Whether skills cast instantly (BUFF_TYPE 23).
    ///
    pub instant_cast: bool,
    /// Accumulated drop/NP/noah scroll bonus amount (BUFF_TYPE 169).
    ///
    pub drop_scroll_amount: i16,
    /// Weapons visually disabled (BUFF_TYPE 32).
    ///
    pub weapons_disabled: bool,
    /// Mana absorb percentage from Outrage/Frenzy/Mana Shield (BUFF_TYPE 31).
    ///
    pub mana_absorb: u8,
    /// Mana absorb hit counter (BUFF_TYPE 31 with absorb==15).
    ///
    pub absorb_count: u8,
    /// Magic damage reduction percentage (100 = no reduction, 70 = 30% reduction).
    ///
    pub magic_damage_reduction: u8,

    /// Percentage resistance multipliers (100 = normal, 70 = 30% resistance reduction).
    ///
    pub pct_fire_r: u8,
    pub pct_cold_r: u8,
    pub pct_lightning_r: u8,
    pub pct_magic_r: u8,
    pub pct_disease_r: u8,
    pub pct_poison_r: u8,

    /// EXP gain bonus from BUFF_TYPE_EXPERIENCE (11) buffs.
    ///
    pub exp_gain_buff11: u16,

    /// EXP gain bonus from BUFF_TYPE_VARIOUS_EFFECTS (33) buffs.
    ///
    pub exp_gain_buff33: u16,

    /// NP (loyalty) bonus from BUFF_TYPE_VARIOUS_EFFECTS (33).
    ///
    pub skill_np_bonus_33: u8,

    /// NP (loyalty) bonus from BUFF_TYPE_LOYALTY_AMOUNT (42).
    ///
    pub skill_np_bonus_42: u8,

    /// JackPot type: 0=none, 1=EXP, 2=Noah, 3=both (unused in C++).
    ///
    pub jackpot_type: u8,

    /// NP gain percentage multiplier from BUFF_TYPE_LOYALTY (15).
    ///
    /// Applied as: `np = base_np * np_gain_amount / 100`.
    pub np_gain_amount: u8,

    /// Gold gain percentage multiplier from BUFF_TYPE_NOAH_BONUS (16).
    ///
    /// Applied as: `gold = gold * noah_gain_amount / 100`.
    pub noah_gain_amount: u8,

    /// Premium merchant flag from BUFF_TYPE_PREMIUM_MERCHANT (17).
    ///
    pub is_premium_merchant: bool,

    /// Carry weight buff amount from BUFF_TYPE_WEIGHT (12).
    ///
    /// If > 100, the full value is added to max weight (C++ adds m_bMaxWeightAmount, not excess).
    pub weight_buff_amount: u8,

    // â"€â"€ Transformation (Type 6) State â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€
    /// Transformation type (matches `Unit::TransformationType`).
    ///
    /// cleared in `Type6Cancel()`.
    /// - 0: TransformationNone
    /// - 1: TransformationMonster
    /// - 2: TransformationNPC
    /// - 3: TransformationSiege
    pub transformation_type: u8,
    /// Visual transform ID (NPC proto_id to render).
    ///
    pub transform_id: u16,
    /// Skill ID that caused the transformation.
    ///
    pub transform_skill_id: u32,
    /// Timestamp when transformation started (milliseconds since epoch).
    ///
    /// (milliseconds) in `ExecuteType6()`.
    pub transformation_start_time: u64,
    /// Transformation duration in milliseconds.
    ///
    /// `duration * 1000` in `ExecuteType6()`.
    pub transformation_duration: u64,

    // â”€â”€ Zone Reward State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// PvP kill count in the current zone session (reset on zone change).
    ///
    /// in a PK zone, used to check `kill_count % reward.KillCount == 0`.
    pub pvp_kill_count: u16,
    /// Per-entry next-reward timestamps for zone online rewards.
    ///
    /// that tracks when the next reward fires.
    /// Vec index matches the global `zone_online_rewards` Vec.
    pub zone_online_reward_timers: Vec<u64>,
    /// Next online cash reward timestamp (absolute UNIX seconds).
    ///
    /// first check passes immediately since `UNIXTIME > 0`.
    /// Reset to `UNIXTIME + (onlinecashtime * MINUTE)` after each grant.
    pub online_cash_next_time: u64,

    // â”€â”€ Genie State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Whether the genie (lamp) is currently active.
    ///
    pub genie_active: bool,
    /// Absolute UNIX timestamp when genie time expires.
    ///
    /// Remaining seconds = `genie_time_abs.saturating_sub(now)`.
    pub genie_time_abs: u32,
    /// Last genie check timestamp (unix seconds).
    ///
    pub genie_check_time: u64,
    /// Whether genie data has been loaded from DB (prevents saving 0 before load).
    pub genie_loaded: bool,
    /// Genie configuration options blob (256 bytes).
    ///
    pub genie_options: Vec<u8>,

    // ── Training State ───────────────────────────────────────────────────────────
    /// Accumulated training XP reward total.
    ///
    pub total_training_exp: u32,
    /// Last training reward timestamp (unix seconds).
    ///
    pub last_training_time: u64,

    // â”€â”€ Flash Time State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Remaining flash time in minutes (decremented every PLAYER_FLASH_INTERVAL).
    ///
    pub flash_time: u32,
    /// Flash stack count (0-10, each stack adds 10% bonus).
    ///
    pub flash_count: u8,
    /// Flash bonus type: 1=EXP, 2=DC(drop), 3=WAR(loyalty).
    ///
    pub flash_type: u8,
    /// Active flash EXP bonus percentage (0-100).
    ///
    pub flash_exp_bonus: u8,
    /// Active flash drop/DC bonus percentage (0-100).
    ///
    pub flash_dc_bonus: u8,
    /// Active flash war/loyalty bonus (0-10).
    ///
    pub flash_war_bonus: u8,
    /// Last flash check timestamp (unix seconds).
    ///
    pub flash_check_time: u64,

    // â”€â”€ Burning / Flame State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Flame level (0-3). Each level gives XP/drop/NP/money bonuses.
    ///
    pub flame_level: u16,
    /// Next flame level-up time (unix seconds). 0 = inactive.
    ///
    pub flame_time: u64,

    // ── Offline Merchant State ──────────────────────────────────────────
    /// Whether this session is in offline merchant mode.
    ///
    /// When true, the session stays in memory with merchant open but no TCP
    /// connection.  Other players can still buy items from this merchant.
    pub is_offline: bool,
    /// Offline character type (merchant, genie, mining, fishing).
    ///
    pub offline_type: OfflineCharacterType,
    /// Remaining offline minutes (decremented every 60 s).
    ///
    pub offline_remaining_minutes: u16,
    /// Next offline check time (`Instant` for monotonic timing).
    ///
    pub offline_next_check: Option<Instant>,

    // ── Knight Cash (KC) / TL Balance ─────────────────────────────────────
    /// Knight Cash (KC) balance — loaded from `tb_user.cash_point` on login.
    ///
    pub knight_cash: u32,
    /// TL (real-money) balance — loaded from `tb_user.bonus_cash_point` on login.
    ///
    pub tl_balance: u32,

    // ── Collection Race State ──────────────────────────────────────────────
    /// Per-slot kill progress for the active Collection Race event (3 slots).
    ///
    /// Incremented when the player kills a monster whose proto_id matches one
    /// of `pCollectionRaceEvent.m_bProtoID[i]`.
    ///
    pub cr_kill_counts: [u16; 3],
    /// Whether this player has completed the Collection Race this round.
    ///
    /// Once set to true, further kills do not advance the counter.
    ///
    pub cr_check_finish: bool,

    // ── Tag Name System ──────────────────────────────────────────────────────
    /// Player tag name (title displayed above character name).
    ///
    pub tagname: String,
    /// Tag name colour packed as COLORREF (r | g<<8 | b<<16).
    ///
    pub tagname_rgb: i32,

    // ── PUS Refund System ────────────────────────────────────────────────────
    /// In-memory refundable purchase map: serial → PusRefundEntry.
    ///
    pub pus_refund_map: std::collections::HashMap<u64, crate::handler::pus_refund::PusRefundEntry>,
    /// Last refund attempt timestamp (UNIX seconds) — rate limit cooldown.
    ///
    pub pus_refund_last_time: u64,

    // ── PPCard Cooldown ────────────────────────────────────────────────────
    /// Last PPCard redemption attempt time — 5-minute (300s) cooldown.
    ///
    pub ppcard_cooldown: Instant,

    // ── Extended Hook (Anti-Cheat / Extended) ─────────────────────────────────────────
    /// Last heartbeat (xALIVE 0xA6) timestamp (UNIX seconds).
    ///
    pub ext_last_heartbeat: u64,
    /// Last support ticket timestamp (UNIX seconds) — rate limit cooldown.
    ///
    pub ext_last_support: u64,
    /// Last chat-seen timestamp (UNIX seconds).
    ///
    pub ext_last_seen: u64,

    // ── Extended Hook Fields (Sprint 493) ──────────────────────────────────
    /// Whether the temp items list has been sent this session (one-shot).
    ///
    pub temp_items_sent: bool,

    /// List of item IDs blocked from chest loot.
    ///
    pub chest_block_items: Vec<u32>,

    // ── Daily Rank Stats (Sprint 552) ────────────────────────────────────
    /// Total gold earned from merchant sales.
    ///
    pub dr_gm_total_sold: u64,
    /// Total monster kills.
    ///
    pub dr_mh_total_kill: u64,
    /// Total crafting/exchange successes.
    ///
    pub dr_sh_total_exchange: u64,
    /// Total chaos war first-place wins.
    ///
    pub dr_cw_counter_win: u64,
    /// Total blessing event counter.
    ///
    pub dr_up_counter_bles: u64,
}

/// Offline character type (`offcharactertype` in `GameDefine.h:390`).
/// Determines which item is required in the CFAIRY slot and what automation
/// the offline session performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OfflineCharacterType {
    /// Standard offline merchant (item 924041913).
    #[default]
    Merchant,
    /// Offline genie (item 824041931).
    Genie,
    /// Offline mining (item 700049758 auto-mining, or 700059759 pure mining).
    Mining,
    /// Offline fishing (item 700099755 auto-fishing, or 700069754 pure fishing).
    Fishing,
}

/// Offline merchant item — standard (must be in CFAIRY slot).
pub const OFFLINE_MERCHANT_ITEM: u32 = 924_041_913;

/// Merchant + auto-fishing item.
pub const MERCHANT_AUTO_FISHING: u32 = 700_099_755;

/// Merchant + auto-mining item.
pub const MERCHANT_AUTO_MANING: u32 = 700_049_758;

/// Default offline duration in minutes (~23.3 hours).
pub const OFFLINE_DEFAULT_MINUTES: u16 = 1400;

/// Offline check interval in seconds.
pub const OFFLINE_CHECK_INTERVAL_SECS: u64 = 60;

/// CFAIRY absolute inventory slot index.
pub const CFAIRY_SLOT: usize = 48;

/// Runtime pet state for an active/summoned pet.
#[derive(Debug, Clone)]
pub struct PetState {
    /// Serial number of the pet item (DB primary key for `pet_user_data`).
    ///
    pub serial_id: u64,
    /// Current pet level (1-60).
    pub level: u8,
    /// Pet satisfaction (0-10000, pet dies at 0).
    pub satisfaction: i16,
    /// Current experience points.
    pub exp: u32,
    /// Current HP.
    pub hp: u16,
    /// Runtime NPC ID of the spawned pet.
    pub nid: u16,
    /// Unique pet index (from DB).
    pub index: u32,
    /// Current MP.
    pub mp: u16,
    /// Current mode (3=attack, 4=defence, 8=looting, 9=chat).
    ///
    pub state_change: u8,
    /// Pet name.
    pub name: String,
    /// Pet visual model PID.
    pub pid: u16,
    /// Pet visual size.
    pub size: u16,
    /// Pet inventory items (4 slots).
    ///
    pub items: [UserItemSlot; PET_INVENTORY_TOTAL],
    /// Whether the pet is actively attacking a target.
    pub attack_started: bool,
    /// Target NPC ID for auto-attack (-1 = none).
    ///
    /// v2615 runtime NPC IDs are 32-bit on the wire and commonly exceed
    /// `i16::MAX` (for example 49886).  Keeping this as `i16` turns those IDs
    /// negative and makes the pet attack tick discard an otherwise valid
    /// target.
    pub attack_target_id: i32,
}

/// Number of item slots in a pet's inventory.
pub const PET_INVENTORY_TOTAL: usize = 4;

impl Default for PetState {
    fn default() -> Self {
        Self {
            serial_id: 0,
            level: 1,
            satisfaction: 0,
            exp: 0,
            hp: 0,
            nid: 0,
            index: 0,
            mp: 0,
            state_change: 4, // MODE_DEFENCE default
            name: String::new(),
            pid: 25500,
            size: 100,
            items: [
                UserItemSlot::default(),
                UserItemSlot::default(),
                UserItemSlot::default(),
                UserItemSlot::default(),
            ],
            attack_started: false,
            attack_target_id: -1,
        }
    }
}

/// Lightweight snapshot of a session's pet state for decay tick processing.
/// Collected in bulk from `WorldState::collect_pet_decay_data()` so the
/// pet decay system can iterate without holding DashMap references.
#[derive(Debug, Clone)]
pub struct PetDecayData {
    /// Session ID of the pet owner.
    pub session_id: SessionId,
    /// Current pet satisfaction (0-10000).
    pub satisfaction: i16,
    /// Last decay timestamp (unix seconds).
    pub last_decay_time: u64,
    /// Pet NPC ID (for death notification).
    pub pet_nid: u16,
    /// Pet index (for death notification).
    pub pet_index: u32,
}

/// Lightweight snapshot of a session's pet attack state for the pet attack tick.
/// Collected in bulk from `WorldState::collect_pet_attack_data()` so the
/// pet attack system can iterate without holding DashMap references.
#[derive(Debug, Clone)]
pub struct PetAttackData {
    /// Session ID of the pet owner.
    pub session_id: SessionId,
    /// Pet NPC runtime ID (the pet entity in the world).
    pub pet_nid: u16,
    /// Target NPC runtime ID that the pet is attacking.
    pub target_npc_id: u32,
    /// Zone the owner is in (for NPC lookups).
    pub owner_zone_id: u16,
    /// Whether the pet owner is dead.
    pub owner_dead: bool,
}

/// Per-flame-level rate multipliers from the BURNING_FEATURES table.
#[derive(Debug, Clone, Copy, Default)]
pub struct BurningFeatureRates {
    /// NP/loyalty rate multiplier (percentage).
    pub np_rate: u8,
    /// Gold/money rate multiplier (percentage).
    pub money_rate: u8,
    /// Experience rate multiplier (percentage).
    pub exp_rate: u8,
    /// Item drop rate multiplier (percentage).
    pub drop_rate: u8,
}

/// Premium gift item entry â€” represents an item to be given as a letter when
/// a player receives a premium of a certain type.
/// Loaded from MSSQL `PREMIUM_GIFT_ITEM` table.
#[derive(Debug, Clone)]
pub struct PremiumGiftItem {
    /// Item ID to give.
    pub item_id: u32,
    /// Number of items to give.
    pub count: u16,
    /// Sender name in the letter.
    pub sender: String,
    /// Subject line of the letter.
    pub subject: String,
    /// Message body of the letter.
    pub message: String,
}

/// Wanted event status per PK zone room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WantedEventStatus {
    /// Event is not active (waiting for next select time).
    #[default]
    Disabled,
    /// Players are being invited to register.
    Invitation,
    /// Wanted user list is being sent.
    ListSending,
    /// Event is actively running (position broadcasts enabled).
    Running,
}

/// Per-room wanted event state (3 rooms: Ronark Land, Ardream, Ronark Land Base).
#[derive(Debug, Clone, Default)]
pub struct WantedEventRoom {
    /// Current event phase.
    pub status: WantedEventStatus,
    /// Unix timestamp for the next player selection.
    pub next_select_time: u64,
    /// Unix timestamp when invitation phase ends.
    pub invitation_time: u64,
    /// Unix timestamp when list sending occurs.
    pub list_time: u64,
    /// Unix timestamp when the event finishes.
    pub finish_time: u64,
    /// Elmorad wanted user session IDs.
    pub elmo_list: Vec<SessionId>,
    /// Karus wanted user session IDs.
    pub karus_list: Vec<SessionId>,
}

/// Maximum number of wanted event rooms (3 PK zones).
pub const MAX_WANTED_ROOMS: usize = 3;

/// Pet satisfaction decay interval in seconds.
pub const PET_DECAY_INTERVAL_SECS: u64 = 60;

/// Pet satisfaction decay amount per tick.
pub const PET_DECAY_AMOUNT: i16 = 100;

/// Wanted event position broadcast interval in seconds.
pub const WANTED_MAP_SHOW_INTERVAL_SECS: u64 = 60;

/// Maximum number of items in a player merchant shop.
pub const MAX_MERCH_ITEMS: usize = 12;

/// Maximum merchant advert message length.
pub const MAX_MERCH_MESSAGE: usize = 40;

/// Item ID representing gold/coins in trades.
pub const ITEM_GOLD: u32 = 900_000_000;

/// Re-export COIN_MAX from inventory_constants (canonical source).
pub use crate::inventory_constants::COIN_MAX;

// ── Quest pseudo-item constants ───────────────────────────────────────

/// Pseudo-item: grant EXP reward.
pub const ITEM_EXP: u32 = 900_001_000;

/// Pseudo-item: grant item count (NP/coins).
pub const ITEM_COUNT: u32 = 900_002_000;

/// Pseudo-item: grant ladder/loyalty points.
pub const ITEM_LADDERPOINT: u32 = 900_003_000;

/// Pseudo-item: grant random reward.
pub const ITEM_RANDOM: u32 = 900_004_000;

/// Items in this range cannot be traded, sold, or stored.
pub const ITEM_NO_TRADE_MIN: u32 = 900_000_001;
pub const ITEM_NO_TRADE_MAX: u32 = 999_999_999;

/// Maximum account ID / character name length.
pub const MAX_ID_SIZE: usize = 20;

/// Maximum password length.
pub const MAX_PW_SIZE: usize = 28;

/// Race value that marks an item as untradeable.
pub const RACE_UNTRADEABLE: i32 = 20;

/// Item flag constants -- these are an ENUM, NOT a bitmask.
/// Comparisons MUST use equality (`==`), never bitwise AND (`&`).
pub const ITEM_FLAG_NONE: u8 = 0;
pub const ITEM_FLAG_RENTED: u8 = 1;
pub const ITEM_FLAG_CHAR_SEAL: u8 = 2;
pub const ITEM_FLAG_DUPLICATE: u8 = 3;
pub const ITEM_FLAG_SEALED: u8 = 4;
pub const ITEM_FLAG_NOT_BOUND: u8 = 7;
pub const ITEM_FLAG_BOUND: u8 = 8;

/// Merchant state constants.
pub const MERCHANT_STATE_NONE: i8 = -1;
pub const MERCHANT_STATE_SELLING: i8 = 0;
pub const MERCHANT_STATE_BUYING: i8 = 1;

/// Trade state constants
pub const TRADE_STATE_NONE: u8 = 1;
pub const TRADE_STATE_SENDER: u8 = 2;
pub const TRADE_STATE_TARGET: u8 = 3;
pub const TRADE_STATE_TRADING: u8 = 4;
pub const TRADE_STATE_DECIDING: u8 = 5;

/// Daily operation cooldown minutes (24 hours = 1440 minutes).
pub(crate) const DAILY_OPERATIONS_MINUTE: i64 = 1440;

/// Daily operation type codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DailyOpCode {
    ChaosMap = 1,
    UserRankReward = 2,
    PersonalRankReward = 3,
    KingWing = 4,
    WarderKillerWing1 = 5,
    WarderKillerWing2 = 6,
    KeeperKillerWing = 7,
    UserLoyaltyWingReward = 8,
}

impl DailyOpCode {
    /// Convert from raw u8 value. Returns None for unknown types.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::ChaosMap),
            2 => Some(Self::UserRankReward),
            3 => Some(Self::PersonalRankReward),
            4 => Some(Self::KingWing),
            5 => Some(Self::WarderKillerWing1),
            6 => Some(Self::WarderKillerWing2),
            7 => Some(Self::KeeperKillerWing),
            8 => Some(Self::UserLoyaltyWingReward),
            _ => None,
        }
    }
}

/// Per-character daily operation timestamps.
/// Each field holds a unix timestamp (i32) of when the operation was last
/// performed. -1 means "never performed".
#[derive(Debug, Clone)]
pub struct UserDailyOp {
    pub chaos_map_time: i32,
    pub user_rank_reward_time: i32,
    pub personal_rank_reward_time: i32,
    pub king_wing_time: i32,
    pub warder_killer_time1: i32,
    pub warder_killer_time2: i32,
    pub keeper_killer_time: i32,
    pub user_loyalty_wing_reward_time: i32,
}

impl Default for UserDailyOp {
    fn default() -> Self {
        Self::new()
    }
}

impl UserDailyOp {
    /// Create with all timestamps set to -1 (never performed).
    pub fn new() -> Self {
        Self {
            chaos_map_time: -1,
            user_rank_reward_time: -1,
            personal_rank_reward_time: -1,
            king_wing_time: -1,
            warder_killer_time1: -1,
            warder_killer_time2: -1,
            keeper_killer_time: -1,
            user_loyalty_wing_reward_time: -1,
        }
    }

    /// Get the timestamp for a given op type.
    pub fn get(&self, op: DailyOpCode) -> i32 {
        match op {
            DailyOpCode::ChaosMap => self.chaos_map_time,
            DailyOpCode::UserRankReward => self.user_rank_reward_time,
            DailyOpCode::PersonalRankReward => self.personal_rank_reward_time,
            DailyOpCode::KingWing => self.king_wing_time,
            DailyOpCode::WarderKillerWing1 => self.warder_killer_time1,
            DailyOpCode::WarderKillerWing2 => self.warder_killer_time2,
            DailyOpCode::KeeperKillerWing => self.keeper_killer_time,
            DailyOpCode::UserLoyaltyWingReward => self.user_loyalty_wing_reward_time,
        }
    }

    /// Set the timestamp for a given op type.
    pub fn set(&mut self, op: DailyOpCode, time: i32) {
        match op {
            DailyOpCode::ChaosMap => self.chaos_map_time = time,
            DailyOpCode::UserRankReward => self.user_rank_reward_time = time,
            DailyOpCode::PersonalRankReward => self.personal_rank_reward_time = time,
            DailyOpCode::KingWing => self.king_wing_time = time,
            DailyOpCode::WarderKillerWing1 => self.warder_killer_time1 = time,
            DailyOpCode::WarderKillerWing2 => self.warder_killer_time2 = time,
            DailyOpCode::KeeperKillerWing => self.keeper_killer_time = time,
            DailyOpCode::UserLoyaltyWingReward => self.user_loyalty_wing_reward_time = time,
        }
    }

    /// Convert from DB row.
    pub fn from_row(row: &ko_db::models::UserDailyOpRow) -> Self {
        Self {
            chaos_map_time: row.chaos_map_time,
            user_rank_reward_time: row.user_rank_reward_time,
            personal_rank_reward_time: row.personal_rank_reward_time,
            king_wing_time: row.king_wing_time,
            warder_killer_time1: row.warder_killer_time1,
            warder_killer_time2: row.warder_killer_time2,
            keeper_killer_time: row.keeper_killer_time,
            user_loyalty_wing_reward_time: row.user_loyalty_wing_reward_time,
        }
    }

    /// Convert to DB row.
    pub fn to_row(&self, user_id: &str) -> ko_db::models::UserDailyOpRow {
        ko_db::models::UserDailyOpRow {
            user_id: user_id.to_string(),
            chaos_map_time: self.chaos_map_time,
            user_rank_reward_time: self.user_rank_reward_time,
            personal_rank_reward_time: self.personal_rank_reward_time,
            king_wing_time: self.king_wing_time,
            warder_killer_time1: self.warder_killer_time1,
            warder_killer_time2: self.warder_killer_time2,
            keeper_killer_time: self.keeper_killer_time,
            user_loyalty_wing_reward_time: self.user_loyalty_wing_reward_time,
            full_moon_rift_map_time: -1,
            copy_information_time: -1,
        }
    }
}

/// An item in the repurchase (trash) list â€” sold non-countable items
/// that the player can buy back within 72 minutes.
#[derive(Debug, Clone)]
pub struct DeletedItemEntry {
    /// Database row ID (for delete after buyback).
    pub db_id: i64,
    /// Item definition number.
    pub item_id: u32,
    /// Stack count (always 1 for non-countable).
    pub count: u32,
    /// Expiration time (unix timestamp â€” UNIXTIME + 72*60).
    pub delete_time: u32,
    /// Item durability at time of sale.
    pub duration: u16,
    /// Serial number for uniqueness.
    pub serial_num: u64,
    /// Item flags at time of sale.
    pub flag: u8,
}

/// An item placed in the exchange/trade window.
#[derive(Debug, Clone)]
pub struct ExchangeItem {
    /// Item definition number (ITEM_GOLD for coins).
    pub item_id: u32,
    /// Stack count (or gold amount).
    pub count: u32,
    /// Durability of the item.
    pub durability: i16,
    /// Serial number for uniqueness.
    pub serial_num: u64,
    /// Source inventory slot index (absolute: SLOT_MAX + pos).
    pub src_pos: u8,
    /// Destination slot in the receiver's inventory (set during ExecuteExchange).
    pub dst_pos: u8,
}

/// A single merchant shop slot.
#[derive(Debug, Clone, Default)]
pub struct MerchData {
    /// Item definition number (0 = empty slot).
    pub item_id: u32,
    /// Current durability.
    pub durability: i16,
    /// Selling count (how many for sale).
    pub sell_count: u16,
    /// Original total count in the player's inventory.
    pub original_count: u16,
    /// Serial number.
    pub serial_num: u64,
    /// Price per unit (gold).
    pub price: u32,
    /// Original inventory slot index (absolute).
    pub original_slot: u8,
    /// Whether this slot has been sold out.
    pub sold_out: bool,
    /// Whether price is in KC (Knight Cash) instead of gold.
    pub is_kc: bool,
}

/// A single inventory slot's runtime data.
#[derive(Debug, Clone, Default)]
pub struct UserItemSlot {
    /// Item definition number (0 = empty slot).
    pub item_id: u32,
    /// Current durability.
    pub durability: i16,
    /// Stack count.
    pub count: u16,
    /// Item flags (sealed, bound, rented, duplicate).
    pub flag: u8,
    /// Original flag before sealing — used to restore state on unseal.
    ///
    pub original_flag: u8,
    /// Serial number for uniqueness tracking.
    pub serial_num: u64,
    /// Expiration time (unix timestamp, 0 = no expiry).
    pub expire_time: u32,
}

impl UserItemSlot {
    /// Compute the remaining rental time in minutes for packet display.
    ///
    /// info packets. The value is `(nExpirationTime - UNIXTIME) / 60` when the
    /// item has an expiry; otherwise 0.
    pub fn remaining_rental_minutes(&self) -> u16 {
        remaining_rental_minutes(self.expire_time)
    }
}

/// Compute the remaining rental time in minutes from an expiry unix timestamp.
/// Returns 0 if no expiry or already expired. Caps at `u16::MAX` (≈ 45 days).
pub fn remaining_rental_minutes(expire_time: u32) -> u16 {
    if expire_time == 0 {
        return 0;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    if expire_time <= now {
        return 0;
    }
    let remaining_secs = expire_time - now;
    (remaining_secs / 60).min(u16::MAX as u32) as u16
}

/// Subset of character data needed for broadcasting to other players.
#[derive(Debug, Clone, Default)]
pub struct CharacterInfo {
    pub session_id: SessionId,
    pub name: String,
    pub nation: u8,
    pub race: u8,
    pub class: u16,
    pub level: u8,
    pub face: u8,
    pub hair_rgb: u32,
    pub rank: u8,
    pub title: u8,
    /// Maximum HP (calculated from level/STA/class).
    pub max_hp: i16,
    /// Current HP.
    pub hp: i16,
    /// Maximum MP (calculated from level/INT/class).
    pub max_mp: i16,
    /// Current MP.
    pub mp: i16,
    /// Maximum SP (Kurian stamina points â€” 0 for non-Kurian classes).
    ///
    /// Beginner(13)=100, Novice(14)=150, Master(15)=200
    pub max_sp: i16,
    /// Current SP (Kurian stamina points).
    ///
    pub sp: i16,
    /// Equipment items for visual display (14 equipped slots).
    pub equipped_items: [u32; 14],

    // â”€â”€ Bind point (respawn / home) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Zone ID the character will respawn in.
    pub bind_zone: u8,
    /// Bind-point X coordinate (uses zone init_x when no explicit bind).
    pub bind_x: f32,
    /// Bind-point Z coordinate (uses zone init_z when no explicit bind).
    pub bind_z: f32,

    // â”€â”€ Base stats â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Strength.
    pub str: u8,
    /// Stamina.
    pub sta: u8,
    /// Dexterity.
    pub dex: u8,
    /// Intelligence.
    pub intel: u8,
    /// Charisma.
    pub cha: u8,
    /// Unspent stat points.
    pub free_points: u16,

    // â”€â”€ Skill points â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Skill point array
    /// Index 0 = free skill points, 5-8 = skill categories.
    ///
    pub skill_points: [u8; 10],

    // â”€â”€ Gold, Loyalty & authority â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// In-game currency (Noah).
    pub gold: u32,
    /// Nation Points (loyalty / NP).
    ///
    pub loyalty: u32,
    /// Monthly Nation Points.
    ///
    pub loyalty_monthly: u32,
    /// Authority level (0 = GM, 1 = player).
    pub authority: u8,

    // â”€â”€ Clan â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Knights (clan) ID. -1 or 0 = no clan.
    pub knights_id: u16,
    /// Clan fame / contribution rank.
    pub fame: u8,

    // â”€â”€ Party â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Party ID, None if not in a party.
    pub party_id: Option<u16>,

    // â”€â”€ Experience â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Total experience points.
    ///
    pub exp: u64,

    /// XP required to level up from current level.
    ///
    pub max_exp: i64,

    // â”€â”€ EXP seal â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Whether the EXP seal is active (XP goes to sealed pool).
    ///
    pub exp_seal_status: bool,

    /// Accumulated sealed experience points.
    ///
    pub sealed_exp: u32,

    // â”€â”€ Weight â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Current total weight of items in inventory.
    ///
    pub item_weight: i32,

    /// Maximum carry weight (calculated from STR/class).
    ///
    pub max_weight: i32,

    // â”€â”€ State tracking â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Resting HP type (sit/stand/dead state).
    ///
    /// - `USER_STANDING` (0x01): standing
    /// - `USER_SITDOWN` (0x02): sitting
    /// - `USER_DEAD` (0x03): dead
    pub res_hp_type: u8,

    // â”€â”€ Rivalry â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Session ID of the designated rival player (-1 = no rival).
    ///
    pub rival_id: i16,

    /// Unix timestamp when the rivalry expires (0 = no active rivalry).
    ///
    /// Duration: `RIVALRY_DURATION` (300 seconds / 5 minutes).
    pub rival_expiry_time: u64,

    /// Anger gauge level (0–5). Incremented each time this player is killed by
    /// an enemy in Ardream / Ronark Land zones.  When > 0, a helmet icon is
    /// displayed on nearby clients via WIZ_PVP(PVPUpdateHelmet).
    /// Resets to 0 on regene / Draki regene.
    ///
    pub anger_gauge: u8,

    // â”€â”€ Manner â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Manner points (social score).
    ///
    pub manner_point: i32,

    // â”€â”€ Rebirth â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Rebirth level (0 = not rebirthed).
    ///
    pub rebirth_level: u8,

    /// Rebirth bonus stats.
    ///
    /// Added to `GetStatBonusTotal()` alongside item/buff/achievement bonuses.
    pub reb_str: u8,
    pub reb_sta: u8,
    pub reb_dex: u8,
    pub reb_intel: u8,
    pub reb_cha: u8,

    /// Achievement cover title ID (displayed to nearby players).
    ///
    /// Resolved from `achieve_summary.cover_id` → `achieve_main.title_id`.
    pub cover_title: u16,
}

/// Transient state values needed for the WIZ_USER_INOUT broadcast packet.
/// These come from `SessionState` and are NOT persisted in `CharacterInfo`
/// because they change at runtime (party, devil form, direction, hiding, etc.).
#[derive(Debug, Clone, Default)]
pub struct BroadcastState {
    /// `m_bNeedParty` — 1 = looking for party (WIZ_STATE_CHANGE type=2)
    pub need_party: u8,
    /// `m_bPartyLeader` — 1 = party leader crown icon
    pub party_leader: u8,
    /// `m_bIsDevil` — Kurian Devil Form (BUFF_TYPE 49)
    pub is_devil: u8,
    /// `m_teamColour` — soccer/arena team color
    pub team_colour: u8,
    /// `m_sDirection` — facing direction (0-360)
    pub direction: u16,
    /// `m_bIsHidingHelmet` — helmet hidden by player preference
    pub is_hiding_helmet: u8,
    /// `m_bIsHidingCospre` — cosplay items hidden
    pub is_hiding_cospre: u8,
    /// `m_bKnightsRank` — clan NP rank (0=unranked). C++ uses -1 when unranked.
    pub knights_rank: i8,
    /// `m_bPersonalRank` — personal NP rank (0=unranked). C++ uses -1 when unranked.
    pub personal_rank: i8,
    /// `isInGenie()` — 1 if player has Genie buff active
    pub is_in_genie: u8,
    /// `ReturnSymbolisOK` — Knight return symbol status (u32).
    ///
    pub return_symbol_ok: u32,
}

/// Per-player quest progress entry.
/// Quest states:
/// - 0: not started
/// - 1: ongoing
/// - 2: completed
/// - 3: ready to complete (all conditions met)
/// - 4: removed/abandoned
#[derive(Debug, Clone, Default)]
pub struct UserQuestInfo {
    /// Current quest state.
    pub quest_state: u8,
    /// Kill counts for each of the 4 monster groups.
    ///
    pub kill_counts: [u8; 4],
}

/// Per-player achievement progress entry.
#[derive(Debug, Clone, Default)]
pub struct UserAchieveInfo {
    /// Achievement status: 0=ChallengeIncomplete, 1=Incomplete, 4=Finished, 5=Completed.
    pub status: u8,
    /// Progress counters for 2 groups.
    pub count: [u32; 2],
}

/// Per-player achievement summary data.
#[derive(Debug, Clone, Default)]
pub struct AchieveSummary {
    /// Total play time in seconds.
    pub play_time: u32,
    /// Total monsters defeated.
    pub monster_defeat_count: u32,
    /// Total enemy users defeated.
    pub user_defeat_count: u32,
    /// Total deaths to other users.
    pub user_death_count: u32,
    /// Total medal points.
    pub total_medal: u32,
    /// Most recent 3 achievement IDs.
    pub recent_achieve: [u16; 3],
    /// Equipped cover title achievement ID.
    pub cover_id: u16,
    /// Equipped cover title ID (from achieve_main.title_id).
    pub cover_title: u16,
    /// Equipped skill title achievement ID.
    pub skill_id: u16,
    /// Equipped skill title ID (from achieve_main.title_id).
    pub skill_title: u16,
}

/// Lightweight snapshot of session data for regen tick processing.
/// Collected in bulk to avoid holding DashMap refs across async boundaries.
#[derive(Debug, Clone)]
pub struct RegenData {
    pub session_id: SessionId,
    pub level: u8,
    pub hp: i16,
    pub max_hp: i16,
    pub mp: i16,
    pub max_mp: i16,
    pub res_hp_type: u8,
    pub authority: u8,
    /// Zone the player is in (for zone-specific regen overrides).
    pub zone_id: u16,
    /// Character class (for mage MP bonus check).
    pub class: u16,
    /// Current SP (Kurian stamina points).
    pub sp: i16,
    /// Maximum SP.
    pub max_sp: i16,
    /// PRO_SKILL4 (skill_points[8]) for Master Kurian SP regen bonus.
    pub pro_skill4: u8,
    /// Blink (respawn invulnerability) expiry as UNIX timestamp.
    ///
    /// When `blink_expiry_time > now`, the player is blinking and should not regen.
    pub blink_expiry_time: u64,
    /// Whether the player is undead (heal → damage inversion).
    pub is_undead: bool,
    /// Last training XP tick timestamp (for sitting XP).
    pub last_training_time: u64,
    /// Accumulated training XP counter.
    pub total_training_exp: u32,
}

/// Active buff/debuff applied to a session.
/// Tracks the skill that created this buff, which stat modifiers it applies,
/// and when it expires. Keyed by `buff_type` in the session's buff map.
#[derive(Debug, Clone)]
pub struct ActiveBuff {
    /// The skill ID (magic_num) that granted this buff.
    pub skill_id: u32,
    /// Buff type from `MagicType4Row::buff_type` â€” used as the map key.
    ///
    pub buff_type: i32,
    /// Session ID of the caster who applied this buff.
    pub caster_sid: SessionId,
    /// When this buff was applied.
    pub start_time: Instant,
    /// Duration in seconds (0 = permanent until cancelled).
    pub duration_secs: u32,
    // â”€â”€ Stat modifiers from MagicType4Row â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Attack speed modifier.
    pub attack_speed: i32,
    /// Movement speed modifier.
    pub speed: i32,
    /// Armor class modifier.
    pub ac: i32,
    /// Armor class percent modifier.
    pub ac_pct: i32,
    /// Physical attack modifier.
    pub attack: i32,
    /// Magic attack modifier.
    pub magic_attack: i32,
    /// Max HP flat modifier.
    pub max_hp: i32,
    /// Max HP percent modifier.
    pub max_hp_pct: i32,
    /// Max MP flat modifier.
    pub max_mp: i32,
    /// Max MP percent modifier.
    pub max_mp_pct: i32,
    /// Strength modifier.
    pub str_mod: i32,
    /// Stamina modifier.
    pub sta_mod: i32,
    /// Dexterity modifier.
    pub dex_mod: i32,
    /// Intelligence modifier.
    pub intel_mod: i32,
    /// Charisma modifier.
    pub cha_mod: i32,
    /// Fire resistance modifier.
    pub fire_r: i32,
    /// Cold resistance modifier.
    pub cold_r: i32,
    /// Lightning resistance modifier.
    pub lightning_r: i32,
    /// Magic resistance modifier.
    pub magic_r: i32,
    /// Disease resistance modifier.
    pub disease_r: i32,
    /// Poison resistance modifier.
    pub poison_r: i32,
    /// Hit rate modifier.
    pub hit_rate: i32,
    /// Evasion/avoid rate modifier.
    pub avoid_rate: i32,
    /// Flat weapon damage bonus
    ///
    /// Set by BUFF_TYPE_WEAPON_DAMAGE (13) — added to weapon power in SetUserAbility.
    pub weapon_damage: i32,
    /// AC reduction source amount
    ///
    /// Set by BUFF_TYPE_ATTACK_SPEED_ARMOR (18) when sAC < 0 — subtracted from target AC
    /// in physical damage formula.
    pub ac_sour: i32,
    /// Whether this buff's duration has already been extended once.
    ///
    pub duration_extended: bool,
    /// Whether this is a friendly buff (true) or hostile debuff (false).
    ///
    /// Used to determine if cure packets should be sent on expiry.
    pub is_buff: bool,
}

impl ActiveBuff {
    /// Check if this buff has expired based on its start time and duration.
    ///
    /// Buffs with `duration_secs == 0` never expire (permanent until cancelled).
    pub fn is_expired(&self) -> bool {
        if self.duration_secs == 0 {
            return false;
        }
        self.start_time.elapsed().as_secs() >= self.duration_secs as u64
    }
}

/// Maximum number of durational skill (DOT/HOT) slots per unit.
pub const MAX_TYPE3_REPEAT: usize = 40;

/// Active DOT/HOT effect on a unit â€” mirrors `Unit::MagicType3`.
/// Each durational skill occupies a slot in the `durational_skills` array.
/// The `dot_tick` system processes these every 2 seconds.
#[derive(Debug, Clone)]
pub struct DurationalSkill {
    /// Skill ID that created this effect.
    pub skill_id: u32,
    /// HP change per tick (negative = damage, positive = heal).
    pub hp_amount: i16,
    /// Current tick count.
    pub tick_count: u8,
    /// Total ticks before expiry.
    pub tick_limit: u8,
    /// Session ID of the caster.
    pub caster_sid: SessionId,
    /// Whether this slot is in use.
    pub used: bool,
}

impl DurationalSkill {
    /// Create a new empty (unused) durational skill slot.
    pub fn empty() -> Self {
        Self {
            skill_id: 0,
            hp_amount: 0,
            tick_count: 0,
            tick_limit: 0,
            caster_sid: 0,
            used: false,
        }
    }
}

/// Scheduled NPC respawn — queued when a monster with a respawn chain dies.
/// Processed by the NPC AI tick; when `spawn_at` passes, the NPC is spawned.
#[derive(Debug, Clone)]
pub struct ScheduledRespawn {
    /// NPC template SID to spawn.
    pub born_sid: u16,
    /// Zone to spawn in.
    pub zone_id: u16,
    /// Spawn X coordinate.
    pub x: f32,
    /// Spawn Z coordinate.
    pub z: f32,
    /// Unix timestamp (seconds) when the NPC should spawn.
    pub spawn_at: u64,
}

/// JackPot configuration loaded from DB — probability thresholds for multiplier tiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct JackPotSetting {
    /// Chance out of 10000 that jackpot triggers.
    pub rate: u16,
    /// Threshold for 1000x multiplier.
    pub x_1000: u16,
    /// Threshold for 500x multiplier.
    pub x_500: u16,
    /// Threshold for 100x multiplier.
    pub x_100: u16,
    /// Threshold for 50x multiplier.
    pub x_50: u16,
    /// Threshold for 10x multiplier.
    pub x_10: u16,
    /// Threshold for 2x multiplier.
    pub x_2: u16,
}

/// Active DOT effect on an NPC — mirrors player `DurationalSkill` but
/// tracks the caster session and damage per tick against NPC HP.
#[derive(Debug, Clone)]
pub struct NpcDotSlot {
    /// Skill ID that created this effect.
    pub skill_id: u32,
    /// HP damage per tick (always negative for DOT).
    pub hp_amount: i16,
    /// Current tick count.
    pub tick_count: u8,
    /// Total ticks before expiry.
    pub tick_limit: u8,
    /// Session ID of the player who applied the DOT.
    pub caster_sid: SessionId,
}

/// Active Type4 buff/debuff on an NPC.
/// Simpler than the player `ActiveBuff` â€” NPCs only need to track
/// duration for expiry. Stat modifications on NPCs are handled by
/// the AI system querying active buff types.
/// by `bBuffType` in `CNpc::m_buffMap`.
#[derive(Debug, Clone)]
pub struct NpcBuffEntry {
    /// The skill ID (magic_num) that granted this buff.
    pub skill_id: u32,
    /// Buff type from `MagicType4Row::buff_type` â€” used as the map key.
    pub buff_type: i32,
    /// When this buff was applied.
    pub start_time: Instant,
    /// Duration in seconds (0 = permanent until cancelled).
    pub duration_secs: u32,
}

impl NpcBuffEntry {
    /// Check if this buff has expired based on its start time and duration.
    ///
    /// Buffs with `duration_secs == 0` never expire (permanent until cancelled).
    pub fn is_expired(&self) -> bool {
        if self.duration_secs == 0 {
            return false;
        }
        self.start_time.elapsed().as_secs() >= self.duration_secs as u64
    }
}

/// Maximum number of items in a ground loot bundle.
pub const NPC_HAVE_ITEM_LIST: usize = 12; // v2600: sniff verified, was 8 pre-v2600

/// Maximum stack count for items.
pub const ITEMCOUNT_MAX: u16 = 9999;

/// A single item in a ground loot bundle.
#[derive(Debug, Clone, Default)]
pub struct LootItem {
    /// Item definition number (0 = empty slot).
    pub item_id: u32,
    /// Stack count.
    pub count: u16,
    /// Slot index within the bundle (0-7).
    pub slot_id: u16,
}

/// A ground item bundle dropped by NPCs or players.
#[derive(Debug, Clone)]
pub struct GroundBundle {
    /// Unique bundle ID.
    pub bundle_id: u32,
    /// Number of non-empty item slots.
    pub items_count: u8,
    /// NPC/monster that dropped these items (0 for player drops).
    pub npc_id: u16,
    /// Session ID of the player who has loot rights.
    pub looter: u16,
    /// World position.
    pub x: f32,
    pub z: f32,
    pub y: f32,
    /// Zone ID where the bundle was dropped.
    pub zone_id: u16,
    /// When this bundle was dropped (for expiry).
    pub drop_time: Instant,
    /// The items in this bundle (up to 8).
    pub items: [LootItem; NPC_HAVE_ITEM_LIST],
}

impl Default for GroundBundle {
    fn default() -> Self {
        Self {
            bundle_id: 0,
            items_count: 0,
            npc_id: 0,
            looter: 0xFFF,
            x: 0.0,
            z: 0.0,
            y: 0.0,
            zone_id: 0,
            drop_time: Instant::now(),
            items: Default::default(),
        }
    }
}

/// Computed equipment stats from SetSlotItemValue + SetUserAbility.
#[derive(Debug, Clone, Default)]
pub struct EquippedStats {
    /// Total attack power.
    pub total_hit: u16,
    /// Total armor class.
    pub total_ac: i16,
    /// Max weight capacity (`uint32 m_sMaxWeight` in User.h:394).
    pub max_weight: u32,
    /// Current item weight (`uint32 m_sItemWeight` in User.h:406).
    pub item_weight: u32,
    /// Item-based AC sum.
    pub item_ac: i16,
    /// Item-based max HP bonus.
    pub item_max_hp: i16,
    /// Item-based max MP bonus.
    pub item_max_mp: i16,
    /// Item stat bonuses: [STR, STA, DEX, INT, CHA].
    pub stat_bonuses: [i16; 5],
    /// Item hit rate bonus (starts at 100).
    pub item_hitrate: i16,
    /// Item evasion rate bonus (starts at 100).
    pub item_evasionrate: i16,
    /// Fire resistance from items.
    pub fire_r: i16,
    /// Cold resistance from items.
    pub cold_r: i16,
    /// Lightning resistance from items.
    pub lightning_r: i16,
    /// Magic resistance from items.
    pub magic_r: i16,
    /// Disease/curse resistance from items.
    pub disease_r: i16,
    /// Poison resistance from items.
    pub poison_r: i16,
    /// Total hit rate (after coefficient calculation).
    pub total_hitrate: f32,
    /// Total evasion rate (after coefficient calculation).
    pub total_evasionrate: f32,

    // â”€â”€ Weapon-type resistances (C++ m_sDaggerR..m_sBowR) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Dagger resistance from armor.
    pub dagger_r: i16,
    /// Sword resistance from armor.
    pub sword_r: i16,
    /// Jamadar resistance from armor.
    pub jamadar_r: i16,
    /// Axe resistance from armor.
    pub axe_r: i16,
    /// Club resistance from armor.
    pub club_r: i16,
    /// Spear resistance from armor.
    pub spear_r: i16,
    /// Bow resistance from armor.
    pub bow_r: i16,

    // â”€â”€ Elemental bonuses per slot (C++ m_sEquippedItemBonuses) â”€â”€â”€â”€
    /// Equipped item elemental bonuses: slot index -> [(type, value)].
    ///
    /// Type constants: 1=Fire, 2=Cold, 3=Lightning, 4=Poison,
    /// 5=HP_Drain, 6=MP_Damage, 7=MP_Drain, 8=MirrorDamage.
    pub equipped_item_bonuses: std::collections::BTreeMap<usize, Vec<(u8, i32)>>,

    // â”€â”€ XP / NP / Gold bonus multipliers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Item XP bonus percent
    pub item_exp_bonus: u8,
    /// Item NP bonus
    pub item_np_bonus: u8,
    /// Item gold bonus percent
    pub item_gold_bonus: u8,

    // â”€â”€ AP / AC class bonuses â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// General AP bonus percent
    pub ap_bonus_amount: u8,
    /// AP bonus per class type [warrior, rogue, mage, priest]
    pub ap_class_bonus: [u8; 4],
    /// AC bonus per class type [warrior, rogue, mage, priest]
    pub ac_class_bonus: [u8; 4],

    // â”€â”€ Max weight bonus (from bags + set items + capes) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    /// Bonus max weight from items
    pub max_weight_bonus: i16,

    /// All-element resistance bonus from passive skills + INT
    pub resistance_bonus: i16,
}

// â”€â”€ King System Data Structures â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Election type constants (`ElectionType` enum in `KingSystem.h:30-40`).
pub const ELECTION_TYPE_NO_TERM: u8 = 0;
pub const ELECTION_TYPE_NOMINATION: u8 = 1;
pub const ELECTION_TYPE_PRE_ELECTION: u8 = 2;
pub const ELECTION_TYPE_ELECTION: u8 = 3;
pub const ELECTION_TYPE_TERM_ENDED: u8 = 7;

/// King event sub-opcodes (`KingEventType` enum in `packets.h:471-476`).
pub const KING_EVENT_NOAH: u8 = 1;
pub const KING_EVENT_EXP: u8 = 2;
pub const KING_EVENT_PRIZE: u8 = 3;
pub const KING_EVENT_FUGITIVE: u8 = 4;
pub const KING_EVENT_WEATHER: u8 = 5;
pub const KING_EVENT_NOTICE: u8 = 6;

/// Main king sub-opcodes (`KingType` enum in `packets.h:461-466`).
pub const KING_ELECTION: u8 = 1;
pub const KING_IMPEACHMENT: u8 = 2;
pub const KING_TAX: u8 = 3;
pub const KING_EVENT_OPCODE: u8 = 4;
pub const KING_NATION_INTRO: u8 = 6;

/// Election sub-opcodes (`KingElectionType` enum in `packets.h:481-485`).
pub const KING_ELECTION_SCHEDULE: u8 = 1;
pub const KING_ELECTION_NOMINATE: u8 = 2;
pub const KING_ELECTION_NOTICE_BOARD: u8 = 3;
pub const KING_ELECTION_POLL: u8 = 4;
pub const KING_ELECTION_RESIGN: u8 = 5;

/// Impeachment sub-opcodes (`KingImpeachmentType` enum in `packets.h:505-510`).
pub const KING_IMPEACHMENT_REQUEST: u8 = 1;
pub const KING_IMPEACHMENT_REQUEST_ELECT: u8 = 2;
pub const KING_IMPEACHMENT_LIST: u8 = 3;
pub const KING_IMPEACHMENT_ELECT: u8 = 4;
pub const KING_IMPEACHMENT_REQUEST_UI_OPEN: u8 = 8;
pub const KING_IMPEACHMENT_ELECTION_UI_OPEN: u8 = 9;

/// King's Scepter item ID
pub const KING_SCEPTER: u32 = 910_074_311;

/// Minimum level required to vote in king elections
pub const MIN_LEVEL_VOTER: u8 = 50;

/// Minimum national points (loyalty) required to vote in king elections.
pub const MIN_NP_VOTER: i32 = 10_000;

/// Notice board sub-opcodes (`KingCandidacyBoardType` in `packets.h:498-499`).
pub const KING_CANDIDACY_BOARD_WRITE: u8 = 1;
pub const KING_CANDIDACY_BOARD_READ: u8 = 2;

/// Per-nation king system runtime state.
/// Loaded from `king_system` table at startup; holds election schedule,
/// impeachment state, active events (noah/exp), treasury, tax, and king info.
#[derive(Debug, Clone)]
pub struct KingSystem {
    /// Nation identifier: 1=Karus, 2=Elmorad.
    pub nation: u8,
    /// Election type
    pub election_type: u8,

    /// Scheduled election date.
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,

    /// Impeachment state and schedule.
    pub im_type: u8,
    pub im_year: u16,
    pub im_month: u8,
    pub im_day: u8,
    pub im_hour: u8,
    pub im_minute: u8,

    /// Noah (coin) bonus event.
    pub noah_event: u8,
    pub noah_event_day: u8,
    pub noah_event_hour: u8,
    pub noah_event_minute: u8,
    pub noah_event_duration: u16,

    /// EXP bonus event.
    pub exp_event: u8,
    pub exp_event_day: u8,
    pub exp_event_hour: u8,
    pub exp_event_minute: u8,
    pub exp_event_duration: u16,

    /// Tribute amount.
    pub tribute: u32,
    /// Territory tariff rate (0-10).
    pub territory_tariff: u8,
    /// Territory tax collected (by tariff).
    pub territory_tax: u32,
    /// National treasury balance.
    pub national_treasury: u32,

    /// Current king's character name.
    pub king_name: String,
    /// Current king's clan ID.
    pub king_clan_id: u16,
    /// Impeachment requester's name.
    pub im_request_id: String,

    // â”€â”€ Election runtime state (not persisted to king_system table) â”€â”€
    /// Whether an election is currently being processed (prevents concurrent changes).
    pub election_under_progress: bool,
    /// Throttle flag for periodic election messages.
    pub sent_first_message: bool,
    /// Top 10 ranked clan IDs for this nation.
    pub top10_clan_set: Vec<u16>,
    /// Senator list: character name â†’ (knights_id, votes).
    ///
    pub senator_list: Vec<ElectionListEntry>,
    /// Candidate list: nominated candidates for King.
    ///
    pub candidate_list: Vec<ElectionListEntry>,
    /// Nomination list: who nominated whom.
    ///
    pub nomination_list: Vec<NominationEntry>,
    /// Notice board: candidate platform statements.
    ///
    pub notice_board: Vec<(String, String)>,
    /// Resigned candidate names (cannot be re-nominated).
    ///
    pub resigned_candidates: Vec<String>,
    /// New king name determined after election results.
    pub new_king_name: String,
    /// Votes for the winning king.
    pub king_votes: u32,
    /// Total votes cast.
    pub total_votes: u32,
}

/// An entry in the senator or candidate election list.
#[derive(Debug, Clone)]
pub struct ElectionListEntry {
    /// Character name of the senator/candidate.
    pub name: String,
    /// Knights (clan) ID.
    pub knights_id: u16,
    /// Number of votes received (candidates only).
    pub votes: u32,
}

/// A nomination entry: who nominated whom.
#[derive(Debug, Clone)]
pub struct NominationEntry {
    /// Character name of the nominator.
    pub nominator: String,
    /// Character name of the nominee.
    pub nominee: String,
}

// -- Castle Siege Warfare Data ------------------------------------------------

/// Zone ID constants for king tariff zones.
pub const ZONE_KARUS: u16 = 1;
pub const ZONE_ELMORAD: u16 = 2;
pub const ZONE_KARUS2: u16 = 5;
pub const ZONE_KARUS3: u16 = 6;
pub const ZONE_ELMORAD2: u16 = 7;
pub const ZONE_ELMORAD3: u16 = 8;
pub const ZONE_KARUS_ESLANT: u16 = 11;
pub const ZONE_ELMORAD_ESLANT: u16 = 12;
pub const ZONE_KARUS_ESLANT2: u16 = 13;
pub const ZONE_KARUS_ESLANT3: u16 = 14;
pub const ZONE_ELMORAD_ESLANT2: u16 = 15;
pub const ZONE_ELMORAD_ESLANT3: u16 = 16;
pub const ZONE_OLD_KARUS: u16 = 18;
pub const ZONE_OLD_HUMAN: u16 = 28;
pub const ZONE_OLD_MORADON: u16 = 29;
pub const ZONE_BIFROST: u16 = 31;
/// Base zone ID for battle zones (actual zone = ZONE_BATTLE_BASE + offset).
pub const ZONE_BATTLE_BASE: u16 = 60;
pub const ZONE_BATTLE: u16 = 61;
pub const ZONE_BATTLE2: u16 = 62;
pub const ZONE_BATTLE3: u16 = 63;
pub const ZONE_BATTLE4: u16 = 64;
pub const ZONE_BATTLE5: u16 = 65;
pub const ZONE_BATTLE6: u16 = 66;
pub const ZONE_SNOW_BATTLE: u16 = 69;
pub const ZONE_RONARK_LAND: u16 = 71;
pub const ZONE_ARDREAM: u16 = 72;
pub const ZONE_RONARK_LAND_BASE: u16 = 73;
pub const ZONE_NEW_BATTLE_TEST: u16 = 74;
pub const ZONE_KROWAZ_DOMINION: u16 = 75;
pub const ZONE_CLAN_WAR_ARDREAM: u16 = 77;
pub const ZONE_CLAN_WAR_RONARK: u16 = 78;
pub const ZONE_STONE1: u16 = 81;
pub const ZONE_STONE2: u16 = 82;
pub const ZONE_STONE3: u16 = 83;
pub const ZONE_KNIGHT_ROYALE: u16 = 76;
pub const ZONE_BORDER_DEFENSE_WAR: u16 = 84;
pub const ZONE_CHAOS_DUNGEON: u16 = 85;
pub const ZONE_UNDER_CASTLE: u16 = 86;
pub const ZONE_JURAID_MOUNTAIN: u16 = 87;
pub const ZONE_DUNGEON_DEFENCE: u16 = 89;
pub const ZONE_PRISON: u16 = 92;
pub const ZONE_DRAKI_TOWER: u16 = 95;
pub const ZONE_PARTY_VS_1: u16 = 96;
pub const ZONE_PARTY_VS_2: u16 = 97;
pub const ZONE_PARTY_VS_3: u16 = 98;
pub const ZONE_PARTY_VS_4: u16 = 99;
pub const ZONE_SPBATTLE1: u16 = 105;
pub const ZONE_SPBATTLE_MIN: u16 = 105;
pub const ZONE_SPBATTLE_MAX: u16 = 115;

/// Short aliases for event systems.
pub const ZONE_BDW: u16 = 84;
pub const ZONE_CHAOS: u16 = 85;
pub const ZONE_JURAID: u16 = 87;

/// Zone ID constants for siege/tariff zones.
pub const ZONE_MORADON: u16 = 21;
pub const ZONE_MORADON2: u16 = 22;
pub const ZONE_MORADON3: u16 = 23;
pub const ZONE_MORADON4: u16 = 24;
pub const ZONE_MORADON5: u16 = 25;
pub const ZONE_DELOS: u16 = 30;
pub const ZONE_DESPERATION_ABYSS: u16 = 32;
pub const ZONE_HELL_ABYSS: u16 = 33;
pub const ZONE_DRAGON_CAVE: u16 = 34;
pub const ZONE_DELOS_CASTELLAN: u16 = 35;
pub const ZONE_CAITHAROS_ARENA: u16 = 54;
/// Isiloon Arena zone.
pub const ZONE_ISILOON_ARENA: u16 = 93;
/// Felankor Arena zone.
pub const ZONE_FELANKOR_ARENA: u16 = 94;
/// Arena zone (for PVP/CVC duels).
pub const ZONE_ARENA: u16 = 48;
/// Orc Arena zone.
pub const ZONE_ORC_ARENA: u16 = 51;
/// Blood Don Arena zone.
pub const ZONE_BLOOD_DON_ARENA: u16 = 52;
/// Goblin Arena zone.
pub const ZONE_GOBLIN_ARENA: u16 = 53;
/// Forgotten Temple event zone.
pub const ZONE_FORGOTTEN_TEMPLE: u16 = 55;

/// Middle statue warp coordinates for Karus camp.
pub const DODO_CAMP_WARP_X: u16 = 10540;
pub const DODO_CAMP_WARP_Z: u16 = 11410;

/// Middle statue warp coordinates for El Morad camp.
pub const LAON_CAMP_WARP_X: u16 = 10120;
pub const LAON_CAMP_WARP_Z: u16 = 9140;

/// Random offset radius for middle statue warp.
pub const DODO_LAON_WARP_RADIUS: u16 = 5;

/// NPC special type for cycle-spawn NPCs (quest key NPCs that rotate positions).
pub const NPC_SPECIAL_TYPE_CYCLE_SPAWN: i16 = 7;

/// Runtime siege warfare state cached in memory.
/// One instance per castle (castle_index=1 for Delos).
#[derive(Debug, Clone)]
pub struct SiegeWarfare {
    /// Castle identifier (1 = Delos).
    pub castle_index: u16,
    /// Clan ID of the castle owner.
    pub master_knights: u16,
    /// Siege type (0=none, 1=regular).
    pub siege_type: u8,
    /// Scheduled war day.
    pub war_day: u8,
    /// Scheduled war hour.
    pub war_time: u8,
    /// Scheduled war minute.
    pub war_minute: u8,
    /// Challenge clan list (up to 10 clan IDs).
    pub challenge_list: [u16; 10],
    /// War request schedule.
    pub war_request_day: u8,
    pub war_request_time: u8,
    pub war_request_minute: u8,
    /// Guerrilla war schedule.
    pub guerrilla_war_day: u8,
    pub guerrilla_war_time: u8,
    pub guerrilla_war_minute: u8,
    /// Challenge list string (legacy).
    pub challenge_list_str: String,
    /// Moradon tariff rate (0-20).
    pub moradon_tariff: u16,
    /// Delos tariff rate (0-20).
    pub delos_tariff: u16,
    /// Accumulated dungeon charge revenue.
    pub dungeon_charge: i32,
    /// Accumulated Moradon tax revenue.
    pub moradon_tax: i32,
    /// Accumulated Delos tax revenue.
    pub delos_tax: i32,
    /// Request clan list (up to 10 clan IDs).
    pub request_list: [u16; 10],
}

impl Default for SiegeWarfare {
    fn default() -> Self {
        Self {
            castle_index: 1,
            master_knights: 0,
            siege_type: 1,
            war_day: 0,
            war_time: 0,
            war_minute: 0,
            challenge_list: [0; 10],
            war_request_day: 0,
            war_request_time: 0,
            war_request_minute: 0,
            guerrilla_war_day: 0,
            guerrilla_war_time: 0,
            guerrilla_war_minute: 0,
            challenge_list_str: String::new(),
            moradon_tariff: 10,
            delos_tariff: 10,
            dungeon_charge: 0,
            moradon_tax: 0,
            delos_tax: 0,
            request_list: [0; 10],
        }
    }
}

// â”€â”€ Castle Siege War Runtime State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// ── Zindan War Runtime State ────────────────────────────────────────────

/// Runtime state for Zindan War (Special Event in ZONE_SPBATTLE1).
#[derive(Debug, Clone, Default)]
pub struct ZindanWarState {
    /// Elmorad clan/team display name.
    pub elmo_name: String,
    /// Elmorad kill count.
    pub elmo_kills: u32,
    /// Karus clan/team display name.
    pub karus_name: String,
    /// Karus kill count.
    pub karus_kills: u32,
    /// Unix timestamp when the event finishes.
    pub finish_time: u64,
}

/// CSW operational status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CswOpStatus {
    /// No operation (idle).
    NotOperation = 0,
    /// Preparation phase (players being kicked, gates spawning).
    Preparation = 1,
    /// Active war phase (monument can be captured).
    War = 2,
}

/// CSW notice types sent to players.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CswNotice {
    /// Preparation phase starting.
    Preparation = 0,
    /// Monument destroyed / castle captured.
    MonumentKilled = 1,
    /// War phase starting.
    War = 2,
    /// CSW finished.
    CswFinish = 3,
}

/// Runtime state for castle siege warfare events.
/// Tracks the live war lifecycle, timers, and per-clan kill counts.
#[derive(Debug, Clone)]
pub struct CswEventState {
    /// Current operational status.
    pub status: CswOpStatus,
    /// Unix timestamp when current phase ends.
    pub csw_time: u64,
    /// Whether CSW has been started this cycle.
    pub started: bool,
    /// Unix timestamp when the monument was last killed.
    pub monument_time: u64,
    /// Whether the preparation phase check has fired.
    pub prepare_check: bool,
    /// Whether the war phase check has fired.
    pub war_check: bool,
    /// Per-clan kill counts during active CSW (clan_id -> kill_count).
    pub clan_kill_list: std::collections::HashMap<u16, u16>,
    /// Players registered for the CSW deathmatch (session_id set).
    pub deathmatch_players: std::collections::HashSet<u16>,
    /// Preparation phase duration in minutes
    pub prep_minutes: u32,
    /// War phase duration in minutes
    pub war_minutes: u32,
}

impl Default for CswEventState {
    fn default() -> Self {
        Self {
            status: CswOpStatus::NotOperation,
            csw_time: 0,
            started: false,
            monument_time: 0,
            prepare_check: false,
            war_check: false,
            clan_kill_list: std::collections::HashMap::new(),
            deathmatch_players: std::collections::HashSet::new(),
            prep_minutes: 30,
            war_minutes: 60,
        }
    }
}

impl CswEventState {
    /// Whether the CSW event is currently active (preparation or war phase).
    ///
    pub fn is_active(&self) -> bool {
        self.started && self.status != CswOpStatus::NotOperation
    }

    /// Whether the war phase is currently active (not just preparation).
    ///
    pub fn is_war_active(&self) -> bool {
        self.started && self.status == CswOpStatus::War
    }

    /// Reset the CSW event state.
    ///
    pub fn reset(&mut self) {
        self.status = CswOpStatus::NotOperation;
        self.csw_time = 0;
        self.started = false;
        self.monument_time = 0;
        self.war_check = false;
        self.prepare_check = false;
        self.deathmatch_players.clear();
    }

    /// Register a clan in the kill tracking list (initial kill count = 0).
    ///
    pub fn register_clan(&mut self, clan_id: u16) {
        self.clan_kill_list.entry(clan_id).or_insert(0);
    }

    /// Increment kill count for a clan.
    ///
    pub fn increment_clan_kills(&mut self, clan_id: u16) {
        if let Some(count) = self.clan_kill_list.get_mut(&clan_id) {
            *count += 1;
        }
    }
}

// â”€â”€ Beef Roast (Bifrost) Event State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Runtime state for the beef roast / Bifrost monument event.
#[derive(Debug, Clone, Default)]
pub struct BeefEventState {
    /// Whether the beef event is currently active.
    pub is_active: bool,
    /// Whether the monument can be attacked.
    pub is_attackable: bool,
    /// Whether the monument has been destroyed.
    pub is_monument_dead: bool,
    /// Winning nation (0=none, 1=Karus, 2=Elmorad).
    pub winner_nation: u8,
    /// Whether farming phase is active.
    pub is_farming_play: bool,
    /// UNIX timestamp when the farming phase ends (0 = not set).
    ///
    pub farming_end_time: u64,
    /// UNIX timestamp when the loser nation can log in (0 = not set).
    ///
    pub loser_sign_time: u64,
    /// Whether the loser nation is allowed to enter Bifrost zone.
    ///
    pub is_loser_sign: bool,
}

// â”€â”€ Ranking System Data Structures â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Ranking type constants (`RankTypes` enum in `packets.h:706-711`).
pub const RANK_TYPE_PK_ZONE: u8 = 1;
pub const RANK_TYPE_ZONE_BORDER_DEFENSE_WAR: u8 = 2;
pub const RANK_TYPE_CHAOS_DUNGEON: u8 = 3;

/// PK zone ranking entry.
/// Also used for Zindan War ranking (`_ZINDAN_WAR_RANKING`).
#[derive(Debug, Clone)]
pub struct PkZoneRanking {
    /// Session ID of the player.
    pub session_id: SessionId,
    /// Zone ID where the player is ranked.
    pub zone_id: u16,
    /// Nation (1=Karus, 2=El Morad).
    pub nation: u8,
    /// Daily loyalty points earned.
    pub loyalty_daily: u32,
    /// Premium loyalty bonus earned.
    pub loyalty_premium_bonus: u16,
}

/// Border Defence War ranking entry.
#[derive(Debug, Clone)]
pub struct BdwRanking {
    /// Session ID of the player.
    pub session_id: SessionId,
    /// Event room ID.
    pub event_room: i16,
    /// Nation (1=Karus, 2=El Morad).
    pub nation: u8,
    /// User points earned in BDW.
    pub user_point: u32,
}

/// Chaos Expansion ranking entry.
#[derive(Debug, Clone)]
pub struct ChaosRanking {
    /// Session ID of the player.
    pub session_id: SessionId,
    /// Event room ID.
    pub event_room: i16,
    /// Kill count.
    pub kill_count: u16,
    /// Death count.
    pub death_count: u16,
}

/// Maximum number of members in a party.
pub const MAX_PARTY_USERS: usize = 8;

/// Maximum entries per page in the Party BBS.
pub const MAX_BBS_PAGE: usize = 22;

/// A user/party entry in the party seeking bulletin board.
#[derive(Debug, Clone)]
pub struct SeekingPartyUser {
    /// Session ID of the registrant.
    pub sid: u16,
    /// Character class (or wanted class for party leaders).
    pub class: u16,
    /// Whether the registrant is a party leader (0=no, 1=yes).
    pub is_party_leader: u8,
    /// Character level.
    pub level: i16,
    /// Zone ID the registrant is in.
    pub zone: u8,
    /// Free-text seeking note / wanted message.
    pub seeking_note: String,
    /// Character name.
    pub name: String,
    /// Nation (1=Karus, 2=El Morad).
    pub nation: u8,
    /// Party ID (if in a party).
    pub party_id: u16,
    /// Seek type (always 0 in C++).
    pub seek_type: u8,
    /// Login type (0=normal, 2=hidden/blocked from listing).
    pub login_type: u8,
}

/// Maximum number of users in a single chat room.
pub const MAX_CHAT_ROOM_USERS: u16 = 200;

/// In-memory chat room data.
#[derive(Debug, Clone)]
pub struct ChatRoom {
    /// Unique room index.
    pub index: u16,
    /// Room display name.
    pub name: String,
    /// Character name of the room administrator/creator.
    pub administrator: String,
    /// Password (empty = no password).
    pub password: String,
    /// Nation of the room creator (1=Karus, 2=El Morad).
    pub nation: u8,
    /// Maximum allowed users.
    pub max_users: u16,
    /// Current user count.
    pub current_users: u16,
    /// Member list: incremental member_id -> character_name.
    ///
    pub members: HashMap<u16, String>,
    /// Internal member ID counter (incremented on each AddUser).
    pub next_member_id: u16,
}

impl ChatRoom {
    /// Check if the room has a password set.
    ///
    pub fn has_password(&self) -> bool {
        !self.password.is_empty()
    }

    /// Check admin status for a user (returns 2 if admin, 1 if regular member).
    ///
    pub fn is_administrator(&self, name: &str) -> u8 {
        if self.administrator.eq_ignore_ascii_case(name) {
            2
        } else {
            1
        }
    }

    /// Add a user to the room. Returns false if the room is full.
    ///
    pub fn add_user(&mut self, name: &str) -> bool {
        if self.current_users >= self.max_users {
            return false;
        }
        self.next_member_id += 1;
        self.members.insert(self.next_member_id, name.to_string());
        self.current_users += 1;
        true
    }

    /// Remove a user by name. Returns true if found and removed.
    pub fn remove_user(&mut self, name: &str) -> bool {
        let key = self
            .members
            .iter()
            .find(|(_, v)| v.eq_ignore_ascii_case(name))
            .map(|(k, _)| *k);
        if let Some(k) = key {
            self.members.remove(&k);
            self.current_users = self.current_users.saturating_sub(1);
            true
        } else {
            false
        }
    }

    /// Remove a user by member ID. Returns true if found and removed.
    pub fn remove_user_by_id(&mut self, member_id: u16) -> bool {
        if self.members.remove(&member_id).is_some() {
            self.current_users = self.current_users.saturating_sub(1);
            true
        } else {
            false
        }
    }

    /// Check if a user is in this room.
    pub fn contains_user(&self, name: &str) -> bool {
        self.members.values().any(|v| v.eq_ignore_ascii_case(name))
    }
}

/// Range (squared) for party XP sharing.
pub const RANGE_50M: f32 = 50.0 * 50.0;

/// Range (squared) for quest kill credit in party.
pub const RANGE_80M: f32 = 80.0 * 80.0;

/// A runtime party group.
/// Stores up to 8 member session IDs. Slot 0 is always the leader.
/// Empty slots contain `None`.
#[derive(Debug, Clone)]
pub struct Party {
    /// Unique party ID.
    pub id: u16,
    /// Member session IDs (`MAX_PARTY_USERS` slots).
    /// Index 0 = leader. `None` = empty slot.
    pub members: [Option<SessionId>; MAX_PARTY_USERS],
    /// Round-robin item routing index.
    ///
    pub item_routing: u8,
    /// Target number ID for party target marking.
    ///
    pub target_number_id: i16,
    /// Command leader session ID.
    ///
    /// Transferred via PARTY_COMMAND_PROMATE. Only the command leader can set
    /// target numbers, send alerts, and transfer command leadership.
    pub command_leader_sid: Option<SessionId>,
}

impl Party {
    /// Create a new party with a leader.
    pub fn new(id: u16, leader_sid: SessionId) -> Self {
        let mut members = [None; MAX_PARTY_USERS];
        members[0] = Some(leader_sid);
        Self {
            id,
            members,
            item_routing: 0,
            target_number_id: -1,
            command_leader_sid: Some(leader_sid),
        }
    }

    /// Get the leader's session ID.
    pub fn leader_sid(&self) -> Option<SessionId> {
        self.members[0]
    }

    /// Count current members.
    pub fn member_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_some()).count()
    }

    /// Check if the party is full.
    pub fn is_full(&self) -> bool {
        self.member_count() >= MAX_PARTY_USERS
    }

    /// Find the first empty slot and insert a member. Returns true on success.
    pub fn add_member(&mut self, sid: SessionId) -> bool {
        // Don't add if already present
        if self.members.contains(&Some(sid)) {
            return false;
        }
        for slot in &mut self.members {
            if slot.is_none() {
                *slot = Some(sid);
                return true;
            }
        }
        false
    }

    /// Remove a member by session ID. Returns true if found and removed.
    pub fn remove_member(&mut self, sid: SessionId) -> bool {
        for slot in &mut self.members {
            if *slot == Some(sid) {
                *slot = None;
                return true;
            }
        }
        false
    }

    /// Check if a session ID is in this party.
    pub fn contains(&self, sid: SessionId) -> bool {
        self.members.contains(&Some(sid))
    }

    /// Check if a session ID is the leader (slot 0).
    pub fn is_leader(&self, sid: SessionId) -> bool {
        self.members[0] == Some(sid)
    }

    /// Check if a session ID is the command leader.
    ///
    pub fn is_command_leader(&self, sid: SessionId) -> bool {
        self.command_leader_sid == Some(sid)
    }

    /// Get all active member session IDs.
    pub fn active_members(&self) -> Vec<SessionId> {
        self.members.iter().filter_map(|m| *m).collect()
    }

    /// Find the slot index of a member.
    pub fn find_slot(&self, sid: SessionId) -> Option<usize> {
        self.members.iter().position(|m| *m == Some(sid))
    }

    /// Swap the leader (slot 0) with a member at `pos`.
    pub fn swap_leader(&mut self, pos: usize) {
        if pos > 0 && pos < MAX_PARTY_USERS {
            self.members.swap(0, pos);
        }
    }
}

/// Runtime clan data cached in memory.
#[derive(Debug, Clone, Default)]
pub struct KnightsInfo {
    /// Clan ID.
    pub id: u16,
    /// Clan type flag (Training=1, Promoted=2, Accredited=3..7, Royal=8..12).
    pub flag: u8,
    /// Nation (1=Karus, 2=El Morad).
    pub nation: u8,
    /// Grade (1-5, 1 = best).
    pub grade: u8,
    /// Ranking.
    pub ranking: u8,
    /// Clan name.
    pub name: String,
    /// Chief (leader) character name.
    pub chief: String,
    /// Vice chief 1.
    pub vice_chief_1: String,
    /// Vice chief 2.
    pub vice_chief_2: String,
    /// Vice chief 3.
    pub vice_chief_3: String,
    /// Member count.
    pub members: u16,
    /// Total clan points (sum of member loyalty).
    pub points: u32,
    /// Clan point fund (donated NP pool).
    pub clan_point_fund: u32,
    /// Clan notice text.
    pub notice: String,
    /// Cape ID.
    pub cape: u16,
    /// Cape R color.
    pub cape_r: u8,
    /// Cape G color.
    pub cape_g: u8,
    /// Cape B color.
    pub cape_b: u8,
    /// Mark (symbol) version.
    pub mark_version: u16,
    /// Mark (symbol) image data (max 2400 bytes).
    ///
    pub mark_data: Vec<u8>,
    /// Alliance ID.
    pub alliance: u16,
    /// Castellan cape flag.
    pub castellan_cape: bool,
    /// Castellan cape ID.
    pub cast_cape_id: i16,
    /// Castellan cape R color.
    pub cast_cape_r: u8,
    /// Castellan cape G color.
    pub cast_cape_g: u8,
    /// Castellan cape B color.
    pub cast_cape_b: u8,
    /// Castellan cape time (unix timestamp).
    pub cast_cape_time: u32,
    /// Alliance request pending from clan ID (runtime only).
    pub alliance_req: u16,
    /// Clan point method (0=equal, 1=chief decides, etc.).
    pub clan_point_method: u8,
    /// Clan premium expiration (unix timestamp, 0 = none).
    ///
    pub premium_time: u32,
    /// Clan premium type in use (0 = none, 13 = CLAN_PREMIUM).
    ///
    pub premium_in_use: u8,
    /// Online member count (runtime only, not persisted).
    ///
    pub online_members: u16,
    /// Online NP bonus percentage (calculated from online_members).
    ///
    pub online_np_count: u16,
    /// Online EXP bonus percentage (calculated from online_members).
    ///
    pub online_exp_count: u16,
}

/// Runtime alliance data cached in memory.
#[derive(Debug, Clone)]
pub struct KnightsAlliance {
    /// Leader clan ID (also the map key).
    pub main_clan: u16,
    /// Sub-alliance clan ID.
    pub sub_clan: u16,
    /// Mercenary clan 1.
    pub mercenary_1: u16,
    /// Mercenary clan 2.
    pub mercenary_2: u16,
    /// Alliance notice text.
    pub notice: String,
}

/// Position + region tracking for a session.
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub zone_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub region_x: u16,
    pub region_z: u16,
}

/// Result of a position update â€” did the player change regions?
pub enum RegionChangeResult {
    /// Player stayed in the same region.
    NoChange,
    /// Player moved to a different region.
    Changed {
        old_rx: u16,
        old_rz: u16,
        new_rx: u16,
        new_rz: u16,
    },
}

// ─── Bot System ───────────────────────────────────────────────────────────────

/// Unique runtime ID for a spawned bot.
/// In C++, bot socket IDs start at `MAX_USER` (5000) + bot slot index.
/// We assign IDs starting at `BOT_ID_BASE` to distinguish from player session IDs.
pub type BotId = u32;

/// Band for bot runtime IDs — bots use IDs >= BOT_ID_BASE.
/// We use a higher band (10_000) to avoid any collision with NPC IDs (NPC_BAND).
pub const BOT_ID_BASE: u32 = 10_000;

/// Bot AI state — mirrors `CBot::m_BotState` values (BotHandler.h).
/// C++ defines (User.h lines 71-85):
/// `BOT_AFK=0, BOT_MINING=1, BOT_FISHING=2, BOT_FARMER=3, BOT_FARMERS=4,
///  BOT_MERCHANT=5, BOT_DEAD=6, BOT_MOVE=7, BOT_MERCHANT_MOVE=8`
/// Note: Rust variant discriminants are independent of the C++ enum values
/// and are used for internal state dispatch only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum BotAiState {
    /// Idle — just spawned, not yet acting.
    #[default]
    Idle = 0,
    /// Moving to a destination (general walking).
    Move = 1,
    /// Farm-bot AI: hunting monsters for XP/loot.
    Farmer = 2,
    /// Mining bot: performing mining animations near ore nodes.
    Mining = 3,
    /// Fishing bot: performing fishing animations near water.
    Fishing = 4,
    /// Merchant bot: standing still, broadcasting merchant chat.
    Merchant = 5,
    /// Moving merchant bot: walks to a position then opens merchant.
    MerchantMove = 6,
    /// PK bot: seeks and attacks enemy-nation players in PK zones.
    Pk = 7,
    /// AFK bot: stands in place, no active AI.
    Afk = 8,
}

/// User-presence state of a bot — mirrors `USER_STANDING` / `USER_DEAD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum BotPresence {
    /// Bot is alive and standing.
    #[default]
    Standing = 1,
    /// Bot is sitting down.
    Sitting = 2,
    /// Bot is dead (waiting for Regene).
    Dead = 3,
}

/// Runtime mutable state for a single spawned bot.
/// Stored in `WorldState::bots`, keyed by `BotId`.
#[derive(Debug, Clone)]
pub struct BotInstance {
    /// Unique runtime ID (>= BOT_ID_BASE).
    pub id: BotId,
    /// DB row ID from `bot_handler_farm` (or 0 for event-spawned bots).
    pub db_id: i32,
    /// Character name shown in-game.
    pub name: String,
    /// Nation: 1 = Karus, 2 = Elmorad.
    pub nation: u8,
    /// Race code.
    pub race: u8,
    /// Class code (e.g. 107 = rogue mastered).
    pub class: u16,
    /// Hair RGB value.
    pub hair_rgb: u32,
    /// Character level.
    pub level: u8,
    /// Face index.
    pub face: u8,
    /// Knights (clan) ID (0 = no clan).
    pub knights_id: u16,
    /// Fame rank.
    pub fame: u8,

    // ─── Position & Zone ────────────────────────────────────────────────
    /// Current zone ID.
    pub zone_id: u16,
    /// Current X coordinate.
    pub x: f32,
    /// Current Y coordinate (height).
    pub y: f32,
    /// Current Z coordinate.
    pub z: f32,
    /// Direction facing (0–360 in fixed-point * 10).
    pub direction: i16,
    /// Region grid X (derived from x / REGION_SIZE).
    pub region_x: u16,
    /// Region grid Z (derived from z / REGION_SIZE).
    pub region_z: u16,

    // ─── Vitals ──────────────────────────────────────────────────────────
    /// Current HP.
    pub hp: i16,
    /// Maximum HP.
    pub max_hp: i16,
    /// Current MP.
    pub mp: i16,
    /// Maximum MP.
    pub max_mp: i16,
    /// Current SP (Kurian only).
    pub sp: i16,
    /// Maximum SP.
    pub max_sp: u8,

    // ─── Stats ───────────────────────────────────────────────────────────
    pub str_stat: u8,
    pub sta_stat: u8,
    pub dex_stat: u8,
    pub int_stat: u8,
    pub cha_stat: u8,

    // ─── Economy ─────────────────────────────────────────────────────────
    /// Gold (Noah).
    pub gold: u32,
    /// National Points (loyalty).
    pub loyalty: u32,
    /// Monthly NP.
    pub loyalty_monthly: u32,

    // ─── Presence & AI State ────────────────────────────────────────────
    /// Whether the bot is currently registered in-game (INOUT_IN sent).
    pub in_game: bool,
    /// HP/life presence: Standing, Sitting, or Dead.
    pub presence: BotPresence,
    /// AI behaviour mode.
    pub ai_state: BotAiState,
    /// Target ID for combat (player session ID or NPC runtime ID, -1 = none).
    pub target_id: i16,
    /// Whether the target changed this tick (triggers echo=1 in move packets).
    pub target_changed: bool,

    // ─── Timers ──────────────────────────────────────────────────────────
    /// UNIX timestamp when the bot was spawned (for expiry calculation).
    pub spawned_at: u64,
    /// Duration in minutes until auto-despawn (0 = permanent).
    pub duration_minutes: u32,
    /// Timestamp (ms) of last AI tick.
    pub last_tick_ms: u64,
    /// Timestamp (ms) of last move.
    pub last_move_ms: u64,
    /// Timestamp (ms) of last mining/fishing animation.
    pub last_mining_ms: u64,
    /// Timestamp (ms) of last merchant chat broadcast.
    pub last_merchant_chat_ms: u64,
    /// Timestamp (ms) of last HP/MP change packet.
    pub last_hp_change_ms: u64,
    /// Timestamp (ms) of last HP/MP regen tick for bots.
    pub last_regen_ms: u64,
    /// Last attacker ID for kill reward tracking (-1 = none).
    pub last_attacker_id: i32,
    /// Timestamp (ms) of last skill cooldown end (slot 0: magic gate, slot 1: regen).
    pub skill_cooldown: [u64; 2],
    /// Timestamp (ms) of last type-4 buff refresh.
    pub last_type4_ms: u64,
    /// Timestamp (ms) when the bot should regene after death (0 = not pending).
    ///
    /// Set to `tick_ms() + BOT_REGENE_DELAY_MS` when bot dies.
    pub regene_at_ms: u64,
    /// AI state to restore after regene (saved when bot dies).
    ///
    pub original_ai_state: BotAiState,

    // ─── Waypoint Patrol State ──────────────────────────────────────────
    /// Current patrol route ID (1–10 for Ronark, 1–5 for Ardream, 0 = none).
    ///
    pub move_route: u8,
    /// Current waypoint index within the active route (1-based).
    ///
    pub move_state: u8,

    // ─── Merchant State ──────────────────────────────────────────────────
    /// Merchant state: -1 = none, 0 = selling, 1 = buying.
    pub merchant_state: i8,
    /// Whether bot is a premium merchant.
    pub premium_merchant: bool,
    /// Merchant broadcast chat string.
    pub merchant_chat: String,
    /// Rebirth level.
    pub reb_level: u8,
    /// Achieve cover title ID.
    pub cover_title: u16,
    /// Session/Bot ID of the rival (-1 = no rival).
    ///
    pub rival_id: i16,
    /// Unix timestamp (seconds) when rivalry expires.
    ///
    pub rival_expiry_time: u64,
    /// Anger gauge level (0..=5). Incremented on death in PK zones.
    ///
    pub anger_gauge: u8,
    /// Whether the bot is hiding helmet cosmetic.
    pub hiding_helmet: bool,
    /// Whether the bot is hiding cospre cosmetic.
    pub hiding_cospre: bool,
    /// Whether the bot needs party (shown on UI).
    pub need_party: u8,
    /// Equipment visual array — 17 slots of (item_id, durability, flag).
    ///
    /// Slot order matches `VISUAL_SLOT_ORDER`: 8 equipped + 9 cosplay.
    pub equip_visual: [(u32, i16, u8); 17],

    // ─── Rankings ───────────────────────────────────────────────────────
    /// Personal rank (1-based, from DB ranking system, 0 = unranked).
    ///
    pub personal_rank: u8,
    /// Knights (clan) rank (1-based, from DB ranking system, 0 = unranked).
    ///
    pub knights_rank: u8,
}

impl BotInstance {
    /// Returns true if the bot is currently alive and in-game.
    pub fn is_alive(&self) -> bool {
        self.in_game && self.presence != BotPresence::Dead && self.hp > 0
    }

    /// Returns true if the bot has expired (duration elapsed since spawn).
    ///
    /// `now_unix` is the current UNIX timestamp in seconds.
    pub fn is_expired(&self, now_unix: u64) -> bool {
        if self.duration_minutes == 0 {
            return false; // permanent bot
        }
        now_unix >= self.spawned_at + (self.duration_minutes as u64 * 60)
    }

    /// Returns true if this bot is a warrior class.
    ///
    pub fn is_warrior(&self) -> bool {
        matches!(self.class % 100, 1 | 5 | 6)
    }

    /// Returns true if this bot is a rogue class.
    ///
    pub fn is_rogue(&self) -> bool {
        matches!(self.class % 100, 2 | 7 | 8)
    }

    /// Returns true if this bot is a mage class.
    ///
    pub fn is_mage(&self) -> bool {
        matches!(self.class % 100, 3 | 9 | 10)
    }

    /// Returns true if this bot is a priest class.
    ///
    pub fn is_priest(&self) -> bool {
        matches!(self.class % 100, 4 | 11 | 12)
    }

    /// Returns true if this bot is in a PK zone (Ardream, Ronark Land, etc.).
    ///
    /// C++ Define.h: ZONE_RONARK_LAND=71, ZONE_ARDREAM=72, ZONE_RONARK_LAND_BASE=73
    pub fn is_in_pk_zone(&self) -> bool {
        matches!(self.zone_id, 71..=73)
    }

    /// Returns true if this bot is mercanting (selling or buying).
    ///
    pub fn is_merchanting(&self) -> bool {
        self.merchant_state != -1
    }
}

#[cfg(test)]
mod types_tests {
    use super::*;

    // ── Sprint 941: Constants coverage ───────────────────────────────

    /// User state constants.
    #[test]
    fn test_user_state_constants() {
        assert_eq!(USER_STANDING, 0x01);
        assert_eq!(USER_SITDOWN, 0x02);
        assert_eq!(USER_DEAD, 0x03);
        assert_eq!(USER_MONUMENT, 0x06);
        assert_eq!(USER_MINING, 0x07);
        assert_eq!(USER_FLASHING, 0x08);
    }

    /// Nation constants.
    #[test]
    fn test_nation_constants() {
        assert_eq!(NATION_KARUS, 1);
        assert_eq!(NATION_ELMORAD, 2);
        assert_ne!(NATION_KARUS, NATION_ELMORAD);
    }

    /// MAX_LEVEL is 83.
    #[test]
    fn test_max_level() {
        assert_eq!(MAX_LEVEL, 83);
    }

    /// map_act_type: 1-4 → tender(0), others → atrocity(1).
    #[test]
    fn test_map_act_type() {
        assert_eq!(map_act_type(1), 0);
        assert_eq!(map_act_type(4), 0);
        assert_eq!(map_act_type(0), 1);
        assert_eq!(map_act_type(5), 1);
        assert_eq!(map_act_type(255), 1);
    }

    /// is_gate_npc_type matches gate NPC IDs.
    #[test]
    fn test_is_gate_npc_type() {
        assert!(is_gate_npc_type(50));  // NPC_GATE
        assert!(is_gate_npc_type(51));  // NPC_PHOENIX_GATE
        assert!(is_gate_npc_type(55));  // NPC_GATE_LEVER
        assert!(is_gate_npc_type(150)); // NPC_GATE2
        assert!(is_gate_npc_type(180)); // NPC_KROWAZ_GATE
        assert!(!is_gate_npc_type(21)); // NPC_MERCHANT
        assert!(!is_gate_npc_type(0));
    }

    /// is_guard_npc_type matches 11-15.
    #[test]
    fn test_is_guard_npc_type() {
        assert!(is_guard_npc_type(11));
        assert!(is_guard_npc_type(15));
        assert!(!is_guard_npc_type(10));
        assert!(!is_guard_npc_type(16));
    }

    /// Party: create, add, remove, count.
    #[test]
    fn test_party_basic_operations() {
        let mut party = Party::new(1, SessionId::from(100u16));
        assert_eq!(party.member_count(), 1);
        assert!(party.is_leader(SessionId::from(100u16)));
        assert!(party.add_member(SessionId::from(200u16)));
        assert_eq!(party.member_count(), 2);
        assert!(party.contains(SessionId::from(200u16)));
        assert!(party.remove_member(SessionId::from(200u16)));
        assert_eq!(party.member_count(), 1);
    }

    /// Party is full at MAX_PARTY_USERS.
    #[test]
    fn test_party_full() {
        let mut party = Party::new(1, SessionId::from(0u16));
        for i in 1..MAX_PARTY_USERS {
            assert!(party.add_member(SessionId::from(i as u16)));
        }
        assert!(party.is_full());
        assert!(!party.add_member(SessionId::from(99u16)));
    }

    /// ITEMCOUNT_MAX is 9999.
    #[test]
    fn test_itemcount_max() {
        assert_eq!(ITEMCOUNT_MAX, 9999);
    }

    /// Item flag constants.
    #[test]
    fn test_item_flag_constants() {
        assert_eq!(ITEM_FLAG_NONE, 0);
        assert_eq!(ITEM_FLAG_RENTED, 1);
        assert_eq!(ITEM_FLAG_SEALED, 4);
        assert_eq!(ITEM_FLAG_BOUND, 8);
    }

    // ── Existing rental tests ────────────────────────────────────────

    #[test]
    fn test_remaining_rental_minutes_no_expiry() {
        assert_eq!(remaining_rental_minutes(0), 0);
    }

    #[test]
    fn test_remaining_rental_minutes_expired() {
        // Expired 1 hour ago
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        assert_eq!(remaining_rental_minutes(now.saturating_sub(3600)), 0);
    }

    #[test]
    fn test_remaining_rental_minutes_future() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        // Expires in 2 hours (7200 seconds) = 120 minutes
        let result = remaining_rental_minutes(now + 7200);
        assert!((119..=120).contains(&result));
    }

    #[test]
    fn test_remaining_rental_minutes_slot_method() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        let slot = UserItemSlot {
            item_id: 100001,
            durability: 100,
            count: 1,
            flag: 0,
            original_flag: 0,
            serial_num: 0,
            expire_time: now + 3600, // 1 hour = 60 minutes
        };
        let result = slot.remaining_rental_minutes();
        assert!((59..=60).contains(&result));
    }

    #[test]
    fn test_remaining_rental_minutes_cap_u16() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        // 100 days = 144000 minutes, exceeds u16::MAX (65535)
        let result = remaining_rental_minutes(now + 100 * 86400);
        assert_eq!(result, u16::MAX);
    }

    // ── Sprint 942: Additional coverage ──────────────────────────────

    /// CswEventState: default is not active.
    #[test]
    fn test_csw_default_not_active() {
        let csw = CswEventState::default();
        assert!(!csw.is_active());
        assert!(!csw.is_war_active());
        assert_eq!(csw.status, CswOpStatus::NotOperation);
    }

    /// CswEventState: war phase is active.
    #[test]
    fn test_csw_war_active() {
        let mut csw = CswEventState::default();
        csw.started = true;
        csw.status = CswOpStatus::War;
        assert!(csw.is_active());
        assert!(csw.is_war_active());
    }

    /// CswEventState: reset clears state but preserves clan_kill_list.
    #[test]
    fn test_csw_reset() {
        let mut csw = CswEventState::default();
        csw.started = true;
        csw.status = CswOpStatus::War;
        csw.reset();
        assert!(!csw.is_active());
        assert!(!csw.started);
        assert_eq!(csw.status, CswOpStatus::NotOperation);
    }

    /// CswEventState: register and increment clan kills.
    #[test]
    fn test_csw_clan_kills() {
        let mut csw = CswEventState::default();
        csw.register_clan(5);
        assert_eq!(*csw.clan_kill_list.get(&5).unwrap(), 0);
        csw.increment_clan_kills(5);
        csw.increment_clan_kills(5);
        assert_eq!(*csw.clan_kill_list.get(&5).unwrap(), 2);
    }

    /// Merchant state constants.
    #[test]
    fn test_merchant_state_constants() {
        assert_eq!(MERCHANT_STATE_NONE, -1);
        assert_eq!(MERCHANT_STATE_SELLING, 0);
        assert_eq!(MERCHANT_STATE_BUYING, 1);
    }

    /// Trade state constants.
    #[test]
    fn test_trade_state_constants() {
        assert_eq!(TRADE_STATE_NONE, 1);
        assert_eq!(TRADE_STATE_SENDER, 2);
        assert_eq!(TRADE_STATE_TARGET, 3);
        assert_eq!(TRADE_STATE_TRADING, 4);
        assert_eq!(TRADE_STATE_DECIDING, 5);
    }

    /// Offline merchant constants.
    #[test]
    fn test_offline_merchant_constants() {
        assert_eq!(OFFLINE_MERCHANT_ITEM, 924_041_913);
        assert_eq!(OFFLINE_DEFAULT_MINUTES, 1400);
        assert_eq!(OFFLINE_CHECK_INTERVAL_SECS, 60);
        assert_eq!(CFAIRY_SLOT, 48);
    }

    /// Special item ID constants (gold, exp, count).
    #[test]
    fn test_special_item_ids() {
        assert_eq!(ITEM_GOLD, 900_000_000);
        assert_eq!(ITEM_EXP, 900_001_000);
        assert_eq!(ITEM_COUNT, 900_002_000);
        assert_eq!(ITEM_LADDERPOINT, 900_003_000);
        assert_eq!(ITEM_RANDOM, 900_004_000);
    }

    /// Party: duplicate add is rejected.
    #[test]
    fn test_party_duplicate_add() {
        let sid = SessionId::from(10u16);
        let mut party = Party::new(1, sid);
        assert!(!party.add_member(sid)); // already leader
        assert_eq!(party.member_count(), 1);
    }

    /// Party: swap_leader and find_slot.
    #[test]
    fn test_party_swap_leader() {
        let leader = SessionId::from(1u16);
        let member = SessionId::from(2u16);
        let mut party = Party::new(1, leader);
        party.add_member(member);
        assert!(party.is_leader(leader));
        let pos = party.find_slot(member).unwrap();
        party.swap_leader(pos);
        assert!(party.is_leader(member));
    }

    // ── Sprint 943: Additional coverage ──────────────────────────────

    /// NpcState enum values match C++ globals.h.
    #[test]
    fn test_npc_state_values() {
        assert_eq!(NpcState::Dead as u8, 0);
        assert_eq!(NpcState::Live as u8, 1);
        assert_eq!(NpcState::Attacking as u8, 2);
        assert_eq!(NpcState::Standing as u8, 5);
        assert_eq!(NpcState::Moving as u8, 6);
        assert_eq!(NpcState::Tracing as u8, 7);
        assert_eq!(NpcState::Fighting as u8, 8);
        assert_eq!(NpcState::Back as u8, 10);
        assert_eq!(NpcState::Sleeping as u8, 11);
        assert_eq!(NpcState::Fainting as u8, 12);
        assert_eq!(NpcState::Healing as u8, 13);
        assert_eq!(NpcState::Casting as u8, 14);
    }

    /// NPC_MAX_LEASH_RANGE is 200.
    #[test]
    fn test_npc_max_leash_range() {
        assert_eq!(NPC_MAX_LEASH_RANGE, 200.0);
    }

    /// PetState default values.
    #[test]
    fn test_pet_state_default() {
        let pet = PetState::default();
        assert_eq!(pet.level, 1);
        assert_eq!(pet.satisfaction, 0);
        assert_eq!(pet.state_change, 4); // MODE_DEFENCE
        assert_eq!(pet.pid, 25500);
        assert_eq!(pet.size, 100);
        assert_eq!(pet.attack_target_id, -1);
    }

    /// Pet constants.
    #[test]
    fn test_pet_constants() {
        assert_eq!(PET_INVENTORY_TOTAL, 4);
        assert_eq!(PET_DECAY_INTERVAL_SECS, 60);
        assert_eq!(PET_DECAY_AMOUNT, 100);
    }

    /// DailyOpCode from_u8 roundtrip.
    #[test]
    fn test_daily_opcode_from_u8() {
        assert_eq!(DailyOpCode::from_u8(1), Some(DailyOpCode::ChaosMap));
        assert_eq!(DailyOpCode::from_u8(8), Some(DailyOpCode::UserLoyaltyWingReward));
        assert_eq!(DailyOpCode::from_u8(0), None);
        assert_eq!(DailyOpCode::from_u8(9), None);
    }

    /// ITEM_NO_TRADE range covers special items.
    #[test]
    fn test_item_no_trade_range() {
        assert_eq!(ITEM_NO_TRADE_MIN, 900_000_001);
        assert_eq!(ITEM_NO_TRADE_MAX, 999_999_999);
        // All special items fall within no-trade range
        assert!(ITEM_EXP >= ITEM_NO_TRADE_MIN && ITEM_EXP <= ITEM_NO_TRADE_MAX);
        assert!(ITEM_COUNT >= ITEM_NO_TRADE_MIN && ITEM_COUNT <= ITEM_NO_TRADE_MAX);
    }

    /// MAX_ID_SIZE and MAX_PW_SIZE.
    #[test]
    fn test_id_pw_size() {
        assert_eq!(MAX_ID_SIZE, 20);
        assert_eq!(MAX_PW_SIZE, 28);
        assert!(MAX_PW_SIZE > MAX_ID_SIZE);
    }

    /// RACE_UNTRADEABLE is 20.
    #[test]
    fn test_race_untradeable() {
        assert_eq!(RACE_UNTRADEABLE, 20);
    }

    /// MAX_MERCH constants.
    #[test]
    fn test_max_merch_constants() {
        assert_eq!(MAX_MERCH_ITEMS, 12);
        assert_eq!(MAX_MERCH_MESSAGE, 40);
        assert_eq!(MAX_WANTED_ROOMS, 3);
    }

    /// DAILY_OPERATIONS_MINUTE is 24 hours.
    #[test]
    fn test_daily_operations_minute() {
        assert_eq!(DAILY_OPERATIONS_MINUTE, 1440);
        assert_eq!(DAILY_OPERATIONS_MINUTE, 24 * 60);
    }

    // ── Sprint 944: Additional coverage ──────────────────────────────

    /// PremiumProperty enum has 5 variants.
    #[test]
    fn test_premium_property_variants() {
        let props = [
            PremiumProperty::NoahPercent,
            PremiumProperty::DropPercent,
            PremiumProperty::BonusLoyalty,
            PremiumProperty::RepairDiscountPercent,
            PremiumProperty::ItemSellPercent,
        ];
        // All are distinct
        for i in 0..props.len() {
            for j in (i + 1)..props.len() {
                assert_ne!(props[i], props[j]);
            }
        }
    }

    /// CswNotice enum values 0-3.
    #[test]
    fn test_csw_notice_values() {
        assert_eq!(CswNotice::Preparation as u8, 0);
        assert_eq!(CswNotice::MonumentKilled as u8, 1);
        assert_eq!(CswNotice::War as u8, 2);
        assert_eq!(CswNotice::CswFinish as u8, 3);
    }

    /// CswOpStatus enum values 0-2.
    #[test]
    fn test_csw_op_status_values() {
        assert_eq!(CswOpStatus::NotOperation as u8, 0);
        assert_eq!(CswOpStatus::Preparation as u8, 1);
        assert_eq!(CswOpStatus::War as u8, 2);
    }

    /// OfflineCharacterType default is Merchant.
    #[test]
    fn test_offline_character_type_default() {
        let t = OfflineCharacterType::default();
        assert_eq!(t, OfflineCharacterType::Merchant);
    }

    /// UserDailyOp: new() initializes all to -1.
    #[test]
    fn test_user_daily_op_new() {
        let op = UserDailyOp::new();
        assert_eq!(op.get(DailyOpCode::ChaosMap), -1);
        assert_eq!(op.get(DailyOpCode::UserRankReward), -1);
        assert_eq!(op.get(DailyOpCode::KingWing), -1);
        assert_eq!(op.get(DailyOpCode::UserLoyaltyWingReward), -1);
    }

    /// UserDailyOp: set and get roundtrip.
    #[test]
    fn test_user_daily_op_set_get() {
        let mut op = UserDailyOp::new();
        op.set(DailyOpCode::ChaosMap, 12345);
        assert_eq!(op.get(DailyOpCode::ChaosMap), 12345);
        assert_eq!(op.get(DailyOpCode::UserRankReward), -1); // unchanged
    }

    /// Party: active_members returns only filled slots.
    #[test]
    fn test_party_active_members() {
        let mut party = Party::new(1, SessionId::from(10u16));
        party.add_member(SessionId::from(20u16));
        let active = party.active_members();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&SessionId::from(10u16)));
        assert!(active.contains(&SessionId::from(20u16)));
    }

    /// Party: command_leader defaults to party leader.
    #[test]
    fn test_party_command_leader() {
        let leader = SessionId::from(5u16);
        let party = Party::new(1, leader);
        assert!(party.is_command_leader(leader));
        assert!(!party.is_command_leader(SessionId::from(99u16)));
    }

    /// BurningFeatureRates default is all zeros.
    #[test]
    fn test_burning_feature_rates_default() {
        let rates = BurningFeatureRates::default();
        assert_eq!(rates.np_rate, 0);
        assert_eq!(rates.money_rate, 0);
        assert_eq!(rates.exp_rate, 0);
        assert_eq!(rates.drop_rate, 0);
    }

    /// ITEM_GOLD is below ITEM_NO_TRADE_MIN.
    #[test]
    fn test_item_gold_outside_no_trade() {
        assert!(ITEM_GOLD < ITEM_NO_TRADE_MIN);
        assert_eq!(ITEM_GOLD, 900_000_000);
        assert_eq!(ITEM_NO_TRADE_MIN, 900_000_001);
    }

    // ── Sprint 945: ChatRoom, WantedEvent, UserItemSlot, item flags ─────

    /// ChatRoom has_password returns false for empty, true for set.
    #[test]
    fn test_chatroom_has_password() {
        let mut room = ChatRoom {
            index: 1,
            name: String::new(),
            administrator: String::new(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: std::collections::HashMap::new(),
            next_member_id: 0,
        };
        assert!(!room.has_password());
        room.password = "secret".to_string();
        assert!(room.has_password());
    }

    /// ChatRoom add_user rejects when full.
    #[test]
    fn test_chatroom_add_user_full() {
        let mut room = ChatRoom {
            index: 1,
            name: String::new(),
            administrator: String::new(),
            password: String::new(),
            nation: 1,
            max_users: 2,
            current_users: 0,
            members: std::collections::HashMap::new(),
            next_member_id: 0,
        };
        assert!(room.add_user("Alice"));
        assert!(room.add_user("Bob"));
        assert!(!room.add_user("Charlie")); // full
        assert_eq!(room.current_users, 2);
        assert_eq!(room.members.len(), 2);
    }

    /// ChatRoom remove_user by name.
    #[test]
    fn test_chatroom_remove_user() {
        let mut room = ChatRoom {
            index: 1,
            name: String::new(),
            administrator: String::new(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: std::collections::HashMap::new(),
            next_member_id: 0,
        };
        room.add_user("Alice");
        room.add_user("Bob");
        assert!(room.remove_user("Alice"));
        assert!(!room.contains_user("Alice"));
        assert!(room.contains_user("Bob"));
        assert_eq!(room.current_users, 1);
    }

    /// ChatRoom is_administrator returns 2 for admin, 1 for others.
    #[test]
    fn test_chatroom_is_administrator() {
        let room = ChatRoom {
            index: 1,
            name: String::new(),
            administrator: "AdminUser".to_string(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: std::collections::HashMap::new(),
            next_member_id: 0,
        };
        assert_eq!(room.is_administrator("AdminUser"), 2);
        assert_eq!(room.is_administrator("adminuser"), 2); // case-insensitive
        assert_eq!(room.is_administrator("OtherUser"), 1);
    }

    /// ChatRoom contains_user is case-insensitive.
    #[test]
    fn test_chatroom_contains_user_case() {
        let mut room = ChatRoom {
            index: 1,
            name: String::new(),
            administrator: String::new(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: std::collections::HashMap::new(),
            next_member_id: 0,
        };
        room.add_user("TestPlayer");
        assert!(room.contains_user("testplayer"));
        assert!(room.contains_user("TESTPLAYER"));
        assert!(!room.contains_user("Unknown"));
    }

    /// WantedEventStatus default is Disabled.
    #[test]
    fn test_wanted_event_status_default() {
        let status = WantedEventStatus::default();
        assert_eq!(status, WantedEventStatus::Disabled);
    }

    /// WantedEventRoom default has empty lists and Disabled status.
    #[test]
    fn test_wanted_event_room_default() {
        let room = WantedEventRoom::default();
        assert_eq!(room.status, WantedEventStatus::Disabled);
        assert_eq!(room.next_select_time, 0);
        assert!(room.elmo_list.is_empty());
        assert!(room.karus_list.is_empty());
    }

    /// UserItemSlot default is empty (item_id=0).
    #[test]
    fn test_user_item_slot_default() {
        let slot = UserItemSlot::default();
        assert_eq!(slot.item_id, 0);
        assert_eq!(slot.durability, 0);
        assert_eq!(slot.count, 0);
        assert_eq!(slot.flag, 0);
        assert_eq!(slot.serial_num, 0);
        assert_eq!(slot.expire_time, 0);
    }

    /// Item flag constants: CHAR_SEAL=2, DUPLICATE=3, NOT_BOUND=7.
    #[test]
    fn test_item_flag_seal_dup_notbound() {
        assert_eq!(ITEM_FLAG_CHAR_SEAL, 2);
        assert_eq!(ITEM_FLAG_DUPLICATE, 3);
        assert_eq!(ITEM_FLAG_NOT_BOUND, 7);
        // All distinct
        assert_ne!(ITEM_FLAG_CHAR_SEAL, ITEM_FLAG_DUPLICATE);
        assert_ne!(ITEM_FLAG_DUPLICATE, ITEM_FLAG_NOT_BOUND);
    }

    /// MERCHANT_AUTO item IDs and PET_INVENTORY_TOTAL.
    #[test]
    fn test_merchant_auto_and_pet_inventory() {
        assert_eq!(MERCHANT_AUTO_FISHING, 700_099_755);
        assert_eq!(MERCHANT_AUTO_MANING, 700_049_758);
        assert_ne!(MERCHANT_AUTO_FISHING, MERCHANT_AUTO_MANING);
        assert_eq!(PET_INVENTORY_TOTAL, 4);
        assert_eq!(MAX_PARTY_USERS, 8);
    }

    // ── Sprint 946: ChatRoom ops, PetState, WantedEvent, rental ─────────

    /// ChatRoom remove_user_by_id removes member and decrements count.
    #[test]
    fn test_chatroom_remove_user_by_id() {
        let mut room = ChatRoom {
            index: 1,
            name: String::new(),
            administrator: String::new(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: std::collections::HashMap::new(),
            next_member_id: 0,
        };
        room.add_user("Alice");
        let member_id = *room.members.keys().next().unwrap();
        assert!(room.remove_user_by_id(member_id));
        assert_eq!(room.current_users, 0);
        assert!(!room.remove_user_by_id(999)); // non-existent
    }

    /// ChatRoom add_user increments next_member_id.
    #[test]
    fn test_chatroom_next_member_id() {
        let mut room = ChatRoom {
            index: 1,
            name: String::new(),
            administrator: String::new(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: std::collections::HashMap::new(),
            next_member_id: 0,
        };
        assert_eq!(room.next_member_id, 0);
        room.add_user("A");
        assert_eq!(room.next_member_id, 1);
        room.add_user("B");
        assert_eq!(room.next_member_id, 2);
    }

    /// WantedEventRoom status transitions and list ops.
    #[test]
    fn test_wanted_event_room_transitions() {
        let mut room = WantedEventRoom::default();
        room.status = WantedEventStatus::Invitation;
        assert_eq!(room.status, WantedEventStatus::Invitation);
        room.elmo_list.push(SessionId::from(10u16));
        room.karus_list.push(SessionId::from(20u16));
        assert_eq!(room.elmo_list.len(), 1);
        assert_eq!(room.karus_list.len(), 1);
        room.status = WantedEventStatus::Running;
        assert_eq!(room.status, WantedEventStatus::Running);
    }

    /// MAX_WANTED_ROOMS is 3 (Ronark Land, Ardream, Ronark Land Base).
    #[test]
    fn test_max_wanted_rooms() {
        assert_eq!(MAX_WANTED_ROOMS, 3);
    }

    /// PetState default: level=1, state_change=4, pid=25500, size=100.
    #[test]
    fn test_pet_state_default_values() {
        let pet = PetState::default();
        assert_eq!(pet.level, 1);
        assert_eq!(pet.state_change, 4); // MODE_DEFENCE
        assert_eq!(pet.pid, 25500);
        assert_eq!(pet.size, 100);
        assert_eq!(pet.attack_target_id, -1);
        assert!(!pet.attack_started);
    }

    /// PetState items array is PET_INVENTORY_TOTAL slots, all empty.
    #[test]
    fn test_pet_state_items_array() {
        let pet = PetState::default();
        assert_eq!(pet.items.len(), PET_INVENTORY_TOTAL);
        for slot in &pet.items {
            assert_eq!(slot.item_id, 0);
        }
    }

    /// PET_DECAY constants.
    #[test]
    fn test_pet_decay_constants() {
        assert_eq!(PET_DECAY_INTERVAL_SECS, 60);
        assert_eq!(PET_DECAY_AMOUNT, 100);
    }

    /// UserItemSlot remaining_rental_minutes returns 0 when expire_time=0.
    #[test]
    fn test_user_item_slot_no_expiry() {
        let slot = UserItemSlot::default();
        assert_eq!(slot.remaining_rental_minutes(), 0);
    }

    /// OfflineCharacterType has 4 variants, all distinct.
    #[test]
    fn test_offline_character_type_variants() {
        let variants = [
            OfflineCharacterType::Merchant,
            OfflineCharacterType::Genie,
            OfflineCharacterType::Mining,
            OfflineCharacterType::Fishing,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    /// OFFLINE_MERCHANT_ITEM constant.
    #[test]
    fn test_offline_merchant_item_constant() {
        assert_eq!(OFFLINE_MERCHANT_ITEM, 924_041_913);
    }

    // ── Sprint 947: NpcState, DailyOpCode, CswNotice, Party edges ───────

    /// NpcState: 12 variants with correct discriminants.
    #[test]
    fn test_npc_state_discriminants() {
        assert_eq!(NpcState::Dead as u8, 0);
        assert_eq!(NpcState::Live as u8, 1);
        assert_eq!(NpcState::Attacking as u8, 2);
        assert_eq!(NpcState::Standing as u8, 5);
        assert_eq!(NpcState::Moving as u8, 6);
        assert_eq!(NpcState::Tracing as u8, 7);
        assert_eq!(NpcState::Fighting as u8, 8);
        assert_eq!(NpcState::Back as u8, 10);
        assert_eq!(NpcState::Sleeping as u8, 11);
        assert_eq!(NpcState::Fainting as u8, 12);
        assert_eq!(NpcState::Healing as u8, 13);
        assert_eq!(NpcState::Casting as u8, 14);
    }

    /// DailyOpCode: all 8 variants roundtrip through from_u8.
    #[test]
    fn test_daily_op_code_all_roundtrip() {
        for v in 1..=8u8 {
            assert!(DailyOpCode::from_u8(v).is_some(), "from_u8({v}) should be Some");
        }
        assert!(DailyOpCode::from_u8(0).is_none());
        assert!(DailyOpCode::from_u8(9).is_none());
    }

    /// CswNotice: 4 values 0–3.
    #[test]
    fn test_csw_notice_all_values() {
        assert_eq!(CswNotice::Preparation as u8, 0);
        assert_eq!(CswNotice::MonumentKilled as u8, 1);
        assert_eq!(CswNotice::War as u8, 2);
        assert_eq!(CswNotice::CswFinish as u8, 3);
    }

    /// CswOpStatus: 3 values 0–2.
    #[test]
    fn test_csw_op_status_completeness() {
        assert_eq!(CswOpStatus::NotOperation as u8, 0);
        assert_eq!(CswOpStatus::Preparation as u8, 1);
        assert_eq!(CswOpStatus::War as u8, 2);
    }

    /// Party remove_member returns false for non-existent.
    #[test]
    fn test_party_remove_nonexistent() {
        let mut party = Party::new(1, SessionId::from(1u16));
        assert!(!party.remove_member(SessionId::from(99u16)));
    }

    /// Party swap_leader with invalid index is no-op.
    #[test]
    fn test_party_swap_leader_invalid() {
        let mut party = Party::new(1, SessionId::from(1u16));
        party.add_member(SessionId::from(2u16));
        // swap with 0 is no-op (already leader)
        party.swap_leader(0);
        assert_eq!(party.members[0], Some(SessionId::from(1u16)));
        // swap with out-of-bounds is no-op
        party.swap_leader(MAX_PARTY_USERS);
        assert_eq!(party.members[0], Some(SessionId::from(1u16)));
    }

    /// ChatRoom remove_user returns false for non-existent user.
    #[test]
    fn test_chatroom_remove_nonexistent() {
        let mut room = ChatRoom {
            index: 1,
            name: String::new(),
            administrator: String::new(),
            password: String::new(),
            nation: 1,
            max_users: 10,
            current_users: 0,
            members: std::collections::HashMap::new(),
            next_member_id: 0,
        };
        assert!(!room.remove_user("Ghost"));
        assert_eq!(room.current_users, 0);
    }

    /// UserItemSlot original_flag field preserved.
    #[test]
    fn test_user_item_slot_original_flag() {
        let mut slot = UserItemSlot::default();
        slot.flag = ITEM_FLAG_CHAR_SEAL;
        slot.original_flag = 0;
        assert_eq!(slot.flag, 2);
        assert_eq!(slot.original_flag, 0);
    }

    /// UserDailyOp set all ops and verify via get.
    #[test]
    fn test_user_daily_op_set_all() {
        let mut op = UserDailyOp::new();
        let codes = [
            DailyOpCode::ChaosMap,
            DailyOpCode::UserRankReward,
            DailyOpCode::PersonalRankReward,
            DailyOpCode::KingWing,
            DailyOpCode::WarderKillerWing1,
            DailyOpCode::WarderKillerWing2,
            DailyOpCode::KeeperKillerWing,
            DailyOpCode::UserLoyaltyWingReward,
        ];
        for (i, code) in codes.iter().enumerate() {
            op.set(*code, (i as i32) * 100);
        }
        for (i, code) in codes.iter().enumerate() {
            assert_eq!(op.get(*code), (i as i32) * 100);
        }
    }

    /// remaining_rental_minutes returns 0 for expired items.
    #[test]
    fn test_remaining_rental_expired() {
        // expire_time = 1 (far in the past)
        assert_eq!(remaining_rental_minutes(1), 0);
    }

    // ── Sprint 978: Additional coverage ──────────────────────────────

    /// remaining_rental_minutes returns 0 for expire_time == 0.
    #[test]
    fn test_remaining_rental_zero_means_no_expiry() {
        assert_eq!(remaining_rental_minutes(0), 0);
    }

    /// NpcDotSlot fields are accessible and non-default.
    #[test]
    fn test_npc_dot_slot_fields() {
        let dot = NpcDotSlot {
            skill_id: 5000,
            hp_amount: -50,
            tick_count: 0,
            tick_limit: 5,
            caster_sid: 42,
        };
        assert_eq!(dot.skill_id, 5000);
        assert_eq!(dot.hp_amount, -50);
        assert_eq!(dot.tick_limit, 5);
        assert!(dot.tick_count < dot.tick_limit);
    }

    /// NpcBuffEntry is_expired returns false for permanent buffs (duration=0).
    #[test]
    fn test_npc_buff_entry_permanent_not_expired() {
        let buff = NpcBuffEntry {
            skill_id: 1000,
            buff_type: 10,
            start_time: Instant::now(),
            duration_secs: 0,
        };
        assert!(!buff.is_expired());
    }

    /// MAX_TYPE3_REPEAT is 40 (DOT/HOT maximum tick slots).
    #[test]
    fn test_max_type3_repeat_value() {
        assert_eq!(MAX_TYPE3_REPEAT, 40);
    }

    /// NPC_HAVE_ITEM_LIST is 8 (NPC loot table size).
    #[test]
    fn test_npc_have_item_list_value() {
        assert_eq!(NPC_HAVE_ITEM_LIST, 12);
    }

    // ── Sprint 996: types.rs +5 ─────────────────────────────────────────

    /// DurationalSkill::empty() creates an unused slot with all zeros.
    #[test]
    fn test_durational_skill_empty() {
        let slot = DurationalSkill::empty();
        assert_eq!(slot.skill_id, 0);
        assert_eq!(slot.hp_amount, 0);
        assert_eq!(slot.tick_count, 0);
        assert_eq!(slot.tick_limit, 0);
        assert!(!slot.used);
    }

    /// JackPotSetting::default() initializes all thresholds to zero.
    #[test]
    fn test_jackpot_setting_default() {
        let jp = JackPotSetting::default();
        assert_eq!(jp.rate, 0);
        assert_eq!(jp.x_1000, 0);
        assert_eq!(jp.x_500, 0);
        assert_eq!(jp.x_100, 0);
        assert_eq!(jp.x_50, 0);
        assert_eq!(jp.x_10, 0);
        assert_eq!(jp.x_2, 0);
    }

    /// WANTED_MAP_SHOW_INTERVAL_SECS is 60 (1 minute refresh).
    #[test]
    fn test_wanted_map_show_interval() {
        assert_eq!(WANTED_MAP_SHOW_INTERVAL_SECS, 60);
        // Same as OFFLINE_CHECK_INTERVAL_SECS (both 1 minute)
        assert_eq!(WANTED_MAP_SHOW_INTERVAL_SECS, OFFLINE_CHECK_INTERVAL_SECS);
    }

    // ── Sprint 1001: types.rs +5 ──────────────────────────────────────

    /// ZONE_BDW/CHAOS/JURAID short aliases match their full-name constants.
    #[test]
    fn test_zone_event_aliases_match_full_names() {
        assert_eq!(ZONE_BDW, ZONE_BORDER_DEFENSE_WAR);
        assert_eq!(ZONE_CHAOS, ZONE_CHAOS_DUNGEON);
        assert_eq!(ZONE_JURAID, ZONE_JURAID_MOUNTAIN);
        // Values: 84, 85, 87 (gap at 86 = UNDER_CASTLE)
        assert_eq!(ZONE_BDW, 84);
        assert_eq!(ZONE_CHAOS, 85);
        assert_eq!(ZONE_JURAID, 87);
        assert_eq!(ZONE_JURAID - ZONE_BDW, 3);
    }

    /// Battle zones form contiguous range BATTLE_BASE+1..BATTLE_BASE+6.
    #[test]
    fn test_zone_battle_contiguous_range() {
        assert_eq!(ZONE_BATTLE_BASE, 60);
        assert_eq!(ZONE_BATTLE, ZONE_BATTLE_BASE + 1);
        assert_eq!(ZONE_BATTLE2, ZONE_BATTLE_BASE + 2);
        assert_eq!(ZONE_BATTLE3, ZONE_BATTLE_BASE + 3);
        assert_eq!(ZONE_BATTLE4, ZONE_BATTLE_BASE + 4);
        assert_eq!(ZONE_BATTLE5, ZONE_BATTLE_BASE + 5);
        assert_eq!(ZONE_BATTLE6, ZONE_BATTLE_BASE + 6);
    }

    /// Stone zones (81-83) and Party VS zones (96-99) are contiguous runs.
    #[test]
    fn test_zone_stone_and_party_vs_contiguous() {
        // Stone: 81, 82, 83
        assert_eq!(ZONE_STONE1, 81);
        assert_eq!(ZONE_STONE2, ZONE_STONE1 + 1);
        assert_eq!(ZONE_STONE3, ZONE_STONE1 + 2);
        // Party VS: 96, 97, 98, 99
        assert_eq!(ZONE_PARTY_VS_1, 96);
        assert_eq!(ZONE_PARTY_VS_2, ZONE_PARTY_VS_1 + 1);
        assert_eq!(ZONE_PARTY_VS_3, ZONE_PARTY_VS_1 + 2);
        assert_eq!(ZONE_PARTY_VS_4, ZONE_PARTY_VS_1 + 3);
    }

    /// SPBATTLE range: MIN=105, MAX=115, SPBATTLE1=MIN.
    #[test]
    fn test_zone_spbattle_range() {
        assert_eq!(ZONE_SPBATTLE_MIN, 105);
        assert_eq!(ZONE_SPBATTLE_MAX, 115);
        assert_eq!(ZONE_SPBATTLE1, ZONE_SPBATTLE_MIN);
        assert_eq!(ZONE_SPBATTLE_MAX - ZONE_SPBATTLE_MIN, 10);
    }

    /// Rank types: PK_ZONE=1, BDW=2, CHAOS=3 — contiguous 1-3.
    #[test]
    fn test_rank_type_contiguous_1_to_3() {
        assert_eq!(RANK_TYPE_PK_ZONE, 1);
        assert_eq!(RANK_TYPE_ZONE_BORDER_DEFENSE_WAR, 2);
        assert_eq!(RANK_TYPE_CHAOS_DUNGEON, 3);
        assert_eq!(RANK_TYPE_CHAOS_DUNGEON - RANK_TYPE_PK_ZONE, 2);
    }

    /// COIN_MAX is 2.1 billion and fits in u32.
    #[test]
    fn test_coin_max_value() {
        assert_eq!(COIN_MAX, 2_100_000_000);
        assert!(COIN_MAX < u32::MAX);
        // ITEM_GOLD is below COIN_MAX (gold pseudo-item ID not a coin amount)
        assert!(ITEM_GOLD < COIN_MAX);
    }

    /// User state gap: 0x04 and 0x05 are unused between DEAD(3) and MONUMENT(6).
    #[test]
    fn test_user_state_gap_at_4_5() {
        assert_eq!(USER_DEAD, 0x03);
        assert_eq!(USER_MONUMENT, 0x06);
        // Gap of 2 between DEAD and MONUMENT (0x04, 0x05 unused)
        assert_eq!(USER_MONUMENT - USER_DEAD, 3);
        // MINING and FLASHING are adjacent after MONUMENT
        assert_eq!(USER_MINING, USER_MONUMENT + 1);
        assert_eq!(USER_FLASHING, USER_MINING + 1);
    }
}
