//! Packet handler dispatch — routes opcodes to handler functions.
//! Each handler receives a mutable reference to the session and the
//! incoming packet, processes it, and sends the appropriate response.

// -- Shared inventory/item constants -----------------------------------------
// Canonical definitions live in `crate::inventory_constants`.
// Re-exported here so existing `use super::SLOT_MAX` imports continue to work.

pub use crate::inventory_constants::{
    COIN_MAX, COSP_MAX, HAVE_MAX, INVENTORY_COSP, INVENTORY_MBAG, INVENTORY_TOTAL,
    ITEM_KIND_COSPRE, ITEM_KIND_PET, ITEM_KIND_UNIQUE, SLOT_MAX, WAREHOUSE_MAX, WEAPON_KIND_1H_AXE,
    WEAPON_KIND_1H_CLUB, WEAPON_KIND_1H_SPEAR, WEAPON_KIND_1H_SWORD, WEAPON_KIND_2H_AXE,
    WEAPON_KIND_2H_CLUB, WEAPON_KIND_2H_SPEAR, WEAPON_KIND_2H_SWORD, WEAPON_KIND_BOW,
    WEAPON_KIND_CROSSBOW, WEAPON_KIND_DAGGER, WEAPON_KIND_JAMADAR,
};
pub use crate::world::ITEMCOUNT_MAX;

pub mod ability;
pub mod achieve;
pub mod achievement2;
pub mod allchar;
pub mod arena;
pub mod attack;
pub mod attendance;
pub mod audit_log;
pub mod awakening;
pub mod bifrost;
pub mod bundle_open;
pub mod cape;
pub mod captcha;
pub mod challenge;
pub mod change_hair;
pub mod chaos_stone;
pub mod character_seal;
pub mod chat;
pub mod cinderella;
pub mod clan_battle;
pub mod clan_nts;
pub mod clan_premium;
pub mod clan_warehouse;
pub mod clanpoints_battle;
pub mod class_change;
pub mod client_event;
pub mod collection;
pub mod collection_race;
pub mod concurrent_user;
pub mod continuous_packet;
pub mod corpse;
pub mod costume;
pub mod cryption;
pub mod daily_quest;
pub mod daily_quest_v2525;
pub mod daily_rank;
pub mod datasave;
pub mod dead;
pub mod delchar;
pub mod draki_tower;
pub mod dungeon_defence;
pub mod durability;
pub mod edit_box;
pub mod enchant;
pub mod event;
pub mod exp_seal;
pub mod ext_hook;
pub mod forgotten_temple;
pub mod friend;
pub mod gamestart;
pub mod gender_change;
pub mod genie;
pub mod guild_bank;
pub mod hacktool;
pub mod helmet;
pub mod home;
pub mod item_drop;
pub mod item_get;
pub mod item_move;
pub mod item_production;
pub mod item_remove;
pub mod item_repair;
pub mod item_upgrade;
pub mod king;
pub mod kiss;
pub mod knight_cash;
pub mod knights;
pub mod letter;
pub mod level;
pub mod loading_login;
pub mod login;
pub mod logosshout;
pub mod logout;
pub mod lottery;
pub mod magic_process;
pub mod map_event;
pub mod market_bbs;
pub mod max_hp_change;
pub mod merchant;
pub mod mining;
pub mod monument;
pub mod moraranker;
pub mod move_handler;
pub mod moving_tower;
pub mod name_change;
pub mod nation;
pub mod nation_transfer;
pub mod native_events;
pub mod newchar;
pub mod notice;
pub mod npc_loot;
pub mod npc_trade;
pub mod object_event;
pub mod operator;
pub mod packet2;
pub mod party;
pub mod party_bbs;
pub mod perks;
pub mod pet;
pub mod premium;
pub mod premium2;
pub mod preset;
pub mod program_check;
pub mod pus_refund;
pub mod quest;
pub mod rank;
pub mod rebirth;
pub mod regene;
pub mod region;
pub mod rental;
pub mod req_npcin;
pub mod req_userin;
pub mod room;
pub mod rotate;
pub mod seal;
pub mod season;
pub mod selchar;
pub mod select_msg;
pub mod server_change;
pub mod server_index;
pub mod sheriff;
pub mod shopping_mall;
pub mod siege;
pub mod skilldata;
pub mod soccer;
pub mod soul;
pub mod speedhack;
pub mod state_change;
pub mod stats;
pub mod stealth;
pub mod story;
pub mod survival;
pub mod tag_change;
pub mod target_hp;
pub mod terrain_effects;
pub mod territory;
pub mod tournament;
pub mod trade;
pub mod under_castle;
pub mod unique_item_info;
pub mod upgrade_notice;
pub mod user_info;
pub mod v2525;
pub mod vanguard;
pub mod version;
pub mod vip_warehouse;
pub mod virtual_server;
pub mod warehouse;
pub mod warp_list;
pub mod webpage;
pub mod weight_change;
pub mod wheel_of_fun;
pub mod world_boss;
pub mod zone_ability;
pub mod zone_change;
pub mod zone_concurrent;

use std::sync::Arc;

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::{debug, error, warn};

use crate::session::{ClientSession, SessionState};
use crate::systems::time_weather::{build_weather_packet, WEATHER_FINE};

