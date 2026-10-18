//! Standalone JstKO management panel.
//!
//! Talks to PostgreSQL directly, so it keeps working while the game server is stopped. Changes that
//! a running game server must pick up (table reloads, kicks, event control) go through the
//! `admin_command` queue, which the game server polls.

mod extra;
mod beginner;
mod bot_loadout;
mod db_browser;
mod chaotic;
mod events;
mod field_bots;
mod info_panel;
mod patch;
mod patch_tcp;
mod pus;
mod rewards;
mod legacy;
mod sheet;
mod world;

use std::{net::SocketAddr, path::PathBuf};

use axum::Router;
use ko_db::DbPool;
use tracing::{info, warn};

#[derive(Clone)]
pub struct App {
    pub pool: DbPool,
    pub icon_dir: PathBuf,
    /// Where the launcher patch set (zips/ + notices.txt) is packaged (see `patch.rs` / `patch_tcp.rs`).
    pub patch_dir: PathBuf,
}

const SCHEMA: &str = include_str!("../schema.sql");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is not set (use the game server's env file)"))?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&url)
        .await?;
    for stmt in SCHEMA.split("-- ;;") {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            // the migration may have created the tables as another role: not fatal
            if let Err(e) = sqlx::query(stmt).execute(&pool).await {
                warn!(error = %e, "schema step skipped");
            }
        }
    }

    // itemicon/ lives next to the sources (crates/ko-admin/assets/itemicon); ITEM_ICON_DIR overrides it
    let icon_dir = std::env::var("ITEM_ICON_DIR")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            ["/opt/ko26xx-build-2625/crates/ko-admin/assets/itemicon", "./crates/ko-admin/assets/itemicon", "./itemicon"]
                .iter()
                .map(PathBuf::from)
                .find(|p| p.is_dir())
        })
        .unwrap_or_else(|| PathBuf::from("/opt/ko26xx-build-2625/crates/ko-admin/assets/itemicon"));
    if !icon_dir.is_dir() {
        warn!(dir = %icon_dir.display(), "ITEM_ICON_DIR not found: item icons fall back to a placeholder");
    }

    let patch_dir = std::env::var("JSTKO_PATCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/opt/ko26xx-build-2625/client_patch_release"));
    if !patch_dir.is_dir() {
        warn!(dir = %patch_dir.display(), "JSTKO_PATCH_DIR not found: the launcher patch system serves version 0 (nothing) until package_patch_zip.py is run");
    }
    // What the launcher is told to download zips from (CAPISocket::FTP_PROTOCOL::HSACSX is
    // plain HTTP, see patch_tcp.rs) -- must be reachable from players' machines, so it's the
    // public VPS address + ko-admin's HTTP port, not ADMIN_BIND_ADDR's own (often 0.0.0.0) host.
    let patch_http_addr = std::env::var("JSTKO_PATCH_HTTP_ADDR").unwrap_or_else(|_| "84.247.183.23:8090".into());
    // NOT 15100: ko-server's own login server binds the whole 15100-15109 range
    // (crates/ko-game/src/login_server.rs), so anything in that span fighting it
    // for the port crashes ko-server with "Address already in use" on startup.
    let patch_tcp_bind = std::env::var("JSTKO_PATCH_TCP_ADDR").unwrap_or_else(|_| "0.0.0.0:15110".into());
    patch_tcp::spawn(patch_dir.clone(), patch_http_addr, patch_tcp_bind);

    let bind = std::env::var("ADMIN_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8090".into());
    let app = App { pool, icon_dir, patch_dir };
    let router: Router = legacy::routes().merge(extra::routes()).merge(world::routes()).merge(beginner::routes()).merge(sheet::routes()).merge(events::routes()).merge(pus::routes()).merge(rewards::routes()).merge(patch::routes()).merge(chaotic::routes()).merge(bot_loadout::routes()).merge(field_bots::routes()).merge(info_panel::routes()).merge(db_browser::routes()).with_state(app);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!(address = %bind, "ko-admin listening");
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}
