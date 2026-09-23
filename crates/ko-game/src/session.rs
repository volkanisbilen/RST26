//! Per-client TCP session with packet framing and encryption.
//! Each TCP connection gets a `ClientSession` that manages:
//! - Reading inbound packets (header detection, length, payload, footer)
//! - Writing outbound packets (framing + encryption)
//! - AES-128-CBC encryption (key sent in 0x2B, replaces JvCryption)
//! - Login flow state machine
//! ## Two-Phase Architecture
//! **Pre-auth** (Connected → CharacterSelected): Session owns the full TcpStream
//! and writes packets directly. This is the original behavior.
//! **In-game** (after GameStart phase 2): The TcpStream is split. A writer task
//! handles outbound packets via a channel. Broadcasting to other sessions is
//! enabled through the shared WorldState.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use std::net::SocketAddr;

use tokio::net::tcp::OwnedReadHalf;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{info, warn};

use ko_db::DbPool;
use ko_protocol::{AesCryption, JvCryption, Packet};

use crate::handler;
use crate::packet_io;
use crate::world::WorldState;
use crate::zone::SessionId;

/// Loading timeout: 10 minutes of inactivity before game entry.
/// Sessions that have connected (or even logged in) but haven't completed the
/// game entry flow within this period are disconnected.
const LOADING_TIMEOUT_SECS: u64 = 10 * 60;

/// Session states for the login flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Just connected, no packets exchanged yet.
    Connected,
    /// Version check completed (AES key sent to client).
    VersionChecked,
    /// Account authenticated.
    LoggedIn,
    /// Nation selected.
    NationSelected,
    /// Character selected, entering game.
    CharacterSelected,
    /// In-game (after WIZ_GAMESTART phase 2).
    InGame,
}

/// Per-client session wrapping a TCP stream and encryption state.
/// Handles the full lifecycle of a client connection:
/// reading packets, dispatching to handlers, and sending responses.
pub struct ClientSession {
    // --- Pre-auth: direct stream access ---
    /// Full TCP stream (Some during pre-auth, None after upgrade).
    stream: Option<TcpStream>,
    /// Read half (Some after upgrade, None during pre-auth).
    read_half: Option<OwnedReadHalf>,

    addr: SocketAddr,

    // --- Crypto: owned during pre-auth, shared after upgrade ---
    /// Owned crypto (Some during pre-auth for key exchange).
    crypto_owned: Option<JvCryption>,
    /// Shared crypto (Some after upgrade, shared with writer task).
    crypto_shared: Option<Arc<JvCryption>>,

    state: SessionState,
    account_id: Option<String>,
    character_id: Option<String>,

    /// Shared sequence counter — incremented on inbound, used on outbound.
    sequence: Arc<AtomicU32>,

    pool: DbPool,

    // --- World state (always present) ---
    /// Unique session ID.
    session_id: SessionId,
    /// Shared world state for broadcasting.
    world: Arc<WorldState>,

    // --- AES-128-CBC encryption (replaces JvCryption for game server) ---
    /// AES encryption state (set during version check, shared with writer after upgrade).
    aes_owned: Option<AesCryption>,
    /// Shared AES state (Some after upgrade to in-game, shared with writer task).
    aes_shared: Option<Arc<AesCryption>>,

    // --- In-game: channel-based sending ---
    /// Outbound packet channel (Some after upgrade).
    tx: Option<mpsc::UnboundedSender<Arc<Packet>>>,
}

impl ClientSession {
    /// Create a new session for an accepted TCP connection.
    pub fn new(
        stream: TcpStream,
        addr: SocketAddr,
        pool: DbPool,
        session_id: SessionId,
        world: Arc<WorldState>,
    ) -> Self {
        // Disable Nagle's algorithm — game servers need low-latency packet delivery.
        let _ = stream.set_nodelay(true);
        Self {
            stream: Some(stream),
            read_half: None,
            addr,
            crypto_owned: Some(JvCryption::new()),
            crypto_shared: None,
            state: SessionState::Connected,
            account_id: None,
            character_id: None,
            sequence: Arc::new(AtomicU32::new(0)),
            pool,
            session_id,
            world,
            aes_owned: Some(AesCryption::new()),
            aes_shared: None,
            tx: None,
        }
    }

    /// Main session loop: read packets and dispatch to handlers.
    ///
    /// During the loading phase (before `InGame`), each `read_packet` call is
    /// wrapped with a 10-minute timeout matching `KOSOCKET_LOADING_TIMEOUT`.
    /// If the client does not send any packet within 10 minutes, the session is
    /// disconnected.
    ///
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("[{}] Client connected (sid={})", self.addr, self.session_id);

        loop {
            // During loading (before InGame), timeout after 10 minutes of no packets.
            let packet = if self.state != SessionState::InGame {
                match tokio::time::timeout(
                    Duration::from_secs(LOADING_TIMEOUT_SECS),
                    self.read_packet(),
                )
                .await
                {
                    Ok(Ok(pkt)) => pkt,
                    Ok(Err(e)) => {
                        info!("[{}] Connection closed: {}", self.addr, e);
                        break;
                    }
                    Err(_) => {
                        warn!(
                            "[{}] Loading timeout ({}s) — disconnecting (sid={})",
                            self.addr, LOADING_TIMEOUT_SECS, self.session_id
                        );
                        break;
                    }
                }
            } else {
                match self.read_packet().await {
                    Ok(pkt) => pkt,
                    Err(e) => {
                        info!("[{}] Connection closed: {}", self.addr, e);
                        break;
                    }
                }
            };

            // Raw packet logging for debugging v2600 connection flow
            if self.state != SessionState::InGame {
                let hex: String = packet
                    .data
                    .iter()
                    .take(32)
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                info!(
                    "[{}] C2S opcode=0x{:02X} len={} state={:?} data=[{}]",
                    self.addr,
                    packet.opcode,
                    packet.data.len(),
                    self.state,
                    hex
                );
            }

            if let Err(e) = handler::dispatch(self, packet).await {
                warn!("[{}] Handler error: {}", self.addr, e);
                break;
            }
        }