/// Check if an opcode is blocked during Cinderella War for event participants.
/// When a player is an event user AND in the Cinderella zone, these handlers
/// are silently skipped (packet dropped).
fn is_cinderella_blocked_opcode(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::WizRental
            | Opcode::WizItemTrade
            | Opcode::WizBundleOpenReq
            | Opcode::WizItemGet
            | Opcode::WizExchange
            | Opcode::WizMerchant
            | Opcode::WizDatasave
            | Opcode::WizWarehouse
            | Opcode::WizClanWarehouse
            | Opcode::WizShoppingMall
            | Opcode::WizMining
            | Opcode::WizSiege
            | Opcode::WizNationTransfer
            | Opcode::WizGenderChange
            | Opcode::WizChangeHair
            | Opcode::WizVipwarehouse
            | Opcode::WizMovingTower
            | Opcode::WizUserAchieve
            | Opcode::WizPet
            | Opcode::WizEditBox
    )
}

/// Dispatch an incoming packet to the appropriate handler.
pub async fn dispatch(session: &mut ClientSession, packet: Packet) -> anyhow::Result<()> {
    let opcode = Opcode::from_byte(packet.opcode);

    debug!(
        "[{}] GS received opcode: {:?} (0x{:02X}), data({} bytes)",
        session.addr(),
        opcode,
        packet.opcode,
        packet.data.len(),
    );

    // Update activity timestamp after successful packet receive.
    session.world().touch_session(session.session_id());

    // ── Rate limit check ──────────────────────────────────────────────────
    // Drop packets that exceed per-session or per-opcode rate limits.
    // Critical floods (>300 pps) disconnect the session entirely.
    // GMs (authority == 0) bypass all rate limits.
    // C++ has no equivalent — this is a server-hardening addition.
    {
        let world = session.world().clone();
        let sid = session.session_id();
        let is_gm = world.is_gm(sid);
        if let Err(e) = world
            .rate_limiter()
            .check_rate_limit(sid, packet.opcode, is_gm)
        {
            if e.should_disconnect() {
                // Critical flood or temporary ban — disconnect the session.
                error!(
                    "[{}] Rate limit DISCONNECT: {} (opcode 0x{:02X})",
                    session.addr(),
                    e,
                    packet.opcode
                );
                anyhow::bail!("rate limit: session disconnected — {}", e);
            }
            // Soft limit — drop the packet but keep the session alive.
            warn!(
                "[{}] Rate limited: {} (opcode 0x{:02X} dropped)",
                session.addr(),
                e,
                packet.opcode
            );
            return Ok(());
        }
    }

    // ── Cinderella War handler gating ─────────────────────────────────────
    // When player is in Cinderella event zone, many handlers are silently blocked.
    if session.state() == SessionState::InGame {
        if let Some(op) = opcode {
            if is_cinderella_blocked_opcode(op) {
                let world = session.world().clone();
                let sid = session.session_id();
                if world.is_player_in_cinderella(sid) {
                    debug!(
                        "[{}] Cinderella gating: blocked opcode {:?} (0x{:02X})",
                        session.addr(),
                        op,
                        packet.opcode
                    );
                    return Ok(());
                }
            }
        }
    }

    let sync_exchange_slots = matches!(opcode, Some(Opcode::WizItemMove | Opcode::WizItemGet
        | Opcode::WizItemUpgrade | Opcode::WizWarehouse | Opcode::WizTradeNpc
        | Opcode::WizShoppingMall | Opcode::WizClientEvent | Opcode::WizNpcEvent | Opcode::WizSelectMsg))
        || (opcode == Some(Opcode::WizExtHook) && matches!((packet.data.first().copied(), packet.data.get(1).copied()),
            (Some(0xE6), Some(4 | 5)) | (Some(0xA8), Some(1 | 3)) | (Some(0xB4), _)));
    let result = match opcode {
        // --- Pre-auth ---
        Some(Opcode::WizKickout) if session.state() == SessionState::Connected => {
            // v2600: client sends 0x51 as first packet to game server (handshake).
            // Parse account + version, set state to VersionChecked, do NOT respond.
            // Sending opcode 0x01 triggers encryption dispatch "Connecting..." which
            // blocks the client. Instead, silently advance state and wait for the
            // client to send WIZ_LOGIN (0x01) or WIZ_CRYPTION (0x2C) next.
            let mut reader = PacketReader::new(&packet.data);
            let account = reader.read_string().unwrap_or_default();
            let client_version = reader.read_u32().unwrap_or(0);
            tracing::info!(
                "[{}] Game server handshake 0x51: account={}, client_version={}",
                session.addr(),
                account,
                client_version
            );
            session.set_state(SessionState::VersionChecked);
            Ok(())
        }
        Some(Opcode::WizVersionCheck) => version::handle(session, packet).await,
        Some(Opcode::WizCryption) => cryption::handle(session, packet).await,
        Some(Opcode::WizLogin) => login::handle(session, packet).await,
        // --- Character selection ---
        Some(Opcode::WizSelNation) => nation::handle(session, packet).await,
        Some(Opcode::WizAllcharInfoReq) => allchar::handle(session, packet).await,
        Some(Opcode::WizNewChar) => newchar::handle(session, packet).await,
        Some(Opcode::WizDelChar) => delchar::handle(session, packet).await,
        Some(Opcode::WizSelChar) => selchar::handle(session, packet).await,
        Some(Opcode::WizChangeHair) => change_hair::handle(session, packet).await,
        Some(Opcode::WizServerIndex) => server_index::handle(session, packet).await,
        Some(Opcode::WizLoadingLogin) => loading_login::handle(session, packet).await,
        // --- Game start ---
        Some(Opcode::WizGamestart) => gamestart::handle(session, packet).await,
        // 0x41: speedhack check — client sends periodically, original server ignores it.
        // MUST NOT route to gamestart — confuses client with unexpected MyInfo response.
        Some(Opcode::WizSpeedhackCheck) => Ok(()),
        Some(Opcode::WizLogout) => logout::handle(session, packet).await,
        // --- In-game: movement ---
        Some(Opcode::WizMove) => move_handler::handle(session, packet).await,
        Some(Opcode::WizRotate) => rotate::handle(session, packet),
        Some(Opcode::WizZoneChange) => zone_change::handle(session, packet).await,
        Some(Opcode::WizTargetHp) => target_hp::handle(session, packet).await,
        Some(Opcode::WizReqUserIn) => req_userin::handle(session, packet).await,
        Some(Opcode::WizReqNpcIn) => req_npcin::handle(session, packet).await,
        Some(Opcode::WizStateChange) => state_change::handle(session, packet).await,
        // --- In-game: systems (implemented) ---
        Some(Opcode::WizSkillData) => skilldata::handle(session, packet).await,
        Some(Opcode::WizUserInfo) => user_info::handle(session, packet).await,
        Some(Opcode::WizShoppingMall) => shopping_mall::handle(session, packet).await,
        Some(Opcode::WizKnightsProcess) => knights::handle(session, packet).await,
        Some(Opcode::WizFriendProcess) => friend::handle(session, packet).await,
        Some(Opcode::WizRental) => rental::handle(session, packet).await,
        Some(Opcode::WizHelmet) => helmet::handle(session, packet),
        Some(Opcode::WizGenie) => genie::handle(session, packet).await,
        Some(Opcode::WizHacktool) => hacktool::handle(session, packet),
        Some(Opcode::WizUserAchieve) => achieve::handle(session, packet).await,
        // --- In-game: combat, chat & environment ---
        Some(Opcode::WizAttack) => attack::handle(session, packet).await,
        Some(Opcode::WizChat) => chat::handle(session, packet).await,
        Some(Opcode::WizDead) => dead::handle(session, packet),
        Some(Opcode::WizRegene) => regene::handle(session, packet).await,
        Some(Opcode::WizTime) => handle_gm_time_weather(session, packet).await,
        Some(Opcode::WizWeather) => handle_gm_time_weather(session, packet).await,
        // Server→client only: client should never send these.
        Some(Opcode::WizHpChange) => Ok(()),
        Some(Opcode::WizMspChange) => Ok(()),
        Some(Opcode::WizNationChat) => chat::handle_nation_chat(session, packet).await,
        Some(Opcode::WizExpChange) => Ok(()), // server→client only
        Some(Opcode::WizLevelChange) => Ok(()), // server→client only
        Some(Opcode::WizItemMove) => item_move::handle(session, packet).await,
        Some(Opcode::WizNpcEvent) => client_event::handle_npc_event(session, packet).await,
        Some(Opcode::WizItemTrade) => npc_trade::handle(session, packet).await,
        Some(Opcode::WizItemDrop) => item_drop::handle(session, packet).await,
        Some(Opcode::WizBundleOpenReq) => bundle_open::handle(session, packet).await,
        Some(Opcode::WizTradeNpc) => Ok(()), // server→client only (opens merchant shop UI)
        Some(Opcode::WizItemGet) => item_get::handle(session, packet).await,
        Some(Opcode::WizPointChange) => stats::handle_point_change(session, packet).await,
        Some(Opcode::WizLoyaltyChange) => Ok(()), // server→client only (sent by NP gain/loss)
        Some(Opcode::WizUserlookChange) => Ok(()), // server→client only (sent by item_move)
        Some(Opcode::WizNotice) => Ok(()),        // server→client only (server announcements)
        Some(Opcode::WizParty) => party::handle(session, packet).await,
        Some(Opcode::WizExchange) => trade::handle(session, packet).await,
        Some(Opcode::WizMagicProcess) => magic_process::handle(session, packet).await,
        Some(Opcode::WizSkillptChange) => stats::handle_skillpt_change(session, packet).await,
        Some(Opcode::WizObjectEvent) => object_event::handle(session, packet).await,
        Some(Opcode::WizClassChange) => class_change::handle(session, packet).await,
        Some(Opcode::WizChatTarget) => chat::handle_chat_target(session, packet).await,
        Some(Opcode::WizDatasave) => datasave::handle(session, packet).await,
        Some(Opcode::WizConcurrentUser) => concurrent_user::handle(session, packet).await,
        Some(Opcode::WizDuration) => Ok(()), // server→client only (durability notification)
        Some(Opcode::WizRepairNpc) => Ok(()), // server→client only (opens tinker shop UI)
        Some(Opcode::WizItemRepair) => item_repair::handle(session, packet).await,
        Some(Opcode::WizKnightsList) => knights::handle_knights_list(session, packet).await,
        Some(Opcode::WizItemRemove) => item_remove::handle(session, packet).await,
        Some(Opcode::WizOperator) => operator::handle(session, packet).await,
        Some(Opcode::WizCompressPacket) => Ok(()), // compression wrapper, not used
        Some(Opcode::WizServerCheck) => Ok(()),    // keepalive, no response needed
        Some(Opcode::WizVirtualServer) => virtual_server::handle(session, packet),
        Some(Opcode::WizWarehouse) => warehouse::handle(session, packet).await,
        Some(Opcode::WizHome) => home::handle(session, packet).await,
        Some(Opcode::WizReportBug) => {
            debug!(
                "[{}] WIZ_REPORT_BUG (0x47): no-op (C++ just breaks)",
                session.addr()
            );
            Ok(())
        }
        Some(Opcode::WizGoldChange) => Ok(()), // server→client only (sent by gold_gain/lose)
        Some(Opcode::WizWarpList) => warp_list::handle(session, packet).await,
        Some(Opcode::WizPartyBbs) => party_bbs::handle(session, packet).await,
        Some(Opcode::WizClientEvent) => client_event::handle(session, packet).await,
        Some(Opcode::WizMapEvent) => map_event::handle(session, packet).await,
        Some(Opcode::WizWeightChange) => weight_change::handle(session, packet).await,
        Some(Opcode::WizSelectMsg) => select_msg::handle(session, packet).await,
        Some(Opcode::WizAuthorityChange) => Ok(()), // server→client only (fame/authority broadcast)
        Some(Opcode::WizEditBox) => edit_box::handle(session, packet).await,
        Some(Opcode::WizSanta) => Ok(()), // server→client only (flying Santa/Angel visual event)
        Some(Opcode::WizNpcSay) => Ok(()), // server→client only (sent by quest/Lua system)
        Some(Opcode::WizItemUpgrade) => {
            // ExchangeSystemProcess(pkt). When isCindIn, it also falls through
            // to TempleProcess(pkt) (WIZ_EVENT handler).
            item_upgrade::handle(session, packet.clone()).await?;
            if session.state() == SessionState::InGame {
                let world = session.world().clone();
                let sid = session.session_id();
                if world.is_player_in_cinderella(sid) {
                    event::handle(session, packet).await?;
                }
            }
            Ok(())
        }
        Some(Opcode::WizZoneability) => Ok(()), // server→client only (zone ability info + status effects)
        Some(Opcode::WizStealth) => stealth::handle(session, packet),
        Some(Opcode::WizEvent) => event::handle(session, packet).await,
        Some(Opcode::WizQuest) => quest::handle(session, packet).await,
        Some(Opcode::WizMerchant) => merchant::handle(session, packet).await,
        Some(Opcode::WizMerchantInout) => merchant::handle_inout(session, packet),
        Some(Opcode::WizEffect) => Ok(()), // server→client only (visual effects broadcast)
        Some(Opcode::WizSiege) => siege::handle(session, packet).await,
        Some(Opcode::WizNameChange) => name_change::handle(session, packet).await,
        Some(Opcode::WizCape) => cape::handle(session, packet).await,
        Some(Opcode::WizPremium) => premium::handle(session, packet).await,
        Some(Opcode::WizChallenge) => challenge::handle(session, packet).await,
        Some(Opcode::WizPet) => pet::handle(session, packet).await,
        Some(Opcode::WizKing) => king::handle(session, packet).await,
        Some(Opcode::WizProgramCheck) => program_check::handle(session, packet),
        Some(Opcode::WizReport) => sheriff::handle(session, packet).await,
        Some(Opcode::WizLogosshout) => logosshout::handle(session, packet).await,
        Some(Opcode::WizBifrost) => bifrost::handle(session, packet).await,
        Some(Opcode::WizRank) => rank::handle(session, packet).await,
        Some(Opcode::WizCaptcha) => captcha::handle(session, packet).await,
        Some(Opcode::WizDailyRank) => daily_rank::handle(session, packet).await,
        Some(Opcode::WizStory) => Ok(()), // server→client only (sent during game start)
        Some(Opcode::WizNationTransfer) => nation_transfer::handle(session, packet).await,
        Some(Opcode::WizTerrainEffects) => Ok(()), // server→client only (terrain effect notification)
        Some(Opcode::WizMovingTower) => moving_tower::handle(session, packet).await,
        // WIZ_PVP (0x88) is server→client only (PVPAssignRival/PVPRemoveRival/PVPUpdateHelmet/PVPResetHelmet).
        // Rival state is set by arena::on_pvp_kill (called from attack.rs PvP kill path).
        // Anger gauge is reset by arena::reset_anger_gauge (called from regene.rs).
        Some(Opcode::WizPvp) => Ok(()),
        Some(Opcode::WizMining) => mining::handle(session, packet).await,
        Some(Opcode::WizVipwarehouse) => vip_warehouse::handle(session, packet).await,
        Some(Opcode::WizGenderChange) => gender_change::handle(session, packet).await,
        Some(Opcode::WizExpSeal) => exp_seal::handle(session, packet).await,
        Some(Opcode::WizKurianSpChange) => Ok(()), // server→client only (Kurian SP update)
        Some(Opcode::WizSound) => Ok(()),          // server→client only (area sound effect)
        Some(Opcode::WizVanguard) => vanguard::handle(session, packet),
        Some(Opcode::WizPreset) => preset::handle(session, packet).await,
        Some(Opcode::WizClanWarehouse) => clan_warehouse::handle(session, packet).await,
        Some(Opcode::WizCinderella) => cinderella::handle(session, packet).await,
        Some(Opcode::WizPartyHp) => Ok(()), // server→client only
        // Server→client broadcast opcodes (sent by server, never received from client)
        Some(Opcode::WizUserInout) => Ok(()), // user region enter/leave
        Some(Opcode::WizNpcInout) => Ok(()),  // NPC region enter/leave
        Some(Opcode::WizNpcMove) => Ok(()),   // NPC movement interpolation
        Some(Opcode::WizRegionChange) => Ok(()), // region user/NPC list
        Some(Opcode::WizNpcRegion) => Ok(()), // NPC region list
        Some(Opcode::WizMyInfo) => Ok(()),    // character detail data
        Some(Opcode::WizWarp) => handle_recv_warp(session, packet).await,
        Some(Opcode::WizItemCountChange) => Ok(()), // item stack update
        Some(Opcode::WizBattleEvent) => Ok(()),     // battle results/rankings
        Some(Opcode::WizExtHook) => handle_ext_hook(session, packet).await,
        Some(Opcode::WizAutoDrop) => Ok(()), // auto-loot config (client-sent, no server action)
        Some(Opcode::WizMerchantList) => merchant::handle_merchant_list(session, packet).await,
        Some(Opcode::WizAddMsg) => Ok(()), // server→client scrolling notice
        // S2C-only opcodes with builders (server sends, client handles gracefully if echoed)
        Some(Opcode::WizCorpse)
        | Some(Opcode::WizClanPremium)
        | Some(Opcode::WizClanBattle)         // 0x63: empty notification, triggers C2S knights sub=6
        | Some(Opcode::WizClanpointsBattle)   // 0x91: [u8 type=1][u8 sub] clan notification display
        => {
            debug!("[{}] S2C-only opcode received from client: 0x{:02X}", session.addr(), packet.opcode);
            Ok(())
        }
        // ── Sprint 23: Minor features ──────────────────────────────────
        Some(Opcode::WizKiss) => kiss::handle(session, packet).await,
        Some(Opcode::WizWebpage) => webpage::handle(session, packet).await,
        Some(Opcode::WizZoneConcurrent) => zone_concurrent::handle(session, packet).await,
        Some(Opcode::WizMarketBbs) => market_bbs::handle(session, packet).await,
        Some(Opcode::WizRoomPacketProcess) => room::handle(session, packet).await,
        // C++ stub/no-op opcodes — dispatched but intentionally unhandled
        Some(Opcode::WizContinousPacket)
        | Some(Opcode::WizServerChange) // S2C-only: server_change.rs has builders
        | Some(Opcode::WizKickout)
        | Some(Opcode::WizPacket2)
        | Some(Opcode::WizRoom)
        | Some(Opcode::WizPpCardLogin)
        | Some(Opcode::WizRecommendUser)
        | Some(Opcode::WizItemExpiration)
        | Some(Opcode::WizChina)
        | Some(Opcode::WizCapture)
        | Some(Opcode::WizLoyaltyShop)
        | Some(Opcode::WizKillAssist)         // 0xC8: v2600 client dispatch default (no handler)
        | Some(Opcode::WizKnightRoyale) => {
            debug!("[{}] Stub opcode: 0x{:02X}", session.addr(), packet.opcode);
            Ok(())
        }
        // ── v2525 native opcodes (client dispatch 0x06-0xD7) ────────
        // C2S active handlers (client sends these):
        Some(Opcode::WizSeal) => seal::handle(session, packet).await,
        Some(Opcode::WizUpgradeNotice) => upgrade_notice::handle(session, packet).await,
        Some(Opcode::WizChallenge2) => v2525::handle_challenge2(session, packet),
        Some(Opcode::WizTerritory) => territory::handle(session, packet).await,
        // S2C-only opcodes (server→client, but handle gracefully if client sends):
        Some(Opcode::WizMaxHpChange) | Some(Opcode::WizAchievement2) => {
            v2525::handle_unexpected_c2s(session, packet)
        }
        Some(Opcode::WizAwakening) => awakening::handle(session, packet).await,
        // Each has independent dispatch (Table G only applies to 0xB7 Attendance):
        Some(Opcode::WizCollection1) => collection::handle_collection1(session, packet).await,
        Some(Opcode::WizCollection2) => collection::handle_collection2(session, packet).await,
        Some(Opcode::WizAttendance) => attendance::handle(session, packet).await,
        // UI-panel-dependent handlers:
        Some(Opcode::WizPremium2) => premium2::handle(session, packet).await,
        Some(Opcode::WizCostume) => costume::handle(session, packet).await,
        Some(Opcode::WizSoul) => soul::handle(session, packet).await,
        Some(Opcode::WizDailyQuest) => daily_quest_v2525::handle(session, packet).await,
        Some(Opcode::WizEnchant) => {
            let native = session.world().get_server_settings()
                .map(|s| s.game_version >= 2600).unwrap_or(false);
            if native { native_events::handle_jigsaw_coin(session, packet).await }
            else { enchant::handle(session, packet).await }
        },
        Some(Opcode::WizAbility) => {
            let native = session.world().get_server_settings()
                .map(|s| s.game_version >= 2600).unwrap_or(false);
            if native { native_events::handle_marble(session, packet).await }
            else { ability::handle(session, packet).await }
        },
        // 0xD0 is version-dependent: v2525 uses Guild Bank, while the
        // verified 26xx client contract uses Manes Survival. Missing settings
        // deliberately preserve the established Guild Bank behavior.
        Some(Opcode::WizGuildBank) => {
            let is_survival_client = session
                .world()
                .get_server_settings()
                .map(|settings| settings.game_version >= 2600)
                .unwrap_or(false);
            if is_survival_client {
                survival::handle(session, packet).await
            } else {
                guild_bank::handle(session, packet).await
            }
        },
        Some(Opcode::WizRebirth) => rebirth::handle(session, packet).await,
        Some(Opcode::WizWorldBoss) => world_boss::handle(session, packet).await,
        Some(Opcode::WizSeason) => season::handle(session, packet).await,
        Some(Opcode::WizRanker) => moraranker::handle(session, packet).await,
        // Special protocol (resource transfer — stub is correct):
        Some(Opcode::WizContinousPacketData) => {
            let native = session.world().get_server_settings()
                .map(|s| s.game_version >= 2600).unwrap_or(false);
            if native { native_events::handle_roulette(session, packet).await }
            else { continuous_packet::handle(session, packet) }
        },
        _ => {
            warn!(
                "[{}] Unhandled opcode: 0x{:02X}",
                session.addr(),
                packet.opcode
            );
            Ok(())
        }
    };
    if result.is_ok() && sync_exchange_slots && session.state() == SessionState::InGame {
        ext_hook::send_right_exchange_slots(session).await?;
    }
    result
}

