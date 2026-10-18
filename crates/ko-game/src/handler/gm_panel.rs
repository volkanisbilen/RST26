//! GM panel (client proxy `gm_panel.cpp`) — WIZ_EXT_HOOK 0xE9, sub 0xF5.
//!
//! Strings are u16 length + bytes (`Packet::write_string` / `read_string`).
//!
//! C->S
//! - `[1]` user list
//! - `[2][str name]` user details ("" = the requesting GM)
//! - `[3][str command]` run a "+command" / "/command" as the GM
//! - `[4][str name][u8 auth]` change a user's authority (0 GM, 1 player)
//! - `[5]` close temple event
//! - `[8][str name][u8 field][i64 value]` user editor APPLY
//! - `[9]` ride / dismount (not supported on this server)
//!
//! S->C
//! - `[1][u16 n]` n x `{str name, u8 level, u8 nation, u16 class, u8 zone, u8 auth}`
//! - `[2][u8 found][str name]` then, when found, the detail block read by `GmPanel_OnRecv`
//! - `[6][u8 ok][str text]` status line
//!
//! Every request is refused unless the sender is a GM (authority 0 or 2).

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::ClientSession;
use crate::world::WorldState;
use crate::zone::SessionId;

pub(crate) const EXT_SUB_GM_PANEL: u8 = 0xF5;

fn base(op: u8) -> Packet {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_GM_PANEL);
    pkt.write_u8(op);
    pkt
}

fn status_packet(ok: bool, text: &str) -> Packet {
    let mut pkt = base(6);
    pkt.write_u8(u8::from(ok));
    pkt.write_string(text);
    pkt
}

fn reply(world: &WorldState, sid: SessionId, ok: bool, text: &str) {
    world.send_to_session_owned(sid, status_packet(ok, text));
}

fn build_list(world: &WorldState) -> Packet {
    let mut rows: Vec<(String, u8, u8, u16, u8, u8)> = Vec::new();
    for id in world.all_ingame_session_ids() {
        let (Some(ch), Some(pos)) = (world.get_character_info(id), world.get_position(id)) else {
            continue;
        };
        rows.push((
            ch.name.clone(),
            ch.level,
            ch.nation,
            ch.class,
            pos.zone_id.min(255) as u8,
            ch.authority,
        ));
    }
    rows.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    rows.truncate(400);
    let mut pkt = base(1);
    pkt.write_u16(rows.len() as u16);
    for (name, level, nation, class, zone, auth) in rows {
        pkt.write_string(&name);
        pkt.write_u8(level);
        pkt.write_u8(nation);
        pkt.write_u16(class);
        pkt.write_u8(zone);
        pkt.write_u8(auth);
    }
    pkt
}

fn build_info(world: &WorldState, requester: SessionId, name: &str) -> Packet {
    let target = if name.is_empty() {
        Some(requester)
    } else {
        world.find_session_by_name(name)
    };
    let found = target.and_then(|t| {
        let ch = world.get_character_info(t)?;
        let pos = world.get_position(t)?;
        let (account, muted, noattack, cash) = world
            .with_session(t, |h| {
                (
                    h.account_id.clone(),
                    h.is_muted,
                    h.attack_disabled_until != 0,
                    h.knight_cash,
                )
            })
            .unwrap_or_default();
        Some((ch, pos, account, muted, noattack, cash))
    });
    let mut pkt = base(2);
    let Some((ch, pos, account, muted, noattack, cash)) = found else {
        pkt.write_u8(0);
        pkt.write_string(name);
        return pkt;
    };
    pkt.write_u8(1);
    pkt.write_string(&ch.name);
    pkt.write_string(&account);
    pkt.write_u8(ch.level);
    pkt.write_u8(ch.nation);
    pkt.write_u8(ch.race);
    pkt.write_u16(ch.class);
    pkt.write_u8(pos.zone_id.min(255) as u8);
    pkt.write_u16(pos.x.max(0.0) as u16);
    pkt.write_u16(pos.z.max(0.0) as u16);
    pkt.write_u32(ch.loyalty);
    pkt.write_u32(ch.loyalty_monthly);
    pkt.write_u32(cash);
    pkt.write_u32(ch.gold);
    let clan = if ch.knights_id > 0 {
        world.get_knights(ch.knights_id).map(|k| k.name).unwrap_or_default()
    } else {
        String::new()
    };
    pkt.write_string(&clan);
    pkt.write_u8(ch.authority);
    pkt.write_u8(u8::from(muted));
    pkt.write_u8(u8::from(noattack));
    for v in [ch.str, ch.sta, ch.dex, ch.intel, ch.cha] {
        pkt.write_u8(v);
    }
    pkt.write_u32(ch.hp.max(0) as u32);
    pkt.write_u32(ch.max_hp.max(0) as u32);
    pkt.write_string(""); // ip
    pkt.write_i64(ch.exp as i64);
    pkt.write_i64(ch.max_exp);
    pkt.write_u16(ch.free_points);
    pkt.write_u16(0); // attack
    pkt.write_u16(0); // defence
    pkt.write_u32(ch.manner_point.max(0) as u32);
    pkt.write_u32(ch.mp.max(0) as u32);
    pkt.write_u32(ch.max_mp.max(0) as u32);
    for _ in 0..6 {
        pkt.write_u16(0); // weapon ac
    }
    for _ in 0..6 {
        pkt.write_u16(0); // resistances
    }
    // free, 1, 2, 3, master
    for i in [0usize, 5, 6, 7, 8] {
        pkt.write_u8(ch.skill_points.get(i).copied().unwrap_or(0));
    }
    pkt
}

