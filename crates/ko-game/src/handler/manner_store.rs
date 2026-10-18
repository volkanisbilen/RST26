//! JstKO "Special Store [Loyalty & Manner]" (hook `WIZ_EXT_HOOK` 0xE9 / sub `MANNER` 0xF0).
//!
//! - C2S 1 open → S2C 1 `[u32 manner][u32 loyalty][u32 daily secs left][u16 daily points][u16 n]`
//!   + n x `[u16 index][u8 tab][u32 item][u16 count][u32 price][u8 currency][u32 days]`
//! - C2S 2 `[u16 index]` buy → S2C 2 `[u8 result][u32 manner][u32 loyalty]` (1 ok, 2 points, 3 bag full, 4 invalid)
//! - C2S 3 daily reward → S2C 3 `[u8 result][u32 manner][u32 daily secs left]` (1 ok, 2 not yet)
//! - C2S 4 `[u32 item]` tooltip → S2C 4 `[u32 item][str name][u32 icon][u8 n]` + n x `[u8 colour][str line]`
//!
//! The shop rows live in the `manner_store_items` table (currency 0 = manner points, 1 = loyalty).

use ko_protocol::{Opcode, Packet, PacketReader};
use sqlx::Row;
use tracing::warn;

use crate::session::ClientSession;

pub const EXT_SUB_MANNER: u8 = 0xF0;
/// Manner points of one daily claim.
const DAILY_POINTS: u16 = 250;
const DAILY_COOLDOWN_SECS: i64 = 24 * 3600;
const MANNER_MAX: i64 = 2_000_000_000;

fn out(op: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_MANNER);
    pkt.write_u8(op);
    pkt
}

/// Seconds until the next daily claim (0 = claimable now).
async fn daily_secs_left(session: &ClientSession, name: &str) -> u32 {
    let row = sqlx::query(
        "SELECT GREATEST(0, EXTRACT(EPOCH FROM (claimed_at + ($2 || ' seconds')::interval - NOW())))::BIGINT AS left_s \
         FROM manner_daily_claim WHERE char_name = $1",
    )
    .bind(name)
    .bind(DAILY_COOLDOWN_SECS.to_string())
    .fetch_optional(session.pool())
    .await;
    match row {
        Ok(Some(r)) => r.try_get::<i64, _>("left_s").unwrap_or(0).max(0) as u32,
        _ => 0,
    }
}

pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let op = reader.read_u8().unwrap_or(0);
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(ch) = world.get_character_info(sid) else {
        return Ok(());
    };
    match op {
        1 => {
            let rows = sqlx::query(
                "SELECT id, tab, item_id, item_count, price, currency, days \
                 FROM manner_store_items WHERE enabled ORDER BY tab, id LIMIT 400",
            )
            .fetch_all(session.pool())
            .await
            .unwrap_or_else(|e| {
                warn!("[{}] manner store load failed: {e}", session.addr());
                Vec::new()
            });
            let left = daily_secs_left(session, &ch.name).await;
            let mut pkt = out(1);
            pkt.write_u32(ch.manner_point.max(0) as u32);
            pkt.write_u32(ch.loyalty);
            pkt.write_u32(left);
            pkt.write_u16(DAILY_POINTS);
            pkt.write_u16(rows.len() as u16);
            for r in &rows {
                pkt.write_u16(r.try_get::<i32, _>("id").unwrap_or(0) as u16);
                pkt.write_u8(r.try_get::<i16, _>("tab").unwrap_or(0) as u8);
                pkt.write_u32(r.try_get::<i32, _>("item_id").unwrap_or(0) as u32);
                pkt.write_u16(r.try_get::<i16, _>("item_count").unwrap_or(1) as u16);
                pkt.write_u32(r.try_get::<i32, _>("price").unwrap_or(0) as u32);
                pkt.write_u8(r.try_get::<i16, _>("currency").unwrap_or(0) as u8);
                pkt.write_u32(r.try_get::<i32, _>("days").unwrap_or(0) as u32);
            }
            session.send_packet(&pkt).await
        }
        2 => {
            let index = reader.read_u16().unwrap_or(0) as i32;
            let row = sqlx::query(
                "SELECT item_id, item_count, price, currency, days FROM manner_store_items WHERE id = $1 AND enabled",
            )
            .bind(index)
            .fetch_optional(session.pool())
            .await
            .ok()
            .flatten();
            let result_pkt = |code: u8, manner: u32, loyalty: u32| {
                let mut p = out(2);
                p.write_u8(code);
                p.write_u32(manner);
                p.write_u32(loyalty);
                p
            };
            let Some(row) = row else {
                return session
                    .send_packet(&result_pkt(4, ch.manner_point.max(0) as u32, ch.loyalty))
                    .await;
            };
            let item_id = row.try_get::<i32, _>("item_id").unwrap_or(0) as u32;
            let count = row.try_get::<i16, _>("item_count").unwrap_or(1).max(1) as u16;
            let price = row.try_get::<i32, _>("price").unwrap_or(0).max(0) as u32;
            let currency = row.try_get::<i16, _>("currency").unwrap_or(0);
            let days = row.try_get::<i32, _>("days").unwrap_or(0).max(0) as u32;

            let balance = if currency == 1 {
                ch.loyalty
            } else {
                ch.manner_point.max(0) as u32
            };
            if balance < price {
                return session
                    .send_packet(&result_pkt(2, ch.manner_point.max(0) as u32, ch.loyalty))
                    .await;
            }
            if world.find_slot_for_item(sid, item_id, count).is_none() {
                return session
                    .send_packet(&result_pkt(3, ch.manner_point.max(0) as u32, ch.loyalty))
                    .await;
            }
            let given = if days > 0 {
                world.give_item_with_expiry(sid, item_id, count, days)
            } else {
                world.give_item(sid, item_id, count)
            };
            if !given {
                return session
                    .send_packet(&result_pkt(3, ch.manner_point.max(0) as u32, ch.loyalty))
                    .await;
            }
            if currency == 1 {
                crate::systems::loyalty::send_loyalty_change(&world, sid, -(price as i32), false, true, false);
            } else {
                world.update_character_stats(sid, |c| {
                    c.manner_point = (c.manner_point as i64 - price as i64).max(0) as i32;
                });
            }
            let now = world.get_character_info(sid).unwrap_or(ch);
            warn!(
                "[{}] manner store: {} bought item {item_id} x{count} for {price} (currency {currency})",
                session.addr(),
                now.name
            );
            session
                .send_packet(&result_pkt(1, now.manner_point.max(0) as u32, now.loyalty))
                .await
        }
        3 => {
            let left = daily_secs_left(session, &ch.name).await;
            let mut p = out(3);
            if left > 0 {
                p.write_u8(2);
                p.write_u32(ch.manner_point.max(0) as u32);
                p.write_u32(left);
                return session.send_packet(&p).await;
            }
            let _ = sqlx::query(
                "INSERT INTO manner_daily_claim(char_name, claimed_at) VALUES($1, NOW()) \
                 ON CONFLICT (char_name) DO UPDATE SET claimed_at = NOW()",
            )
            .bind(&ch.name)
            .execute(session.pool())
            .await;
            world.update_character_stats(sid, |c| {
                c.manner_point = (c.manner_point as i64 + DAILY_POINTS as i64).min(MANNER_MAX) as i32;
            });
            let now = world.get_character_info(sid).unwrap_or(ch);
            p.write_u8(1);
            p.write_u32(now.manner_point.max(0) as u32);
            p.write_u32(DAILY_COOLDOWN_SECS as u32);
            session.send_packet(&p).await
        }
        4 => {
            let item_id = reader.read_u32().unwrap_or(0);
            let Some(item) = world.get_item(item_id) else {
                return Ok(());
            };
            let mut p = out(4);
            p.write_u32(item_id);
            p.write_string(item.str_name.as_deref().unwrap_or("?"));
            p.write_u32(item.item_icon_id1.unwrap_or(0).max(0) as u32);
            let lines: Vec<String> = item
                .description
                .as_deref()
                .unwrap_or("")
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .take(8)
                .collect();
            p.write_u8(lines.len() as u8);
            for line in &lines {
                p.write_u8(3);
                p.write_string(line);
            }
            session.send_packet(&p).await
        }
        _ => Ok(()),
    }
}
