//! Knight Online Unified Server
//!
//! Combined login + game server in a single binary. The login server handles
//! launcher authentication (LS_* protocol, ports 15100-15109) and the game
//! server handles all in-game packet processing (WIZ_* protocol, port 15001).
//!
//! Both servers share a single database connection pool and tokio runtime.
//!
//! ## Usage
//!
//! ```sh
//! DATABASE_URL=postgresql://user:pass@localhost:5432/ko_server \
//! RUST_LOG=info \
//! cargo run -p ko-server
//! ```

use std::path::PathBuf;

use tracing::{error, info, warn};

use ko_db::repositories::server_settings::ServerSettingsRepository;
use ko_game::login_server::{LoginServer, LoginServerConfig};
use ko_game::server::{GameServer, ServerConfig};

/// Server version displayed in the startup banner.
const VERSION: &str = "0.5.0";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("╔══════════════════════════════════════════╗");
    println!("║   Knight Online Server  v{}          ║", VERSION);
    println!("║   Login + Game (unified)                 ║");
    println!("╚══════════════════════════════════════════╝");

    // ── Configuration ────────────────────────────────────────────────────
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable is required");
    let bind_ip = std::env::var("BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
    let login_base_port: u16 = std::env::var("BASE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(15100);
    let game_bind_addr =
        std::env::var("BIND_ADDR").unwrap_or_else(|_| format!("{}:15001", bind_ip));
    let map_dir = PathBuf::from(std::env::var("MAP_DIR").unwrap_or_else(|_| "./Map".to_string()));

    info!(
        "[1/6] Config: login={}:{}-{}, game={}, map={}",
        bind_ip,
        login_base_port,
        login_base_port + 9,
        game_bind_addr,
        map_dir.display(),
    );

    // ── Database ─────────────────────────────────────────────────────────
    info!("[2/6] Connecting to database...");
    let pool = ko_db::create_pool(&database_url).await?;
    info!("[2/6] Database connected");

    info!("[3/6] Running migrations...");
    ko_db::run_migrations(&pool).await?;
    info!("[3/6] Migrations applied");

    // ── Server settings from DB ──────────────────────────────────────────
    let (_game_version, launcher_version, patch_url, patch_path) =
        load_server_settings(&pool).await;

    // ── Login server config ──────────────────────────────────────────────
    let login_config = LoginServerConfig {
        bind_ip: bind_ip.clone(),
        base_port: login_base_port,
        game_server_ip: std::env::var("GAME_SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string()),
        game_server_port: std::env::var("GAME_SERVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(15001),
        version: launcher_version,
        ftp_url: patch_url,
        ftp_path: patch_path,
        news_title: "Login Notice".to_string(),
        news_content: "Welcome to Knight Online!".to_string(),
    };

    // ── Game server config ───────────────────────────────────────────────
    let game_config = ServerConfig {
        bind_addr: game_bind_addr,
        map_dir,
    };

    // ── Start both servers ───────────────────────────────────────────────
    info!("[4/6] Loading world data (zones, NPCs, items, magic)...");
    let game_server = GameServer::new(game_config, pool.clone()).await?;

    info!("[5/6] Starting login server...");
    let login_server = LoginServer::new(login_config, pool);

    info!("[6/6] All systems ready — accepting connections");
    info!("====================================");
    info!(
        "  Login:  {}:{}-{}",
        bind_ip,
        login_base_port,
        login_base_port + 9
    );
    info!("  Game:   {}", game_server.bind_addr());
    info!("====================================");

    // Run both servers concurrently — if either exits, the whole process stops.
    let login_handle = tokio::spawn(async move {
        if let Err(e) = login_server.run().await {
            error!("Login server error: {:#}", e);
        }
    });

    let game_handle = tokio::spawn(async move {
        if let Err(e) = game_server.run().await {
            error!("Game server error: {:#}", e);
        }
    });

    // Wait for BOTH servers to finish their own shutdown before the process exits.
    //
    // This used to be a `tokio::select!` on whichever finished first, which raced login
    // and game shutdown: both register their own SIGTERM/Ctrl-C handler and wake on the
    // same signal, but login's shutdown is near-instant (no save) while game's does an
    // awaited, up-to-30s save_all_characters_sync pass over every online character.
    // Whichever branch `select!` picked first (almost always login, since it does
    // nothing) made the whole process exit immediately, dropping game_handle mid-save —
    // silently truncating that save and rolling players still online at restart time
    // back to their last periodic autosave (every 10 minutes). Waiting for both ensures
    // the character-save pass always finishes before the binary actually exits.
    let (login_res, game_res) = tokio::join!(login_handle, game_handle);
    if let Err(e) = login_res {
        warn!("Login server task panicked: {e}");
    }
    if let Err(e) = game_res {
        warn!("Game server task panicked: {e}");
    }

    Ok(())
}

/// Initialize tracing with environment filter.
fn init_tracing() {
    let default_filter = if cfg!(debug_assertions) {
        "info"
    } else {
        "warn"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter)),
        )
        .init();
}

/// Load server settings from the database (version, patch URL).
async fn load_server_settings(pool: &ko_db::DbPool) -> (u16, u16, String, String) {
    let repo = ServerSettingsRepository::new(pool);
    match repo.load_server_settings().await {
        Ok(s) => {
            info!(
                "[3/6] DB: game_version={}, launcher_version={}, patch={}{}",
                s.game_version, s.launcher_version, s.patch_url, s.patch_path
            );
            (
                s.game_version as u16,
                s.launcher_version as u16,
                s.patch_url,
                s.patch_path,
            )
        }
        Err(e) => {
            warn!("[3/6] Failed to read server_settings: {}", e);
            (
                2625u16,
                2625u16,
                "http://127.0.0.1:8080".to_string(),
                "/patches/".to_string(),
            )
        }
    }
}