/// Applies one user-editor value. Returns a text for the status line.
fn apply_edit(world: &WorldState, target: SessionId, field: u8, value: i64) -> Result<(), &'static str> {
    if value < 0 {
        return Err("negative value");
    }
    let byte = |v: i64| v.clamp(0, 255) as u8;
    match field {
        1 => {
            world.update_character_stats(target, |c| c.level = byte(value).clamp(1, 83));
            if super::daily_quest::auto_start_all(world, target) {
                super::daily_quest::send_list(world, target);
                super::daily_quest::send_progress(world, target);
            }
        }
        2 => world.update_character_stats(target, |c| c.exp = value as u64),
        3 => world.update_character_loyalty(target, value.min(i64::from(u32::MAX)) as u32),
        4 => world.update_character_loyalty_monthly(target, value.min(i64::from(u32::MAX)) as u32),
        5 => world.update_character_stats(target, |c| c.free_points = value.min(65535) as u16),
        6 => world.update_session(target, |h| h.knight_cash = value.min(i64::from(u32::MAX)) as u32),
        7 => world.update_character_stats(target, |c| c.str = byte(value)),
        8 => world.update_character_stats(target, |c| c.sta = byte(value)),
        9 => world.update_character_stats(target, |c| c.dex = byte(value)),
        10 => world.update_character_stats(target, |c| c.intel = byte(value)),
        11 => world.update_character_stats(target, |c| c.cha = byte(value)),
        12..=16 => {
            const SLOT: [usize; 5] = [5, 6, 7, 8, 9];
            let idx = SLOT[(field - 12) as usize];
            world.update_character_stats(target, |c| c.skill_points[idx] = byte(value));
        }
        17 => world.update_character_stats(target, |c| c.manner_point = value.min(i64::from(i32::MAX)) as i32),
        _ => return Err("unknown field"),
    }
    Ok(())
}

pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();
    let Some(me) = world.get_character_info(sid) else {
        return Ok(());
    };
    if me.authority != 0 && me.authority != 2 {
        tracing::warn!(sid, name = %me.name, "GM panel request from a non-GM");
        return Ok(());
    }

    let mut r = PacketReader::new(&pkt.data);
    match r.read_u8().unwrap_or(0) {
        1 => world.send_to_session_owned(sid, build_list(&world)),
        2 => {
            let name = r.read_string().unwrap_or_default();
            world.send_to_session_owned(sid, build_info(&world, sid, &name));
        }
        3 => {
            let cmd = r.read_string().unwrap_or_default();
            let cmd = cmd.trim();
            if !(cmd.starts_with('+') || cmd.starts_with('/')) || cmd.len() < 2 {
                reply(&world, sid, false, "Invalid command");
                return Ok(());
            }
            let handled = super::operator::process_chat_command(session, cmd).await?;
            reply(&world, sid, handled, if handled { "OK" } else { "Command not found" });
        }
        4 => {
            let name = r.read_string().unwrap_or_default();
            let auth = r.read_u8().unwrap_or(1);
            if me.authority != 0 {
                reply(&world, sid, false, "Only a full GM can change authority");
                return Ok(());
            }
            if !matches!(auth, 0 | 1 | 2) {
                reply(&world, sid, false, "Invalid authority");
                return Ok(());
            }
            let Some(target) = world.find_session_by_name(&name) else {
                reply(&world, sid, false, "User is not online");
                return Ok(());
            };
            world.update_session(target, |h| {
                if let Some(ref mut ch) = h.character {
                    ch.authority = auth;
                }
            });
            let repo = ko_db::repositories::character::CharacterRepository::new(session.pool());
            if let Err(e) = repo.update_authority(&name, auth as i16).await {
                tracing::warn!("GM panel: authority save failed for {name}: {e}");
            }
            reply(&world, sid, true, &format!("{name}: authority {auth} (relog to apply)"));
        }
        5 => {
            let handled = super::operator::process_chat_command(session, "+juraidclose").await?;
            reply(&world, sid, handled, "Temple event close requested");
        }
        8 => {
            let name = r.read_string().unwrap_or_default();
            let field = r.read_u8().unwrap_or(0);
            let value = r.read_i64().unwrap_or(-1);
            let Some(target) = world.find_session_by_name(&name) else {
                reply(&world, sid, false, "User is not online");
                return Ok(());
            };
            match apply_edit(&world, target, field, value) {
                Ok(()) => {
                    tracing::info!(gm = %me.name, target = %name, field, value, "GM panel edit");
                    reply(&world, sid, true, &format!("{name}: field {field} = {value} (visible after relog)"));
                }
                Err(e) => reply(&world, sid, false, e),
            }
        }
        9 => reply(&world, sid, false, "Not supported"),
        _ => {}
    }
    Ok(())
}