        info!(
            "[{}] Client disconnected (sid={})",
            self.addr, self.session_id
        );
        Ok(())
    }

    /// Read a complete packet from the TCP stream.
    ///
    /// AES path: If AES is enabled, reads the raw frame and delegates to
    /// `packet_io::read_packet_aes()` for flag-byte detection and decryption.
    /// JvCryption path: Legacy fallback (login server still uses it).
    async fn read_packet(&mut self) -> anyhow::Result<Packet> {
        if let Some(ref mut read_half) = self.read_half {
            // In-game mode: read from split read half.
            let aes = self.aes_shared.as_ref();
            if aes.is_some_and(|a| a.is_enabled()) {
                let aes = aes.unwrap();
                return packet_io::read_packet_aes(read_half, aes).await;
            }
            let crypto = self
                .crypto_shared
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("crypto_shared not initialized in read_packet"))?;
            let seq_val = self.sequence.load(Ordering::Acquire);
            let mut seq = seq_val;
            let result = packet_io::read_packet_from(read_half, crypto.as_ref(), &mut seq).await;
            self.sequence.store(seq, Ordering::Release);
            result
        } else if let Some(ref mut stream) = self.stream {
            // Pre-auth mode: check AES first, then JvCryption.
            let aes_enabled = self.aes_owned.as_ref().is_some_and(|a| a.is_enabled());
            if aes_enabled {
                let aes = self.aes_owned.as_ref().unwrap();
                return packet_io::read_packet_aes(stream, aes).await;
            }
            let crypto = self
                .crypto_owned
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("crypto_owned not initialized in read_packet"))?;
            let seq_val = self.sequence.load(Ordering::Acquire);
            let mut seq = seq_val;
            let result = packet_io::read_packet(stream, crypto, &mut seq).await;
            self.sequence.store(seq, Ordering::Release);
            result
        } else {
            anyhow::bail!("no stream available")
        }
    }

    /// Send a packet to the client.
    ///
    /// In-game: pushes to the writer channel (writer task handles AES/JvCryption).
    /// Pre-auth: writes directly to the TCP stream.
    ///
    /// AES path: If AES is enabled, encrypts with AES-128-CBC.
    /// JvCryption path: Legacy fallback (login server).
    pub async fn send_packet(&mut self, packet: &Packet) -> anyhow::Result<()> {
        // S2C logging for ALL phases (Phase 1 + Phase 2)
        {
            let hex: String = packet
                .data
                .iter()
                .take(32)
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            let mode = if self.tx.is_some() {
                "channel"
            } else {
                "direct"
            };
            tracing::info!(
                "[{}] S2C opcode=0x{:02X} len={} state={:?} mode={} plaintext={} data=[{}]",
                self.addr,
                packet.opcode,
                packet.data.len(),
                self.state,
                mode,
                packet.plaintext,
                hex
            );
        }

        if let Some(ref tx) = self.tx {
            // In-game: send via channel to writer task
            tx.send(Arc::new(packet.clone())).map_err(|e| {
                tracing::warn!(
                    "[{}] Writer channel CLOSED — opcode=0x{:02X} lost (sid={})",
                    self.addr,
                    e.0.opcode,
                    self.session_id,
                );
                anyhow::anyhow!("writer channel closed")
            })?;
            Ok(())
        } else if let Some(ref mut stream) = self.stream {
            // Pre-auth: direct write.
            // Check AES first, then fall back to JvCryption.
            let aes_enabled = self.aes_owned.as_ref().is_some_and(|a| a.is_enabled());
            if aes_enabled {
                let aes = self.aes_owned.as_ref().unwrap();
                return packet_io::send_packet_aes(stream, aes, packet).await;
            }
            let crypto = self
                .crypto_owned
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("crypto_owned not initialized in send_packet"))?;
            let seq = self.sequence.load(Ordering::Acquire);
            packet_io::send_packet(stream, crypto, seq, packet).await
        } else {
            anyhow::bail!("no stream or channel available for sending")
        }
    }

    /// Upgrade session from pre-auth to in-game mode.
    ///
    /// Splits the TcpStream, wraps crypto/AES in Arc, spawns writer task,
    /// and registers in the WorldState.
    pub async fn upgrade_to_ingame(&mut self) -> anyhow::Result<()> {
        let stream = self
            .stream
            .take()
            .ok_or_else(|| anyhow::anyhow!("stream already taken"))?;
        let crypto = self
            .crypto_owned
            .take()
            .ok_or_else(|| anyhow::anyhow!("crypto already taken"))?;
        let aes = self.aes_owned.take().unwrap_or_default();

        // Split stream
        let (read_half, write_half) = stream.into_split();
        self.read_half = Some(read_half);

        // Share crypto and AES with writer task
        let crypto_arc = Arc::new(crypto);
        self.crypto_shared = Some(crypto_arc.clone());
        let aes_arc = Arc::new(aes);
        self.aes_shared = Some(aes_arc.clone());

        // Create channel for outbound packets
        let (tx, rx) = mpsc::unbounded_channel();
        self.tx = Some(tx.clone());

        // Spawn writer task
        let seq = self.sequence.clone();
        tokio::spawn(crate::writer::writer_loop(
            write_half, rx, crypto_arc, seq, aes_arc,
        ));

        // Register in world state
        self.world.register_session(self.session_id, tx);

        info!(
            "[{}] Session upgraded to in-game mode (sid={})",
            self.addr, self.session_id
        );
        Ok(())
    }

    /// Cleanup on disconnect — save character data, clean up world state, mark offline.
    ///
    ///
    /// This mirrors the logout handler's save logic so that abrupt disconnects
    /// (crashes, network drops) don't lose character data.
    pub async fn cleanup(&mut self) {
        if self.state == SessionState::InGame {
            let sid = self.session_id;

            // ── 0. Offline merchant activation ─────────────────────────────
            // disconnects while merchant is open, the session stays in memory
            // and other players can still buy from the shop.
            //
            // Conditions:
            //   1. Player is in an active merchant state (selling or buying)
            //   2. Not already offline
            //   3. CFAIRY slot contains a valid offline merchant item
            if self.world.is_merchanting(sid) && !self.world.is_offline_status(sid) {
                use crate::world::OfflineCharacterType;
                if self
                    .world
                    .activate_offline_status(sid, OfflineCharacterType::Merchant)
                {
                    tracing::info!("[{}] Offline merchant activated (sid={})", self.addr, sid);
                    // Save character data to DB before going offline so that
                    // if the server restarts, the latest state is persisted.
                    if let Some(pool) = self.world.db_pool() {
                        crate::systems::character_save::save_single_character_sync(
                            &self.world,
                            pool,
                            sid,
                            "Offline merchant activation save",
                        )
                        .await;
                    }
                    // SKIP the rest of cleanup — session stays in memory.
                    return;
                }
            }

            let char_id = self.character_id.clone().unwrap_or_default();
            let account_id = self.account_id.clone().unwrap_or_default();

            // FerihaLog: DisconnectInsertLog (abrupt disconnect / network drop)
            if let Some(pool) = self.world.db_pool() {
                crate::handler::audit_log::log_disconnect(
                    pool,
                    &account_id,
                    &char_id,
                    &self.addr.to_string(),
                    "cleanup",
                    "abrupt_disconnect",
                    1,
                );
            }

            // ── 1. Save character stats + position ────────────────────────
            if !char_id.is_empty() {
                if let Some(ch) = self.world.get_character_info(sid) {
                    let pool = self.pool.clone();
                    let cid = char_id.clone();
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
                        if let Err(e) = repo
                            .save_stats(&ko_db::repositories::character::SaveStatsParams {
                                char_id: &cid,
                                level: ch.level as i16,
                                hp: ch.hp,
                                mp: ch.mp,
                                sp: ch.sp,
                                exp: ch.exp as i64,
                                gold: ch.gold as i32,
                                loyalty: ch.loyalty as i32,
                                loyalty_monthly: ch.loyalty_monthly as i32,
                                manner_point: ch.manner_point,
                            })
                            .await
                        {
                            warn!("Disconnect: failed to save stats for {}: {}", cid, e);
                        }
                    });
                }
                if let Some(pos) = self.world.get_position(sid) {
                    let pool = self.pool.clone();
                    let cid = char_id.clone();
                    let px = (pos.x * 100.0) as i32;
                    let pz = (pos.z * 100.0) as i32;
                    let zone_id = pos.zone_id as i16;
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
                        if let Err(e) = repo.save_position(&cid, zone_id, px, 0, pz).await {
                            warn!("Disconnect: failed to save position for {}: {}", cid, e);
                        }
                    });
                }
            }

            // ── 1a. Save class/race (safety net for Lua PromoteUser*) ────────
            if !char_id.is_empty() {
                if let Some(ch) = self.world.get_character_info(sid) {
                    let pool = self.pool.clone();
                    let cid = char_id.clone();
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
                        if let Err(e) = repo
                            .save_class_change(&cid, ch.class as i16, ch.race as i16)
                            .await
                        {
                            warn!("Disconnect: failed to save class/race for {}: {}", cid, e);
                        }
                    });
                }
            }

            // ── 1a2. Save flash time/count/type ─────────────────────────────
            if !char_id.is_empty() {
                let flash_data = self
                    .world
                    .with_session(sid, |h| (h.flash_time, h.flash_count, h.flash_type));
                if let Some((ft, fc, ftype)) = flash_data {
                    if ft > 0 || fc > 0 {
                        let pool = self.pool.clone();
                        let cid = char_id.clone();
                        tokio::spawn(async move {
                            let repo =
                                ko_db::repositories::character::CharacterRepository::new(&pool);
                            if let Err(e) = repo
                                .save_flash(&cid, ft as i32, fc as i16, ftype as i16)
                                .await
                            {
                                warn!("Disconnect: failed to save flash for {}: {}", cid, e);
                            }
                        });
                    }
                }
            }

            // ── 1a3. Save stat + skill points ──────────────────────────────
            if !char_id.is_empty() {
                if let Some(ch) = self.world.get_character_info(sid) {
                    let pool = self.pool.clone();
                    let cid = char_id.clone();
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
                        if let Err(e) = repo
                            .save_stat_points(
                                &ko_db::repositories::character::SaveStatPointsParams {
                                    char_id: &cid,
                                    str_val: ch.str as i16,
                                    sta: ch.sta as i16,
                                    dex: ch.dex as i16,
                                    intel: ch.intel as i16,
                                    cha: ch.cha as i16,
                                    free_points: ch.free_points as i16,
                                    skill_points: [
                                        ch.skill_points[0] as i16,
                                        ch.skill_points[1] as i16,
                                        ch.skill_points[2] as i16,
                                        ch.skill_points[3] as i16,
                                        ch.skill_points[4] as i16,
                                        ch.skill_points[5] as i16,
                                        ch.skill_points[6] as i16,
                                        ch.skill_points[7] as i16,
                                        ch.skill_points[8] as i16,
                                        ch.skill_points[9] as i16,
                                    ],
                                },
                            )
                            .await
                        {
                            warn!("Disconnect: failed to save stat points for {}: {}", cid, e);
                        }
                    });
                }
            }

            // ── 1b. Bulk save inventory items ──────────────────────────────
            if !char_id.is_empty() {
                let inventory = self.world.get_persistent_inventory(sid);
                let non_empty: Vec<(usize, u32)> = inventory
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.item_id != 0)
                    .map(|(s, i)| (s, i.item_id))
                    .collect();
                tracing::info!(
                    "Disconnect save: char={} inventory_len={} non_empty={}  slots={:?}",
                    char_id,
                    inventory.len(),
                    non_empty.len(),
                    non_empty
                        .iter()
                        .map(|(s, id)| format!("[{}]={}", s, id))
                        .collect::<Vec<_>>()
                );
                if !inventory.is_empty() {
                    let pool = self.pool.clone();
                    let cid = char_id.clone();
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::character::CharacterRepository::new(&pool);
                        let params: Vec<ko_db::repositories::character::SaveItemParams> = inventory
                            .iter()
                            .enumerate()
                            .map(
                                |(slot, item)| ko_db::repositories::character::SaveItemParams {
                                    char_id: &cid,
                                    slot_index: slot as i16,
                                    item_id: item.item_id as i32,
                                    durability: item.durability,
                                    count: item.count as i16,
                                    flag: item.flag as i16,
                                    original_flag: item.original_flag as i16,
                                    serial_num: item.serial_num as i64,
                                    expire_time: item.expire_time as i32,
                                },
                            )
                            .collect();
                        match repo.save_items_batch(&params).await {
                            Ok(()) => tracing::info!(
                                "Disconnect save OK: char={} slots={}",
                                cid,
                                params.len()
                            ),
                            Err(e) => {
                                warn!("Disconnect: failed to save inventory for {}: {}", cid, e)
                            }
                        }
                    });
                } else {
                    tracing::warn!("Disconnect save SKIPPED: char={} inventory empty!", char_id);
                }
            }

            // ── 1c. Bulk save warehouse items ─────────────────────────────
            if !account_id.is_empty() {
                let wh_data = self.world.with_session(sid, |h| {
                    (h.warehouse.clone(), h.inn_coins, h.warehouse_loaded)
                });
                if let Some((warehouse, inn_coins, loaded)) = wh_data {
                    if loaded && !warehouse.is_empty() {
                        let pool = self.pool.clone();
                        let acct = account_id.clone();
                        tokio::spawn(async move {
                            let repo =
                                ko_db::repositories::character::CharacterRepository::new(&pool);
                            let wh_params: Vec<
                                ko_db::repositories::character::SaveWarehouseItemParams,
                            > = warehouse
                                .iter()
                                .enumerate()
                                .map(|(slot, item)| {
                                    ko_db::repositories::character::SaveWarehouseItemParams {
                                        account_id: &acct,
                                        slot_index: slot as i16,
                                        item_id: item.item_id as i32,
                                        durability: item.durability,
                                        count: item.count as i16,
                                        flag: item.flag as i16,
                                        original_flag: item.original_flag as i16,
                                        serial_num: item.serial_num as i64,
                                        expire_time: item.expire_time as i32,
                                    }
                                })
                                .collect();
                            if let Err(e) = repo.save_warehouse_items_batch(&wh_params).await {
                                warn!("Disconnect: failed to save warehouse for {}: {}", acct, e);
                            }
                            if let Err(e) = repo.save_warehouse_coins(&acct, inn_coins as i32).await
                            {
                                warn!(
                                    "Disconnect: failed to save warehouse coins for {}: {}",
                                    acct, e
                                );
                            }
                        });
                    }
                }
            }

            // ── 2. Save active buffs (saved magic) ───────────────────────
            self.save_saved_magic().await;

            // ── 3. Save premium state ─────────────────────────────────────
            if !account_id.is_empty() {
                let premium_slots: Vec<(i16, i16, i32)> = self
                    .world
                    .with_session(sid, |h| {
                        h.premium_map
                            .iter()
                            .enumerate()
                            .map(|(idx, (&p_type, &expiry))| {
                                (idx as i16, p_type as i16, expiry as i32)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if !premium_slots.is_empty() {
                    let pool = self.pool.clone();
                    let acct = account_id.clone();
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::premium::PremiumRepository::new(&pool);
                        if let Err(e) = repo.save_account_premium(&acct, &premium_slots).await {
                            warn!("Disconnect: failed to save premium for {}: {}", acct, e);
                        }
                    });
                }
            }

            // ── 4. Save achievement data ──────────────────────────────────
            if !char_id.is_empty() {
                // Update play_time before saving.
                self.world.update_session(sid, |h| {
                    if h.achieve_login_time > 0 {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as u32;
                        if now > h.achieve_login_time {
                            h.achieve_summary.play_time += now - h.achieve_login_time;
                        }
                        h.achieve_login_time = now;
                    }
                });
                let achieve_data = self.world.with_session(sid, |h| {
                    let entries: Vec<(u16, u8, u32, u32)> = h
                        .achieve_map
                        .iter()
                        .map(|(&id, info)| (id, info.status, info.count[0], info.count[1]))
                        .collect();
                    let summary = h.achieve_summary.clone();
                    (entries, summary)
                });
                if let Some((entries, summary)) = achieve_data {
                    if !entries.is_empty() || summary.play_time > 0 {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        let batch_entries: Vec<(i32, i16, i32, i32)> = entries
                            .iter()
                            .map(|&(id, status, count1, count2)| {
                                (id as i32, status as i16, count1 as i32, count2 as i32)
                            })
                            .collect();
                        tokio::spawn(async move {
                            let repo = ko_db::repositories::achieve::AchieveRepository::new(&pool);
                            if !batch_entries.is_empty() {
                                if let Err(e) =
                                    repo.save_user_achieves_batch(&name, &batch_entries).await
                                {
                                    warn!(
                                        "Disconnect: failed to save achievements for {}: {}",
                                        name, e
                                    );
                                }
                            }
                            if let Err(e) = repo
                                .save_user_achieve_summary(
                                    &name,
                                    summary.play_time as i32,
                                    summary.monster_defeat_count as i32,
                                    summary.user_defeat_count as i32,
                                    summary.user_death_count as i32,
                                    summary.total_medal as i32,
                                    [
                                        summary.recent_achieve[0] as i16,
                                        summary.recent_achieve[1] as i16,
                                        summary.recent_achieve[2] as i16,
                                    ],
                                    summary.cover_id as i16,
                                    summary.skill_id as i16,
                                )
                                .await
                            {
                                warn!(
                                    "Disconnect: failed to save achieve summary for {}: {}",
                                    name, e
                                );
                            }
                        });
                    }
                }
            }

            // ── 5. Save user perks ────────────────────────────────────────
            if !char_id.is_empty() {
                let perk_data = self
                    .world
                    .with_session(sid, |h| (h.perk_levels, h.rem_perk));
                if let Some((perk_levels, rem_perk)) = perk_data {
                    if perk_levels.iter().any(|&v| v != 0) || rem_perk != 0 {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        tokio::spawn(async move {
                            let repo = ko_db::repositories::perk::PerkRepository::new(&pool);
                            if let Err(e) =
                                repo.save_user_perks(&name, &perk_levels, rem_perk).await
                            {
                                warn!("Disconnect: failed to save perks for {}: {}", name, e);
                            }
                        });
                    }
                }
            }

            // ── 5b. Save soul data (v2525) ──────────────────────────────────
            if !char_id.is_empty() {
                let soul_data = self
                    .world
                    .with_session(sid, |h| {
                        if h.soul_loaded {
                            Some((h.soul_categories, h.soul_slots))
                        } else {
                            None
                        }
                    })
                    .flatten();
                if let Some((cats, slots)) = soul_data {
                    let has_data = cats.iter().any(|c| c[1] != 0 || c[2] != 0 || c[3] != 0)
                        || slots.iter().any(|s| s[1] != 0);
                    if has_data {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        tokio::spawn(async move {
                            let repo = ko_db::repositories::soul::SoulRepository::new(&pool);
                            if let Err(e) = repo.save(&name, &cats, &slots).await {
                                warn!("Disconnect: failed to save soul for {}: {}", name, e);
                            }
                        });
                    }
                }
            }

            // ── 5c. Save hermetic seal data (v2525) ───────────────────────────
            if !char_id.is_empty() {
                let seal_data = self
                    .world
                    .with_session(sid, |h| {
                        if h.seal_loaded {
                            Some((
                                h.seal_max_tier,
                                h.seal_selected_slot,
                                h.seal_status,
                                h.seal_upgrade_count,
                                h.seal_current_level,
                                h.seal_elapsed_time,
                            ))
                        } else {
                            None
                        }
                    })
                    .flatten();
                if let Some((
                    max_tier,
                    selected_slot,
                    status,
                    upgrade_count,
                    current_level,
                    elapsed_time,
                )) = seal_data
                {
                    let has_data = max_tier > 0
                        || selected_slot > 0
                        || current_level > 0
                        || upgrade_count > 0
                        || elapsed_time > 0.0;
                    if has_data {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        tokio::spawn(async move {
                            let repo =
                                ko_db::repositories::hermetic_seal::HermeticSealRepository::new(
                                    &pool,
                                );
                            if let Err(e) = repo
                                .save(
                                    &name,
                                    max_tier as i16,
                                    selected_slot as i16,
                                    status as i16,
                                    upgrade_count as i16,
                                    current_level as i16,
                                    elapsed_time as f32,
                                )
                                .await
                            {
                                warn!(
                                    "Disconnect: failed to save hermetic seal for {}: {}",
                                    name, e
                                );
                            }
                        });
                    }
                }
            }

            // ── 5d. Save costume data (v2525) ────────────────────────────────
            if !char_id.is_empty() {
                let costume_data = self
                    .world
                    .with_session(sid, |h| {
                        if h.costume_loaded {
                            Some((
                                h.costume_active_type,
                                h.costume_item_id,
                                h.costume_item_param,
                                h.costume_scale_raw,
                                h.costume_color_index,
                                h.costume_expiry_time,
                            ))
                        } else {
                            None
                        }
                    })
                    .flatten();
                if let Some((
                    active_type,
                    item_id,
                    item_param,
                    scale_raw,
                    color_index,
                    expiry_time,
                )) = costume_data
                {
                    let has_data = active_type > 0 || item_id != 0;
                    if has_data {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        tokio::spawn(async move {
                            let repo = ko_db::repositories::costume::CostumeRepository::new(&pool);
                            if let Err(e) = repo
                                .save(
                                    &name,
                                    active_type as i16,
                                    item_id,
                                    item_param,
                                    scale_raw,
                                    color_index as i16,
                                    expiry_time,
                                )
                                .await
                            {
                                warn!("Disconnect: failed to save costume for {}: {}", name, e);
                            }
                        });
                    }
                }
            }

            // ── 5e. Save enchant data (v2525) ─────────────────────────────────
            if !char_id.is_empty() {
                let enchant_data = self
                    .world
                    .with_session(sid, |h| {
                        if h.enchant_loaded {
                            Some((
                                h.enchant_max_star,
                                h.enchant_count,
                                h.enchant_slot_levels,
                                h.enchant_slot_unlocked,
                                h.enchant_item_category,
                                h.enchant_item_slot_unlock,
                                h.enchant_item_markers,
                            ))
                        } else {
                            None
                        }
                    })
                    .flatten();
                if let Some((
                    max_star,
                    enc_count,
                    levels,
                    unlocked,
                    item_cat,
                    item_unlock,
                    markers,
                )) = enchant_data
                {
                    let has_data = max_star > 0
                        || enc_count > 0
                        || levels.iter().any(|&v| v > 0)
                        || unlocked.iter().any(|&v| v > 0);
                    if has_data {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        tokio::spawn(async move {
                            let repo = ko_db::repositories::enchant::EnchantRepository::new(&pool);
                            if let Err(e) = repo
                                .save(
                                    &name,
                                    max_star as i16,
                                    enc_count as i16,
                                    &levels,
                                    &unlocked,
                                    item_cat as i16,
                                    item_unlock as i16,
                                    &markers,
                                )
                                .await
                            {
                                warn!("Disconnect: failed to save enchant for {}: {}", name, e);
                            }
                        });
                    }
                }
            }

            // ── 6. Save quest progress ──────────────────────────────────────
            if !char_id.is_empty() {
                let quest_data = self.world.with_session(sid, |h| h.quests.clone());
                if let Some(quests) = quest_data {
                    if !quests.is_empty() {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        tokio::spawn(async move {
                            let repo = ko_db::repositories::quest::QuestRepository::new(&pool);
                            let entries: Vec<(i16, i16, [i16; 4])> = quests
                                .iter()
                                .map(|(&quest_id, info)| {
                                    (
                                        quest_id as i16,
                                        info.quest_state as i16,
                                        [
                                            info.kill_counts[0] as i16,
                                            info.kill_counts[1] as i16,
                                            info.kill_counts[2] as i16,
                                            info.kill_counts[3] as i16,
                                        ],
                                    )
                                })
                                .collect();
                            if let Err(e) = repo.save_user_quests_batch(&name, &entries).await {
                                warn!("Disconnect: failed to save quests for {}: {}", name, e);
                            }
                        });
                    }
                }
            }

            // ── 6b. Save daily quest progress ────────────────────────────────
            if !char_id.is_empty() {
                let dq_data = self.world.with_session(sid, |h| h.daily_quests.clone());
                if let Some(dq_map) = dq_data {
                    if !dq_map.is_empty() {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        let entries: Vec<_> = dq_map.into_values().collect();
                        tokio::spawn(async move {
                            let repo =
                                ko_db::repositories::daily_quest::DailyQuestRepository::new(&pool);
                            if let Err(e) = repo.save_all_user_quests(&name, &entries).await {
                                tracing::warn!(
                                    "Disconnect: daily quest save failed for {}: {}",
                                    name,
                                    e
                                );
                            }
                        });
                    }
                }
            }

            // ── 7. Save genie data ────────────────────────────────────────
            // Only save if genie data has been loaded from DB (prevents overwriting with 0).
            if !char_id.is_empty() {
                let genie_data = self.world.with_session(sid, |h| {
                    (h.genie_time_abs, h.genie_options.clone(), h.genie_loaded)
                });
                if let Some((genie_abs, genie_options, genie_loaded)) = genie_data {
                    if !genie_loaded {
                        tracing::warn!(
                            "Disconnect: skipping genie save for {} — not loaded from DB",
                            char_id
                        );
                    } else if genie_abs > 0 || !genie_options.is_empty() {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        tokio::spawn(async move {
                            let repo =
                                ko_db::repositories::user_data::UserDataRepository::new(&pool);
                            let db_val = crate::handler::genie::genie_abs_to_db(genie_abs);
                            if let Err(e) =
                                repo.save_genie_data(&name, db_val, &genie_options, 0).await
                            {
                                warn!("Disconnect: failed to save genie for {}: {}", name, e);
                            }
                        });
                    }
                }
            }

            // ── 7b. Save daily operation cooldowns ─────────────────────────
            if !char_id.is_empty() {
                if let Some((_, data)) = self.world.daily_ops.remove(&char_id) {
                    let pool = self.pool.clone();
                    let name = char_id.clone();
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::user_data::UserDataRepository::new(&pool);
                        let row = data.to_row(&name);
                        if let Err(e) = repo.save_daily_op(&row).await {
                            tracing::warn!(
                                "Disconnect: failed to save daily_op for {}: {}",
                                name,
                                e
                            );
                        }
                    });
                }
            }

            // ── 7c. Save daily rank raw stats ──────────────────────────────
            if !char_id.is_empty() {
                let dr_data = self.world.with_session(sid, |h| {
                    (
                        h.dr_gm_total_sold,
                        h.dr_mh_total_kill,
                        h.dr_sh_total_exchange,
                        h.dr_cw_counter_win,
                        h.dr_up_counter_bles,
                    )
                });
                if let Some((gm, mh, sh, cw, up)) = dr_data {
                    if gm > 0 || mh > 0 || sh > 0 || cw > 0 || up > 0 {
                        let pool = self.pool.clone();
                        let name = char_id.clone();
                        tokio::spawn(async move {
                            let repo =
                                ko_db::repositories::daily_rank::DailyRankRepository::new(&pool);
                            if let Err(e) = repo
                                .save_user_stats(
                                    &name, gm as i64, mh as i64, sh as i64, cw as i64, up as i64,
                                )
                                .await
                            {
                                warn!(
                                    "Disconnect: failed to save daily rank stats for {}: {}",
                                    name, e
                                );
                            }
                        });
                    }
                }
            }

            // ── 8. Party cleanup (before region removal) ──────────────────
            self.world.cleanup_party_on_disconnect(sid);

            // ── 9. Trade/exchange cleanup ──────────────────────────────────
            if self.world.is_trading(sid) {
                let partner_sid = self.world.get_exchange_user(sid);
                self.world.reset_trade(sid);
                if let Some(partner) = partner_sid {
                    self.world.reset_trade(partner);
                    let mut cancel_pkt = Packet::new(ko_protocol::Opcode::WizExchange as u8);
                    cancel_pkt.write_u8(0x08); // EXCHANGE_CANCEL
                    self.world.send_to_session_owned(partner, cancel_pkt);
                }
            }

            // ── 10. Merchant cleanup ──────────────────────────────────────
            if self.world.is_merchanting(sid) {
                self.world.close_merchant(sid);
            }

            // ── 10a. Pet save + cleanup ────────────────────────────────────
            // Save pet state to DB before clearing, then clear.
            {
                let pet_snapshot = self.world.with_session(sid, |h| {
                    h.pet_data.as_ref().filter(|p| p.serial_id > 0).map(|p| {
                        ko_db::models::pet::PetUserDataRow {
                            n_serial_id: p.serial_id as i64,
                            s_pet_name: p.name.clone(),
                            b_level: p.level as i16,
                            s_hp: p.hp as i16,
                            s_mp: p.mp as i16,
                            n_index: p.index as i32,
                            s_satisfaction: p.satisfaction,
                            n_exp: p.exp as i32,
                            s_pid: p.pid as i16,
                            s_size: p.size as i16,
                        }
                    })
                });
                if let Some(Some(pet_row)) = pet_snapshot {
                    let pool_pet = self.pool.clone();
                    tokio::spawn(async move {
                        let repo = ko_db::repositories::pet::PetRepository::new(&pool_pet);
                        if let Err(e) = repo.save_pet_data(&pet_row).await {
                            warn!(
                                "Disconnect: failed to save pet data (serial={}): {}",
                                pet_row.n_serial_id, e
                            );
                        }
                    });
                }
            }
            self.world.update_session(sid, |h| {
                h.pet_data = None;
            });

            // ── 10b. BDW cleanup ────────────────────────────────────────
            crate::handler::logout::bdw_user_logout(&self.world, sid);

            // ── 10c. Monster Stone cleanup ──────────────────────────────
            {
                let ms_active = self
                    .world
                    .with_session(sid, |h| h.event_room > 0)
                    .unwrap_or(false);
                if ms_active {
                    crate::handler::zone_change::monster_stone_exit_room(&self.world, sid);
                }
            }

            // ── 10d. Draki Tower cleanup ────────────────────────────────
            {
                let room_id = self
                    .world
                    .with_session(sid, |h| h.draki_room_id)
                    .unwrap_or(0);
                if room_id > 0 {
                    self.world.update_session(sid, |h| {
                        h.event_room = 0;
                        h.draki_room_id = 0;
                    });
                    self.world
                        .despawn_room_npcs(crate::handler::draki_tower::ZONE_DRAKI_TOWER, room_id);
                    let mut rooms = self.world.draki_tower_rooms_write();
                    if let Some(room) = rooms.get_mut(&room_id) {
                        room.reset();
                    }
                }
            }

            // ── 10e. Challenge/duel cancel ──────────────────────────────
            {
                let (requesting, requested, challenge_user) = self.world.get_challenge_state(sid);
                if requesting > 0 || requested > 0 {
                    let target = challenge_user as u16;
                    if challenge_user >= 0 {
                        self.world.update_session(target, |h| {
                            h.challenge_user = -1;
                            h.requesting_challenge = 0;
                            h.challenge_requested = 0;
                        });
                        let mut cancel_pkt = Packet::new(ko_protocol::Opcode::WizChallenge as u8);
                        cancel_pkt.write_u8(if requesting > 0 { 2 } else { 4 });
                        self.world.send_to_session_owned(target, cancel_pkt);
                    }
                    self.world.update_session(sid, |h| {
                        h.challenge_user = -1;
                        h.requesting_challenge = 0;
                        h.challenge_requested = 0;
                    });
                }
            }

            // ── 10f. Remove rival ───────────────────────────────────────
            {
                let has_rival = self
                    .world
                    .get_character_info(sid)
                    .map(|ch| ch.rival_id >= 0)
                    .unwrap_or(false);
                if has_rival {
                    self.world.remove_rival(sid);
                }
            }

            // ── 10g. Remove from merchant lookers ───────────────────────
            self.world.remove_from_merchant_lookers(sid);

            // ── 10h. Stop mining / fishing ──────────────────────────────
            crate::handler::mining::stop_mining_internal(&self.world, sid);
            crate::handler::mining::stop_fishing_internal(&self.world, sid);

            // ── 11. Party BBS cleanup ─────────────────────────────────────
            crate::handler::party_bbs::cleanup_on_disconnect(&self.world, sid);

            // ── 11a. Soccer event cleanup ─────────────────────────────────
            if let Some(ch) = self.world.get_character_info(sid) {
                if let Some(pos) = self.world.get_position(sid) {
                    let soccer_state = self.world.soccer_state().clone();
                    let mut state = soccer_state.write();
                    if let Some(room) = state.get_room_mut(pos.zone_id) {
                        crate::handler::soccer::remove_user(room, &ch.name);
                    }
                }
            }

            // ── 11b. Chat room cleanup ─────────────────────────────────────
            // Remove player from chat room on disconnect (admin → room deleted).
            let room_index = self.world.get_chat_room_index(sid);
            if room_index > 0 {
                if let Some(ch) = self.world.get_character_info(sid) {
                    let is_admin = self
                        .world
                        .get_chat_room(room_index)
                        .map(|r| r.is_administrator(&ch.name) == 2)
                        .unwrap_or(false);
                    if is_admin {
                        self.world.remove_chat_room(room_index);
                    } else if let Some(mut room) = self.world.get_chat_room_mut(room_index) {
                        room.remove_user(&ch.name);
                    }
                }
                self.world.set_chat_room_index(sid, 0);
            }

            // ── 11b. BottomUserLogOut — broadcast zone-wide logout notification
            if let Some(ch) = self.world.get_character_info(sid) {
                if let Some(pos) = self.world.get_position(sid) {
                    let region_del_pkt =
                        crate::handler::user_info::build_region_delete_packet(&ch.name);
                    self.world
                        .broadcast_to_zone(pos.zone_id, Arc::new(region_del_pkt), Some(sid));
                }
            }

            // ── 11c. GM list removal
            if let Some(ch) = self.world.get_character_info(sid) {
                if ch.authority == 0 {
                    self.world.gm_list_remove(&ch.name);
                }
            }

            // ── 11d. Knights/Clan cleanup — clan buff + offline notification
            if let Some(ch) = self.world.get_character_info(sid) {
                if ch.knights_id > 0 {
                    self.world
                        .knights_clan_buff_update(ch.knights_id, false, sid);
                    crate::handler::knights::send_clan_offline_notification(
                        &self.world,
                        ch.knights_id,
                        &ch.name,
                        sid,
                    );
                }
            }

            // ── 12. Zone/region removal + INOUT_OUT broadcast ─────────────
            if let Some((pos, event_room)) =
                self.world.with_session(sid, |h| (h.position, h.event_room))
            {
                if let Some(zone) = self.world.get_zone(pos.zone_id) {
                    zone.remove_user(pos.region_x, pos.region_z, sid);
                }

                let out_pkt = crate::handler::region::build_user_inout(
                    crate::handler::region::INOUT_OUT,
                    sid,
                    None,
                    &Default::default(),
                );
                self.world.broadcast_to_3x3(
                    pos.zone_id,
                    pos.region_x,
                    pos.region_z,
                    Arc::new(out_pkt),
                    Some(sid),
                    event_room,
                );
            }

            // ── 13. ALL ranking cleanup ──────────────────────────────────
            self.world.pk_zone_remove_player(sid);
            self.world.zindan_remove_player(sid);
            self.world.bdw_remove_player(sid);
            self.world.chaos_remove_player(sid);

            // ── 13b. Cinderella War cleanup ─────────────────────────────
            crate::handler::cinderella::cinderella_logout(&self.world, sid, true);

            // ── 13c. Wanted event cleanup ────────────────────────────────
            {
                let is_wanted = self
                    .world
                    .with_session(sid, |h| h.is_wanted)
                    .unwrap_or(false);
                if is_wanted {
                    crate::handler::vanguard::handle_wanted_logout(&self.world, sid);
                }
            }

            // ── 13d. Temple event sign-up cleanup ────────────────────────
            // Remove player from event sign-up queue if still in signing phase.
            {
                let (active_event, is_active) = self
                    .world
                    .event_room_manager
                    .read_temple_event(|s| (s.active_event, s.is_active));
                if active_event >= 0 && !is_active {
                    if let Some(name) = self.world.get_session_name(sid) {
                        if let Some(removed) =
                            self.world.event_room_manager.remove_signed_up_user(&name)
                        {
                            self.world.event_room_manager.update_temple_event(|s| {
                                if removed.nation == 1 {
                                    s.karus_user_count = s.karus_user_count.saturating_sub(1);
                                } else {
                                    s.elmorad_user_count = s.elmorad_user_count.saturating_sub(1);
                                }
                                s.all_user_count = s.karus_user_count + s.elmorad_user_count;
                            });
                            tracing::debug!(
                                "[{}] Disconnect: removed '{}' from event sign-up (event={})",
                                self.addr,
                                name,
                                active_event
                            );
                        }
                    }
                }
            }

            // ── 14. Mark account offline ──────────────────────────────────
            if !account_id.is_empty() {
                let pool = self.pool.clone();
                let acct = account_id;
                tokio::spawn(async move {
                    let repo = ko_db::repositories::account::AccountRepository::new(&pool);
                    if let Err(e) = repo.set_offline(&acct).await {
                        warn!("Disconnect: failed to set account offline: {}", e);
                    }
                });
            }
        }

        self.world.unregister_session(self.session_id);
    }

    // --- Public accessors ---

    /// Current session state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Set session state.
    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
    }

    /// Remote client address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Mutable reference to the encryption state (pre-auth only).
    pub fn crypto_mut(&mut self) -> &mut JvCryption {
        self.crypto_owned
            .as_mut()
            .expect("crypto_mut called after upgrade")
    }

    /// Mutable reference to AES encryption state (pre-auth only).
    pub fn aes_mut(&mut self) -> &mut AesCryption {
        self.aes_owned
            .as_mut()
            .expect("aes_mut called after upgrade")
    }

    /// Check if AES encryption is currently enabled for this session.
    pub fn aes_enabled(&self) -> bool {
        self.aes_owned.as_ref().is_some_and(|a| a.is_enabled())
    }

    /// Account ID (set after successful login).
    pub fn account_id(&self) -> Option<&str> {
        self.account_id.as_deref()
    }

    /// Set the authenticated account ID.
    pub fn set_account_id(&mut self, id: String) {
        self.account_id = Some(id);
    }

    /// Selected character ID (set after WIZ_SEL_CHAR).
    pub fn character_id(&self) -> Option<&str> {
        self.character_id.as_deref()
    }

    /// Set the selected character ID.
    pub fn set_character_id(&mut self, id: String) {
        self.character_id = Some(id);
    }

    /// Database connection pool.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Unique session ID.
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Shared world state.
    pub fn world(&self) -> &Arc<WorldState> {
        &self.world
    }

    /// Save active saved magic entries to DB and wait until the replacement
    /// is complete. Zone transitions must not race this delete/insert with a
    /// later logout save, or the character can lose its persistent scrolls.
    pub(crate) async fn save_saved_magic(&self) {
        let char_id = match self.character_id.as_deref() {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => return,
        };
        let entries = self.world.get_saved_magic_entries(self.session_id);
        let repo = ko_db::repositories::saved_magic::SavedMagicRepository::new(&self.pool);
        if let Err(e) = repo.save_saved_magic(&char_id, &entries).await {
            tracing::error!(char_id, "failed to save magic on zone change: {}", e);
        }
    }
}
