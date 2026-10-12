//! Standalone JstKO management panel.
//!
//! Talks to PostgreSQL directly, so it keeps working while the game server is stopped. Changes that
//! a running game server must pick up (table reloads, kicks, event control) go through the
//! `admin_command` queue, which the game server polls.

mod extra;
mod legacy;

use std::{net::SocketAddr, path::PathBuf};

use axum::Router;
use ko_db::DbPool;
use tracing::{info, warn};

#[derive(Clone)]
pub struct App {
    pub pool: DbPool,
    pub icon_dir: PathBuf,
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
            sqlx::query(stmt).execute(&pool).await?;
        }
    }

    let icon_dir = PathBuf::from(
        std::env::var("ITEM_ICON_DIR")
            .unwrap_or_else(|_| "/var/www/jstkoadminpanel2/public/assets/img/itemicon".into()),
    );
    if !icon_dir.is_dir() {
        warn!(dir = %icon_dir.display(), "ITEM_ICON_DIR not found: item icons fall back to a placeholder");
    }

    let bind = std::env::var("ADMIN_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8090".into());
    let app = App { pool, icon_dir };
    let router: Router = legacy::routes().merge(extra::routes()).with_state(app);
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!(address = %bind, "ko-admin listening");
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}
