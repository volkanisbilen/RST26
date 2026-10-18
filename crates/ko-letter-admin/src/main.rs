use anyhow::{bail, Context, Result};
use chrono::{Datelike, Local};
use serde_json::json;
use sqlx::{PgPool, Row};

const DEFAULT_DATABASE_URL: &str = "postgresql://koserver:koserver123@localhost:5432/ko_server";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        println!("{}", json!({ "ok": false, "error": format!("{error:#}") }));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("ping");
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let pool = PgPool::connect(&database_url)
        .await
        .context("PostgreSQL bağlantısı kurulamadı")?;

    match command {
        "ping" => ping(&pool).await,
        "characters" => characters(&pool, args.get(1).map(String::as_str).unwrap_or("")).await,
        "items" => items(&pool, args.get(1).map(String::as_str).unwrap_or("")).await,
        "send" => send(&pool, &args[1..]).await,
        "recent" => recent(&pool).await,
        _ => bail!("Bilinmeyen komut: {command}"),
    }
}

async fn ping(pool: &PgPool) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM userdata")
        .fetch_one(pool)
        .await?;
    println!("{}", json!({ "ok": true, "characters": count }));
    Ok(())
}

async fn characters(pool: &PgPool, search: &str) -> Result<()> {
    let rows = sqlx::query(
        "SELECT str_user_id, level::integer AS level, nation::integer AS nation, class::integer AS class FROM userdata \
         WHERE str_user_id ILIKE $1 ORDER BY str_user_id LIMIT 100",
    )
    .bind(format!("%{}%", search.trim()))
    .fetch_all(pool)
    .await?;
    let data: Vec<_> = rows
        .iter()
        .map(|r| {
            json!({
                "name": r.get::<String, _>("str_user_id"),
                "level": r.try_get::<i32, _>("level").unwrap_or_default(),
                "nation": r.try_get::<i32, _>("nation").unwrap_or_default(),
                "class": r.try_get::<i32, _>("class").unwrap_or_default()
            })
        })
        .collect();
    println!("{}", json!({ "ok": true, "data": data }));
    Ok(())
}

async fn items(pool: &PgPool, search: &str) -> Result<()> {
    let trimmed = search.trim();
    let numeric = trimmed.parse::<i32>().ok();
    // A wildcard between words makes searches such as "VIP Vault" match
    // client names containing punctuation, e.g. "[VIP] Vault key".
    let name_pattern = format!(
        "%{}%",
        trimmed.split_whitespace().collect::<Vec<_>>().join("%")
    );
    let rows = sqlx::query(
        "SELECT num, COALESCE(str_name, '') AS str_name, COALESCE(duration, 0) AS duration, \
                COALESCE(countable, 0) AS countable, COALESCE(weight, 0) AS weight, \
                COALESCE(kind, 0) AS kind \
         FROM item WHERE ($1::integer IS NOT NULL AND num = $1) OR str_name ILIKE $2 \
         ORDER BY CASE WHEN num = $1 THEN 0 ELSE 1 END, num LIMIT 100",
    )
    .bind(numeric)
    .bind(name_pattern)
    .fetch_all(pool)
    .await?;
    let data: Vec<_> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i32, _>("num"),
                "name": r.get::<String, _>("str_name"),
                "duration": r.get::<i32, _>("duration"),
                "countable": r.get::<i32, _>("countable"),
                "weight": r.get::<i32, _>("weight"),
                "kind": r.get::<i32, _>("kind")
            })
        })
        .collect();
    println!("{}", json!({ "ok": true, "data": data }));
    Ok(())
}

async fn send(pool: &PgPool, args: &[String]) -> Result<()> {
    if args.len() != 8 {
        bail!("send: recipient item_id count durability expiry_days subject message sender bekleniyor");
    }
    let recipient = args[0].trim();
    let item_id: i32 = args[1].parse().context("Item ID geçersiz")?;
    let mut count: i16 = args[2].parse().context("Adet geçersiz")?;
    let durability: i16 = args[3].parse().context("Dayanıklılık geçersiz")?;
    let expiry_days: i32 = args[4].parse().context("Süre geçersiz")?;
    let subject = args[5].trim();
    let message = args[6].trim();
    let sender = args[7].trim();

    if recipient.is_empty() || recipient.chars().count() > 21 {
        bail!("Karakter adı boş veya 21 karakterden uzun");
    }
    if sender.is_empty() || sender.chars().count() > 21 {
        bail!("Gönderen adı boş veya 21 karakterden uzun");
    }
    if subject.chars().count() > 32 || message.chars().count() > 128 {
        bail!("Konu en fazla 32, mesaj en fazla 128 karakter olabilir");
    }
    if !(1..=9999).contains(&count) || durability < 0 || !(0..=3650).contains(&expiry_days) {
        bail!("Adet/dayanıklılık/süre sınır dışında");
    }

    let mut tx = pool.begin().await?;
    let recipient_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM userdata WHERE str_user_id = $1)")
            .bind(recipient)
            .fetch_one(&mut *tx)
            .await?;
    if !recipient_exists {
        bail!("Karakter bulunamadı: {recipient}");
    }

    let item = sqlx::query(
        "SELECT COALESCE(str_name, '') AS str_name, COALESCE(countable, 0) AS countable \
         FROM item WHERE num = $1",
    )
    .bind(item_id)
    .fetch_optional(&mut *tx)
    .await?
    .with_context(|| format!("Item bulunamadı: {item_id}"))?;
    let item_name: String = item.get("str_name");
    let countable: i32 = item.get("countable");
    if countable == 0 {
        count = 1;
    }

    let now = Local::now();
    let send_date = (now.year() % 100) * 10_000 + now.month() as i32 * 100 + now.day() as i32;
    let letter_id: i32 = sqlx::query_scalar(
        "INSERT INTO letter (sender_name, recipient_name, subject, message, b_type, \
          item_id, item_count, item_durability, item_serial, item_expiry, coins, \
          send_date, days_remaining) \
         VALUES ($1,$2,$3,$4,2,$5,$6,$7,0,$8,0,$9,30) RETURNING letter_id",
    )
    .bind(sender)
    .bind(recipient)
    .bind(subject)
    .bind(message)
    .bind(item_id)
    .bind(count)
    .bind(durability)
    .bind(expiry_days)
    .bind(send_date)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO letter_admin_notification (letter_id, recipient_name) VALUES ($1, $2)",
    )
    .bind(letter_id)
    .bind(recipient)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    println!(
        "{}",
        json!({ "ok": true, "letter_id": letter_id, "recipient": recipient,
                "item_id": item_id, "item_name": item_name, "count": count })
    );
    Ok(())
}

async fn recent(pool: &PgPool) -> Result<()> {
    let rows = sqlx::query(
        "SELECT letter_id, recipient_name, subject, item_id, item_count, created_at::text AS created_at, item_taken \
         FROM letter WHERE b_type = 2 ORDER BY letter_id DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await?;
    let data: Vec<_> = rows
        .iter()
        .map(|r| {
            json!({
                "letter_id": r.get::<i32,_>("letter_id"),
                "recipient": r.get::<String,_>("recipient_name"),
                "subject": r.get::<String,_>("subject"),
                "item_id": r.get::<i32,_>("item_id"),
                "count": r.get::<i16,_>("item_count"),
                "created_at": r.get::<String,_>("created_at"),
                "taken": r.get::<i16,_>("item_taken")
            })
        })
        .collect();
    println!("{}", json!({ "ok": true, "data": data }));
    Ok(())
}
