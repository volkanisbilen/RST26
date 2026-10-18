use anyhow::{Context, Result};
use sqlx::Row;

#[tokio::main]
async fn main() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://koserver:koserver123@localhost:5432/ko_server".into());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .context("connect to PostgreSQL")?;
    sqlx::migrate::Migrator::new(std::path::Path::new("./migrations"))
        .await?
        .run(&pool)
        .await
        .context("apply pending workspace migrations")?;

    let rows = sqlx::query(
        r#"
        SELECT zone_id, left_x, top_z
        FROM npc_spawn
        WHERE npc_id = 31999 AND is_monster = FALSE
        ORDER BY zone_id
        "#,
    )
    .fetch_all(&pool)
    .await?;
    for row in rows {
        println!(
            "zone={} x={} z={}",
            row.try_get::<i16, _>("zone_id")?,
            row.try_get::<i32, _>("left_x")?,
            row.try_get::<i32, _>("top_z")?
        );
    }
    pool.close().await;
    Ok(())
}