/// Handle WIZ_TIME / WIZ_WEATHER from client (GM-only update).
/// GM clients can update the server's time or weather. Non-GM clients
/// are silently ignored (matching C++ behavior).
async fn handle_gm_time_weather(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Only GMs can update time/weather
    let char_info = match world.get_character_info(sid) {
        Some(info) => info,
        None => return Ok(()),
    };

    if char_info.authority != 0 {
        // Non-GM: silently ignore
        return Ok(());
    }

    let opcode = pkt.opcode;
    let tw = world.game_time_weather();

    if opcode == Opcode::WizWeather as u8 {
        let mut reader = PacketReader::new(&pkt.data);
        let weather_type = reader.read_u8().unwrap_or(WEATHER_FINE);
        let weather_amount = reader.read_u16().unwrap_or(0);

        tw.weather_type
            .store(weather_type, std::sync::atomic::Ordering::Relaxed);
        tw.weather_amount
            .store(weather_amount, std::sync::atomic::Ordering::Relaxed);

        // Broadcast updated weather to all
        let broadcast = build_weather_packet(weather_type, weather_amount);
        world.broadcast_to_all(Arc::new(broadcast), None);

        debug!(
            "[{}] GM set weather: type={}, amount={}",
            session.addr(),
            weather_type,
            weather_amount
        );
    } else {
        // WIZ_TIME — GM time update
        // We read but don't override real server time (the time packet
        // is built from chrono::Local and broadcast by the background task).
        debug!("[{}] GM WIZ_TIME received (acknowledged)", session.addr());
    }

    session.send_packet(&pkt).await?;

    Ok(())
}

