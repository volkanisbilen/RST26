//! Knight Online Login Server
//!
//! Handles launcher authentication and server list before the game client
//! connects to the Game Server. Listens on ports 15100-15109 (LS_* protocol).
//!
//! ## Usage
//!
//! ```sh
//! DATABASE_URL=postgresql://user:pass@localhost:5432/ko_server \
//! RUST_LOG=info \
//! cargo run -p ko-login-server
//! ```

use tracing::{info, warn};

use ko_db::repositories::server_settings::ServerSettingsRepository;
use ko_game::login_server::{LoginServer, LoginServerConfig};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    // Debug build: info + warn + error (all logs visible)
    // Release build: warn + error only (critical info only)
    // Override at runtime: RUST_LOG=debug cargo run -p ko-login-server
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

    println!("╔══════════════════════════════════════════╗");
    println!("║   Knight Online Login Server  v0.4.50    ║");
    println!("╚══════════════════════════════════════════╝");
    info!("[1/4] Initializing...");

    // Configuration from environment variables
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable is required");
    let bind_ip = std::env::var("BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
    let base_port: u16 = std::env::var("BASE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(15100);

    info!(
        "[1/4] Config: bind={}:{}-{}",
        bind_ip,
        base_port,
        base_port + 9
    );

    // Database connection pool
    info!("[2/4] Connecting to database...");
    let pool = ko_db::create_pool(&database_url).await?;
    info!("[2/4] Database connected");

    // Run pending migrations
    info!("[3/4] Running migrations...");
    ko_db::run_migrations(&pool).await?;
    info!("[3/4] Migrations applied");

    // Read server_settings from DB (single query)
    let (version, patch_url, patch_path) = {
        let repo = ServerSettingsRepository::new(&pool);
        match repo.load_server_settings().await {
            Ok(s) => {
                info!(
                    "[3/4] DB: launcher_version={}, game_version={}, patch={}{}",
                    s.launcher_version, s.game_version, s.patch_url, s.patch_path
                );
                (s.launcher_version as u16, s.patch_url, s.patch_path)
            }
            Err(e) => {
                warn!("[3/4] Failed to read server_settings: {}", e);
                (
                    2598u16,
                    "http://127.0.0.1:8080".to_string(),
                    "/patches/".to_string(),
                )
            }
        }
    };

    // Login server config
    let config = LoginServerConfig {
        bind_ip,
        base_port,
        game_server_ip: std::env::var("GAME_SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string()),
        game_server_port: std::env::var("GAME_SERVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(15001),
        version,
        ftp_url: patch_url,
        ftp_path: patch_path,
        news_title: "Login Notice".to_string(),
        news_content: "Welcome to Knight Online!".to_string(),
    };

    // Start login server (listens on 10 ports: base_port .. base_port+9)
    info!("[4/4] Starting login server...");
    let server = LoginServer::new(config, pool);
    server.run().await
}
