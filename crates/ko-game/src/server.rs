//! Game server — TCP listener and connection accept loop.
//! Binds to a TCP port and spawns a `ClientSession` task per connection.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tracing::{error, info, warn};

use ko_db::DbPool;

use crate::session::ClientSession;
use crate::world::WorldState;

/// Maximum concurrent sessions before rejecting new connections.
/// Prevents unbounded memory growth from excessive connections.
const MAX_SESSIONS: usize = 5000;

/// Server configuration loaded from environment or config file.
pub struct ServerConfig {
    /// Address to bind the TCP listener (e.g. `0.0.0.0:15001`).
    pub bind_addr: String,
    /// Path to the directory containing SMD map files.
    pub map_dir: PathBuf,
}

/// Game server — accepts TCP connections and spawns sessions.
pub struct GameServer {
    config: ServerConfig,
    pool: DbPool,
    world: Arc<WorldState>,
}

impl GameServer {
    /// Create a new game server — loads zone data from DB and SMD files.
    ///
    pub async fn new(config: ServerConfig, pool: DbPool) -> anyhow::Result<Self> {
        let world = match WorldState::load(&pool, &config.map_dir).await {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::warn!("failed to load world from DB: {}, using fallback", e);
                Arc::new(WorldState::new())
            }
        };
        Ok(Self {
            config,
            pool,
            world,
        })
    }

    /// Get the configured bind address (for logging in unified binary).
    pub fn bind_addr(&self) -> &str {
        &self.config.bind_addr
    }

    /// Run the server — listens for connections and spawns session tasks.
    pub async fn run(&self) -> anyhow::Result<()> {
        // Clear stale online entries from a previous crash.
        let repo = ko_db::repositories::account::AccountRepository::new(&self.pool);
        match repo.clear_all_online().await {
            Ok(n) if n > 0 => info!("Cleared {} stale online entries from previous run", n),
            Ok(_) => {}
            Err(e) => warn!("Failed to clear stale online entries: {}", e),
        }

        // Reset concurrent user count so login server shows 0 until first real update.
        let srv_repo = ko_db::repositories::server_list::ServerListRepository::new(&self.pool);
        if let Err(e) = srv_repo.update_concurrent_users(1, 0).await {
            warn!("Failed to reset concurrent user count: {}", e);
        }

        // Native v2615 panels are always available after a server restart.
        // GM open/close commands remain effective for the current runtime.
        let native_repo =
            ko_db::repositories::native_events::NativeEventsRepository::new(&self.pool);
        match native_repo.activate_all().await {
            Ok(count) => info!("Native client events activated at startup: {} rows", count),
            Err(e) => warn!("Failed to activate native client events at startup: {}", e),
        }

        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("Listening on {}", self.config.bind_addr);

        // Initialize wanted event rooms (3 PK zone rooms).
        crate::handler::vanguard::initialize_wanted_rooms(&self.world);

        // Spawn random boss monsters at startup.
        self.world.random_boss_system_load();

        // Chaos Stone table loading only creates event state. Materialise the
        // enabled stones now so Ronark Land/Ardream contain their rank-1 NPCs.
        let chaos_stone_count =
            crate::systems::chaos_stone_tick::spawn_initial_chaos_stones(&self.world);
        info!("Initial Chaos Stones spawned: {}", chaos_stone_count);

        // Start background tick systems — collect handles for clean shutdown.
        let mut bg_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

        let pus_bind =
            std::env::var("PUS_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string());
        bg_tasks.push(crate::pus_web::start(
            self.world.clone(),
            self.pool.clone(),
            pus_bind,
        ));
        info!("Power Up Store web service started");

        bg_tasks.push(crate::systems::regen::start_regen_task(self.world.clone()));
        info!("HP/MP regen tick started");

        bg_tasks.push(crate::systems::buff_tick::start_buff_tick_task(
            self.world.clone(),
        ));
        info!("Buff expiry tick started");

        bg_tasks.push(crate::systems::dot_tick::start_dot_tick_task(
            self.world.clone(),
        ));
        info!("DOT/HOT tick started");

        bg_tasks.push(crate::systems::sp_regen::start_sp_regen_task(
            self.world.clone(),
        ));
        info!("Kurian SP regen tick started (2s interval)");

        bg_tasks.push(crate::systems::npc_ai::start_npc_ai_task(
            self.world.clone(),
        ));
        info!("NPC AI tick started");

        let time_weather = self.world.game_time_weather().clone();
        bg_tasks.push(crate::systems::time_weather::start_time_broadcast_task(
            self.world.clone(),
        ));
        info!("Game time broadcast started");

        bg_tasks.push(crate::systems::time_weather::start_weather_task(
            self.world.clone(),
            time_weather,
        ));
        info!("Weather cycle started");

        bg_tasks.push(crate::systems::war::start_war_task(self.world.clone()));
        info!("War tick started");

        bg_tasks.push(crate::systems::event_system::start_event_system_task(
            self.world.clone(),
        ));
        info!("Event system tick started");

        bg_tasks.push(crate::systems::expiry_tick::start_expiry_tick_task(
            self.world.clone(),
        ));
        info!("Premium/item expiry tick started (10s interval)");

        bg_tasks.push(crate::systems::character_save::start_character_save_task(
            self.world.clone(),
            self.pool.clone(),
        ));
        info!("Periodic character save started (10min interval)");

        bg_tasks.push(crate::systems::knights_save::start_knights_save_task(
            self.world.clone(),
            self.pool.clone(),
        ));
        info!("Periodic knights save started (5min interval)");

        bg_tasks.push(crate::systems::timed_notice::start_timed_notice_task(
            self.world.clone(),
            self.pool.clone(),
        ));
        info!("Timed notice system started");

        bg_tasks.push(
            crate::systems::letter_admin_notify::start_letter_admin_notify_task(
                self.world.clone(),
                self.pool.clone(),
            ),
        );
        info!("Letter admin realtime notification started (500ms interval)");

        bg_tasks.push(crate::systems::pet_tick::start_pet_tick_task(
            self.world.clone(),
        ));
        info!("Pet satisfaction decay tick started (15s interval)");

        bg_tasks.push(crate::systems::pet_attack_tick::start_pet_attack_tick_task(
            self.world.clone(),
        ));
        info!("Pet auto-attack tick started (1s interval)");

        bg_tasks.push(
            crate::systems::offline_merchant::start_offline_merchant_task(self.world.clone()),
        );
        info!("Offline merchant tick started (10s interval)");

        bg_tasks.push(
            crate::systems::chaos_stone_tick::start_chaos_stone_tick_task(self.world.clone()),
        );
        info!("Chaos stone respawn tick started (1s interval)");

        bg_tasks.push(crate::systems::daily_reset::start_daily_reset_task(
            self.world.clone(),
            self.pool.clone(),
        ));
        info!("Daily reset system started (60s check interval)");

        bg_tasks.push(crate::handler::tournament::start_tournament_tick_task(
            self.world.clone(),
        ));
        info!("Tournament timer tick started (1s interval)");

        bg_tasks.push(crate::systems::auto_harvest::start_auto_harvest_task(
            self.world.clone(),
        ));
        info!("Auto harvest system started (5s tick)");

        bg_tasks.push(
            crate::systems::concurrent_update::start_concurrent_update_task(
                self.world.clone(),
                self.pool.clone(),
            ),
        );
        info!("Concurrent user update started (120s interval)");

        bg_tasks.push(crate::systems::zone_rewards::start_zone_online_reward_task(
            self.world.clone(),
        ));
        info!("Zone online reward tick started (10s interval)");

        // DB bot definitions are loaded above, but are materialised only by an
        // explicit GM command. Starting thousands of bots automatically makes
        // a development client unusable and differs from the reference flow.
        info!(
            templates = self.world.get_all_bot_templates().len(),
            merchant_stalls = self.world.get_all_bot_merchant_data().len(),
            active = self.world.bot_count(),
            "DB bot definitions ready — manual GM spawn only"
        );
        bg_tasks.push(crate::systems::bot_ai::start_bot_ai_task(
            self.world.clone(),
        ));
        info!("Bot AI tick started");

        // DISABLED: heartbeat probe sends opcode=0x02 (WIZ_NEW_CHAR) with 16 random bytes.
        // v2600 client parses this as NPC data → garbage parse → memory corruption → bag items cleared.
        // TODO: either remove entirely or send a properly formatted 0x02 packet.
        // bg_tasks.push(
        //     crate::systems::heartbeat_probe::start_heartbeat_probe_task(self.world.clone()),
        // );
        info!("S→C heartbeat probe DISABLED (0x02 collision with WIZ_NEW_CHAR)");

        // King election timer — runs every 60s, checks both nations.
        // CheckKingTimer() per nation once per minute.
        {
            let world = self.world.clone();
            let pool = self.pool.clone();
            bg_tasks.push(tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    crate::handler::king::check_king_timer(&world, 1, &pool);
                    crate::handler::king::check_king_timer(&world, 2, &pool);
                }
            }));
        }
        info!("King election timer started (60s interval)");

        info!("====================================");
        info!("  Game Server READY — accepting connections");
        info!("  Bind: {}", self.config.bind_addr);
        info!("====================================");

        // Main accept loop with graceful shutdown on Ctrl+C / SIGTERM.
        let shutdown = async {
            let ctrl_c = tokio::signal::ctrl_c();
            #[cfg(unix)]
            {
                let mut sigterm =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("failed to register SIGTERM handler");
                tokio::select! {
                    _ = ctrl_c => {}
                    _ = sigterm.recv() => {}
                }
            }
            #[cfg(not(unix))]
            {
                ctrl_c.await.ok();
            }
        };
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let pool = self.pool.clone();
                            let world = self.world.clone();

                            // Global session cap — reject when at capacity
                            if world.session_count() >= MAX_SESSIONS {
                                warn!("[{}] Connection rejected: server full ({} sessions)", addr, MAX_SESSIONS);
                                drop(stream);
                                continue;
                            }

                            // Rate limit: check per-IP connection count
                            let ip = addr.ip();
                            if let Err(e) = world.rate_limiter().register_connection(ip) {
                                warn!("[{}] Connection rejected: {}", addr, e);
                                drop(stream);
                                continue;
                            }

                            // allocate_session_id() internally registers in the rate limiter
                            let session_id = world.allocate_session_id();
                            tokio::spawn(async move {
                                let session = ClientSession::new(stream, addr, pool, session_id, world.clone());
                                // Run in a nested spawn so panics are caught and
                                // cleanup is guaranteed even if run() panics.
                                let run_handle = tokio::spawn(async move {
                                    let mut s = session;
                                    let result = s.run().await;
                                    (s, result)
                                });
                                match run_handle.await {
                                    Ok((mut s, result)) => {
                                        if let Err(e) = result {
                                            error!("[{}] Session error: {}", addr, e);
                                        }
                                        s.cleanup().await;
                                    }
                                    Err(join_err) => {
                                        // Panic: session object lost — do minimal cleanup.
                                        error!("[{}] Session panicked: {}", addr, join_err);
                                        world.unregister_session(session_id);
                                    }
                                }
                                // Rate limit: decrement IP connection count
                                world.rate_limiter().unregister_connection(ip);
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = &mut shutdown => {
                    break;
                }
            }
        }

        // Shutdown path (reached by Ctrl+C or SIGTERM)
        info!(
            "Shutdown signal received — notifying {} online players...",
            self.world.session_count()
        );

        // Broadcast shutdown notice to all connected players before saving.
        let notice = crate::systems::timed_notice::build_notice_packet(
            8, // WAR_SYSTEM_CHAT
            "Server shutting down for maintenance. Your progress will be saved.",
        );
        self.world.broadcast_to_all(Arc::new(notice), None);

        info!("Stopping {} background tasks...", bg_tasks.len());
        for handle in &bg_tasks {
            handle.abort();
        }

        info!("Saving all online characters (30s timeout)...");
        match tokio::time::timeout(
            Duration::from_secs(30),
            crate::systems::character_save::save_all_characters_sync(&self.world, &self.pool),
        )
        .await
        {
            Ok(()) => info!("All characters saved."),
            Err(_) => warn!("Shutdown save timed out after 30s — some data may be lost."),
        }

        // Reset concurrent user count so login server shows 0 immediately.
        let srv_repo = ko_db::repositories::server_list::ServerListRepository::new(&self.pool);
        if let Err(e) = srv_repo.update_concurrent_users(1, 0).await {
            tracing::warn!("Failed to reset concurrent user count on shutdown: {}", e);
        }

        info!("Server shut down.");
        Ok(())
    }
}