/// Strip the first byte (ext sub-opcode) from a WIZ_EXT_HOOK packet,
/// returning a new packet with the remaining payload.
/// Used by ext_hook dispatch arms that delegate to handlers expecting
/// the sub-opcode byte already consumed.
fn repack_ext_data(pkt: &Packet) -> Packet {
    let mut repacked = Packet::new(pkt.opcode);
    if pkt.data.len() > 1 {
        repacked.data = pkt.data[1..].to_vec();
    }
    repacked
}

/// Handle WIZ_EXT_HOOK (0xE9) — dispatches to sub-handlers based on ext sub-opcodes.
/// First byte of data = ext sub-opcode, which routes to specific handlers.
async fn handle_ext_hook(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let sub_opcode = reader.read_u8().unwrap_or(0);

    tracing::info!(target: "hook_protocol", sid = session.session_id(),
        sub = sub_opcode, command = pkt.data.get(1).copied(), bytes = pkt.data.len(),
        "HOOK C2S dispatch");

    match sub_opcode {
        ext_hook::EXT_SUB_CINDERELLA => cinderella::handle(session, repack_ext_data(&pkt)).await,
        ext_hook::EXT_SUB_PERKS => perks::handle(session, repack_ext_data(&pkt)).await,
        // ExtSub::KCUPDATE = 0xB9 — client requests current KC/TL balance
        knight_cash::EXT_SUB_KCUPDATE => knight_cash::handle_kcupdate_query(session).await,
        // ExtSub::LOTTERY = 0xC7 — lottery event join/state
        lottery::EXT_SUB_LOTTERY => lottery::handle(session, repack_ext_data(&pkt)).await,
        // ExtSub::CR = 0xAF — Collection Race (server→client only, silently ignore)
        collection_race::EXT_SUB_COLLECTION_RACE => {
            debug!(
                "[{}] CR packet from client (server→client only, ignoring sub=0x{:02X})",
                session.addr(),
                pkt.data.first().copied().unwrap_or(0)
            );
            Ok(())
        }
        // ExtSub::TagInfo = 0xD1 — Tag name change
        tag_change::EXT_SUB_TAG_INFO => tag_change::handle(session, repack_ext_data(&pkt)).await,
        // ExtSub::PusRefund = 0xD4 — Cash shop item return
        pus_refund::EXT_SUB_PUS_REFUND => pus_refund::handle(session, repack_ext_data(&pkt)).await,
        // ExtSub::WheelData = 0xDA — Wheel of Fun spin
        wheel_of_fun::EXT_SUB_WHEEL_DATA => {
            wheel_of_fun::handle(session, repack_ext_data(&pkt)).await
        }
        // Extended hook sub-opcodes — anti-cheat + extended gameplay features
        ext_hook::EXT_SUB_AUTHINFO => ext_hook::handle_authinfo(session, &pkt.data[1..]),
        ext_hook::EXT_SUB_XALIVE => ext_hook::handle_xalive(session, &pkt.data[1..]),
        ext_hook::EXT_SUB_UIINFO => ext_hook::handle_ui_request(session).await,
        ext_hook::EXT_SUB_USERINFO => ext_hook::handle_req_userinfo(session).await,
        ext_hook::EXT_SUB_LOOT_SETTINGS => {
            ext_hook::handle_loot_settings(session, &pkt.data[1..]).await
        }
        ext_hook::EXT_SUB_SUPPORT => ext_hook::handle_support(session, &pkt.data[1..]),
        ext_hook::EXT_SUB_CHAT_LASTSEEN => ext_hook::handle_chat_lastseen(session, &pkt.data[1..]),
        ext_hook::EXT_SUB_SKILL_STAT_RESET => {
            ext_hook::handle_skill_stat_reset(session, &pkt.data[1..]).await
        }
        // Sprint 493: remaining ext_hook sub-opcodes
        ext_hook::EXT_SUB_PROCINFO => ext_hook::handle_procinfo(session, &pkt.data[1..]),
        ext_hook::EXT_SUB_LOG => ext_hook::handle_log(session, &pkt.data[1..]),
        ext_hook::EXT_SUB_PUS => ext_hook::handle_pus(session, &pkt.data[1..]).await,
        ext_hook::EXT_SUB_DROP_LIST | ext_hook::EXT_SUB_DROP_REQUEST => {
            ext_hook::handle_drop_request(session, &pkt.data[1..]).await
        }
        ext_hook::EXT_SUB_RESET => ext_hook::handle_reset(session, &pkt.data[1..]),
        ext_hook::EXT_SUB_CLANBANK => ext_hook::handle_clanbank(session, &pkt.data[1..]),
        ext_hook::EXT_SUB_CHAOTIC_EXCHANGE => {
            ext_hook::handle_chaotic_exchange(session, &pkt.data[1..]).await
        }
        ext_hook::EXT_SUB_MERCHANT => ext_hook::handle_merchant(session, &pkt.data[1..]).await,
        ext_hook::EXT_SUB_TEMPITEMS => ext_hook::handle_temp_items(session).await,
        ext_hook::EXT_SUB_MERCHANTLIST => {
            ext_hook::handle_merchantlist(session, &pkt.data[1..]).await
        }
        ext_hook::EXT_SUB_ACCOUNT_INFO_SAVE => {
            ext_hook::handle_account_info_save(session, &pkt.data[1..]).await
        }
        ext_hook::EXT_SUB_REPURCHASE => ext_hook::handle_repurchase(session, &pkt.data[1..]).await,
        ext_hook::EXT_SUB_CHEST_BLOCKITEM => {
            ext_hook::handle_chest_block(session, &pkt.data[1..]).await
        }
        ext_hook::EXT_SUB_ITEM_EXCHANGE_INFO => {
            ext_hook::handle_item_exchange_info(session, &pkt.data[1..]).await
        }
        ext_hook::EXT_SUB_DAILY_REWARD => {
            ext_hook::handle_daily_reward(session, &pkt.data[1..]).await
        }
        ext_hook::EXT_SUB_RESETREBSTAT => ext_hook::handle_resetrebstat(session),
        // Sprint 26: BANSYSTEM (0xBF) — life skill data query (C++ repurposed this sub-opcode)
        ext_hook::EXT_SUB_BANSYSTEM => ext_hook::handle_bansystem(session, &pkt.data[1..]).await,
        // Sprint 26: GAME_MASTER_MODE (0xE9) — GM mode UI toggle
        ext_hook::EXT_SUB_GAME_MASTER_MODE => {
            ext_hook::handle_game_master_mode(session, &pkt.data[1..]).await
        }
        _ => {
            tracing::warn!(
                "[{}] WIZ_EXT_HOOK unhandled sub-opcode: 0x{:02X}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Handle WIZ_WARP (0x1E) — GM intra-zone teleport.
/// Only GMs can use this. Client sends `[u16 PosX] [u16 PosZ]` and the
/// server warps the player within the same zone using the existing
/// `same_zone_warp()` flow.
async fn handle_recv_warp(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Only GMs can use WIZ_WARP — C++ checks isDead() || !isGM()
    let char_info = match world.get_character_info(sid) {
        Some(info) => info,
        None => return Ok(()),
    };

    if char_info.authority != 0 {
        return Ok(()); // Non-GM: silently ignore
    }

    if world.is_player_dead(sid) {
        return Ok(()); // Dead players can't warp
    }

    let mut reader = PacketReader::new(&pkt.data);
    let pos_x = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };
    let pos_z = match reader.read_u16() {
        Some(v) => v,
        None => return Ok(()),
    };

    // C++ Warp() expects raw tile coordinates; same_zone_warp expects float.
    // Client sends PosX/PosZ as tile coords (same unit as position * 10).
    let dest_x = pos_x as f32 / 10.0;
    let dest_z = pos_z as f32 / 10.0;

    debug!(
        "[{}] GM WIZ_WARP: ({}, {}) → ({:.1}, {:.1})",
        session.addr(),
        pos_x,
        pos_z,
        dest_x,
        dest_z
    );

    zone_change::same_zone_warp(session, dest_x, dest_z).await
}

#[cfg(test)]
mod tests {
    use super::is_cinderella_blocked_opcode;
    use ko_protocol::Opcode;

    // ── Sprint 46: Cinderella gating tests ──────────────────────────────

    #[test]
    fn test_cinderella_blocked_rental() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizRental));
    }

    #[test]
    fn test_cinderella_blocked_item_trade() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizItemTrade));
    }

    #[test]
    fn test_cinderella_blocked_bundle_open() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizBundleOpenReq));
    }

    #[test]
    fn test_cinderella_blocked_item_get() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizItemGet));
    }

    #[test]
    fn test_cinderella_blocked_exchange() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizExchange));
    }

    #[test]
    fn test_cinderella_blocked_merchant() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizMerchant));
    }

    #[test]
    fn test_cinderella_blocked_datasave() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizDatasave));
    }

    #[test]
    fn test_cinderella_blocked_warehouse() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizWarehouse));
    }

    #[test]
    fn test_cinderella_blocked_clan_warehouse() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizClanWarehouse));
    }

    #[test]
    fn test_cinderella_blocked_shopping_mall() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizShoppingMall));
    }

    #[test]
    fn test_cinderella_blocked_mining() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizMining));
    }

    #[test]
    fn test_cinderella_blocked_siege() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizSiege));
    }

    #[test]
    fn test_cinderella_blocked_nation_transfer() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizNationTransfer));
    }

    #[test]
    fn test_cinderella_blocked_gender_change() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizGenderChange));
    }

    #[test]
    fn test_cinderella_blocked_vip_warehouse() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizVipwarehouse));
    }

    #[test]
    fn test_cinderella_blocked_achieve() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizUserAchieve));
    }

    #[test]
    fn test_cinderella_blocked_pet() {
        assert!(is_cinderella_blocked_opcode(Opcode::WizPet));
    }

    #[test]
    fn test_cinderella_not_blocked_attack() {
        assert!(!is_cinderella_blocked_opcode(Opcode::WizAttack));
    }

    #[test]
    fn test_cinderella_not_blocked_move() {
        assert!(!is_cinderella_blocked_opcode(Opcode::WizMove));
    }

    #[test]
    fn test_cinderella_not_blocked_magic() {
        assert!(!is_cinderella_blocked_opcode(Opcode::WizMagicProcess));
    }

    #[test]
    fn test_cinderella_not_blocked_chat() {
        assert!(!is_cinderella_blocked_opcode(Opcode::WizChat));
    }

    #[test]
    fn test_cinderella_not_blocked_zone_change() {
        assert!(!is_cinderella_blocked_opcode(Opcode::WizZoneChange));
    }

    #[test]
    fn test_cinderella_not_blocked_dead() {
        assert!(!is_cinderella_blocked_opcode(Opcode::WizDead));
    }

    #[test]
    fn test_cinderella_not_blocked_regene() {
        assert!(!is_cinderella_blocked_opcode(Opcode::WizRegene));
    }

    // ── Sprint 58: WIZ_WARP packet parsing tests ───────────────────────

    #[test]
    fn test_wiz_warp_packet_parsing() {
        use ko_protocol::{Packet, PacketReader};

        // Build a WIZ_WARP packet with u16 x=6160, u16 z=3410
        let mut pkt = Packet::new(Opcode::WizWarp as u8);
        pkt.write_u16(6160);
        pkt.write_u16(3410);

        // Parse it back
        let mut reader = PacketReader::new(&pkt.data);
        let pos_x = reader.read_u16().unwrap();
        let pos_z = reader.read_u16().unwrap();

        // Verify raw values
        assert_eq!(pos_x, 6160);
        assert_eq!(pos_z, 3410);

        // Verify conversion to float coordinates (after /10.0)
        let dest_x = pos_x as f32 / 10.0;
        let dest_z = pos_z as f32 / 10.0;
        assert_eq!(dest_x, 616.0);
        assert_eq!(dest_z, 341.0);
    }
}
