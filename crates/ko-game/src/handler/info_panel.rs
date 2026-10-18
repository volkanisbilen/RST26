//! Right-top info list of the client: admin-managed entries plus the live CLAN PREMIUM entry.
//!
//! The client has a native list at the top right (under the Knight Genie bar) that the server
//! fills with `WIZ_NOTICE` type 4 packets, see [`ko_protocol::info_panel`] for the wire format.
//! Each entry is a title button that unfolds its description when it is clicked, so no client
//! patch is needed for the "info window" behaviour.
//!
//! * `info_panel_entry` (DB, edited on the admin panel's "Bilgi Paneli" page) holds the entries:
//!   title, multi-line description, enabled flag, "requires clan premium" flag, order.
//! * The `CLAN PREMIUM` entry is not stored anywhere: it exists exactly while the player's clan
//!   premium is active and carries the remaining time.
//! * Every session remembers what it has been shown ([`InfoPanelSession`]); a periodic tick
//!   compares that with what the player should see now (clan premium started / ran out, joined or
//!   left a clan, the admin changed an entry) and sends only the difference. The admin panel's
//!   save queues an `info_panel_reload` command, which reloads the table and syncs everybody at
//!   once.

use std::sync::Arc;

use ko_db::DbPool;
use ko_protocol::info_panel as wire;
use ko_protocol::Packet;
use sqlx::Row;
use tracing::{info, warn};

use crate::world::{InfoPanelSession, WorldState};
use crate::zone::SessionId;

/// Title of the live clan premium entry (reserved: admin entries cannot use it).
pub const CLAN_PREMIUM_TITLE: &str = "CLAN PREMIUM";

/// Seconds after world entry before the first entries are sent (the client drops notices that
/// arrive during its loading screen).
pub const READY_DELAY_SECS: u64 = 6;

/// Most rows read from the table (the panel page enforces the same bound).
pub const MAX_STORED_ENTRIES: usize = 40;

/// One admin-managed entry, already sanitized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoPanelEntry {
    pub id: i64,
    pub title: String,
    pub description: String,
    /// Only shown to members of a clan whose premium is active.
    pub requires_clan_premium: bool,
}

/// Builds a sanitized entry; `None` when the title or the description is empty after
/// sanitizing, or when it uses the reserved clan premium title.
pub fn make_entry(id: i64, title: &str, description: &str, requires_clan_premium: bool) -> Option<InfoPanelEntry> {
    let title = wire::sanitize_title(title);
    let description = wire::sanitize_description(description);
    if title.is_empty() || description.is_empty() || title.eq_ignore_ascii_case(CLAN_PREMIUM_TITLE) {
        return None;
    }
    Some(InfoPanelEntry { id, title, description, requires_clan_premium })
}

/// Description of the live clan premium entry for `remaining_secs` of premium left.
///
/// Day granularity while more than two days are left, hour granularity afterwards, so the text
/// (and with it the entry in the client) changes rarely.
pub fn clan_premium_description(remaining_secs: u32) -> String {
    let hours = remaining_secs / 3600;
    let left = if hours >= 48 {
        format!("Kalan: {} gun", hours / 24)
    } else if hours >= 1 {
        format!("Kalan: {hours} saat")
    } else {
        "Kalan: 1 saatten az".to_string()
    };
    wire::sanitize_description(&format!("Clan Premium aktif|{left}|EXP, NP, drop ve tamir indirimi"))
}

/// What a player should see: the live clan premium entry first (when the clan premium is
/// active), then the admin entries in order (those that need the clan premium only while it is
/// active), duplicate titles dropped, at most [`wire::MAX_PANEL_ENTRIES`] entries.
pub fn desired_entries(entries: &[InfoPanelEntry], clan_premium_remaining: Option<u32>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(left) = clan_premium_remaining {
        out.push((CLAN_PREMIUM_TITLE.to_string(), clan_premium_description(left)));
    }
    for e in entries {
        if out.len() >= wire::MAX_PANEL_ENTRIES {
            break;
        }
        if e.requires_clan_premium && clan_premium_remaining.is_none() {
            continue;
        }
        if out.iter().any(|(t, _)| t.eq_ignore_ascii_case(&e.title)) {
            continue;
        }
        out.push((e.title.clone(), e.description.clone()));
    }
    out
}

/// Packets that turn what the client shows (`shown`) into `desired`: the entries after the common
/// prefix are removed (last first) and the missing ones added. Empty when nothing changed.
pub fn diff_packets(shown: &[(String, String)], desired: &[(String, String)]) -> Vec<Packet> {
    let keep = shown.iter().zip(desired).take_while(|(a, b)| a == b).count();
    let mut out = Vec::new();
    for (t, d) in shown[keep..].iter().rev() {
        out.push(wire::build_remove_packet(t, d));
    }
    for (t, d) in &desired[keep..] {
        out.push(wire::build_add_packet(t, d));
    }
    out
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Seconds of clan premium the player's clan has left (`None` = no clan or no active premium).
fn clan_premium_remaining(world: &WorldState, sid: SessionId, now: u64) -> Option<u32> {
    let clan_id = world.get_character_info(sid)?.knights_id;
    if clan_id == 0 {
        return None;
    }
    let premium_time = world.get_knights(clan_id)?.premium_time;
    let now = now.min(u32::MAX as u64) as u32;
    (premium_time > now).then(|| premium_time - now)
}

/// Brings the list of one player up to date. Does nothing before the player's ready time.
pub fn sync_session(world: &WorldState, sid: SessionId, now: u64) {
    let Some((ready_at, shown)) = world.with_session(sid, |h| (h.info_panel.ready_at, h.info_panel.shown.clone())) else {
        return;
    };
    if ready_at == 0 || now < ready_at {
        return;
    }
    let entries = world.info_panel_entries();
    let desired = desired_entries(&entries, clan_premium_remaining(world, sid, now));
    if desired == shown {
        return;
    }
    for pkt in diff_packets(&shown, &desired) {
        world.send_to_session_owned(sid, pkt);
    }
    world.update_session(sid, |h| h.info_panel.shown = desired);
}

/// The player entered the world: the list starts empty on the client side, entries follow
/// [`READY_DELAY_SECS`] later from the tick.
pub fn on_entered_game(world: &WorldState, sid: SessionId) {
    let ready_at = unix_now() + READY_DELAY_SECS;
    world.update_session(sid, |h| h.info_panel = InfoPanelSession { ready_at, shown: Vec::new() });
}

/// Periodic sync of every player in the world (clan premium started / expired, clan joined / left).
pub fn tick(world: &WorldState) {
    let now = unix_now();
    for sid in world.collect_sessions_by(|h| h.info_panel.ready_at != 0 && h.info_panel.ready_at <= now) {
        sync_session(world, sid, now);
    }
}

/// Reads the enabled entries of `info_panel_entry` (sanitized, ordered).
pub async fn load_entries(pool: &DbPool) -> Result<Vec<InfoPanelEntry>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, title, description, requires_clan_premium FROM info_panel_entry \
         WHERE enabled ORDER BY sort_order, id LIMIT $1",
    )
    .bind(MAX_STORED_ENTRIES as i64)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let id: i64 = r.try_get("id")?;
        let title: String = r.try_get("title")?;
        let description: String = r.try_get("description")?;
        let requires: bool = r.try_get("requires_clan_premium")?;
        match make_entry(id, &title, &description, requires) {
            Some(e) => out.push(e),
            None => warn!(id, "info_panel_entry skipped: empty or reserved title/description"),
        }
    }
    Ok(out)
}

/// Loads the table into the world (startup and `info_panel_reload`). Returns the entry count.
pub async fn reload_entries(world: &WorldState, pool: &DbPool) -> Result<usize, sqlx::Error> {
    let entries = load_entries(pool).await?;
    let n = entries.len();
    world.set_info_panel_entries(entries);
    info!(count = n, "info_panel_entry table loaded");
    Ok(n)
}

/// Startup loader: a missing table (migration not applied yet) is not fatal.
pub async fn load_at_startup(world: &WorldState, pool: &DbPool) {
    if let Err(e) = reload_entries(world, pool).await {
        warn!("info_panel_entry table not found or error: {e}, skipping");
    }
}

/// `info_panel_reload` admin command: reload the table, then update every player in the world.
pub async fn reload_and_sync(world: &Arc<WorldState>, pool: &DbPool) -> Result<String, String> {
    let n = reload_entries(world, pool).await.map_err(|e| e.to_string())?;
    let now = unix_now();
    let sids = world.collect_sessions_by(|h| h.info_panel.ready_at != 0 && h.info_panel.ready_at <= now);
    let players = sids.len();
    for sid in sids {
        sync_session(world, sid, now);
    }
    Ok(format!("{n} info panel entries loaded, {players} player(s) updated"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{CharacterInfo, KnightsInfo, Position};
    use tokio::sync::mpsc;

    fn entry(id: i64, title: &str, desc: &str, req: bool) -> InfoPanelEntry {
        make_entry(id, title, desc, req).unwrap()
    }

    fn pair(t: &str, d: &str) -> (String, String) {
        (t.to_string(), d.to_string())
    }

    #[test]
    fn make_entry_sanitizes_and_rejects_empty_or_reserved() {
        let e = entry(1, "  Discord   Kalil kazan! ", "Line one\nSatır iki", false);
        assert_eq!(e.title, "Discord Kalil kazan!");
        assert_eq!(e.description, "Line one|Satir iki");
        assert!(make_entry(2, "", "x", false).is_none());
        assert!(make_entry(3, "x", "  \n ", false).is_none());
        assert!(make_entry(4, "clan premium", "x", false).is_none());
        assert!(make_entry(5, "\u{1}", "x", false).is_none());
        let long = make_entry(6, &"T".repeat(400), &"D".repeat(4000), false).unwrap();
        assert!(long.title.chars().count() <= wire::MAX_TITLE_CHARS);
        assert!(long.description.chars().count() <= wire::MAX_DESCRIPTION_CHARS);
    }

    #[test]
    fn clan_premium_text_changes_rarely() {
        let d = clan_premium_description(10 * 86_400 + 5_000);
        assert_eq!(d, "Clan Premium aktif|Kalan: 10 gun|EXP, NP, drop ve tamir indirimi");
        // within a day-bucket the text is identical, so the entry is not re-sent
        assert_eq!(clan_premium_description(10 * 86_400 + 5_000), clan_premium_description(10 * 86_400 + 80_000));
        assert_eq!(clan_premium_description(30 * 3600), "Clan Premium aktif|Kalan: 30 saat|EXP, NP, drop ve tamir indirimi");
        assert!(clan_premium_description(100).contains("1 saatten az"));
        assert!(clan_premium_description(u32::MAX).chars().count() <= wire::MAX_DESCRIPTION_CHARS);
    }

    #[test]
    fn desired_entries_follow_clan_premium_state() {
        let list = vec![
            entry(1, "Discord", "join", false),
            entry(2, "VIP", "clan only", true),
            entry(3, "Rules", "be nice", false),
        ];
        let off = desired_entries(&list, None);
        assert_eq!(off.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), ["Discord", "Rules"]);
        let on = desired_entries(&list, Some(3 * 86_400));
        assert_eq!(on.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), [CLAN_PREMIUM_TITLE, "Discord", "VIP", "Rules"]);
        assert!(on[0].1.contains("Kalan: 3 gun"));
    }

    #[test]
    fn desired_entries_dedupe_and_cap() {
        let mut list = vec![entry(1, "Same", "a", false), entry(2, "same", "b", false)];
        assert_eq!(desired_entries(&list, None).len(), 1);
        list = (0..40).map(|i| entry(i, &format!("E{i}"), "d", false)).collect();
        assert_eq!(desired_entries(&list, None).len(), wire::MAX_PANEL_ENTRIES);
        // the live entry counts against the cap and always comes first
        let capped = desired_entries(&list, Some(100));
        assert_eq!(capped.len(), wire::MAX_PANEL_ENTRIES);
        assert_eq!(capped[0].0, CLAN_PREMIUM_TITLE);
    }

    #[test]
    fn diff_sends_only_what_changed() {
        let a = pair("A", "1");
        let b = pair("B", "2");
        let c = pair("C", "3");
        assert!(diff_packets(&[], &[]).is_empty());
        assert!(diff_packets(&[a.clone(), b.clone()], &[a.clone(), b.clone()]).is_empty());

        let adds = diff_packets(&[], &[a.clone(), b.clone()]);
        let parsed: Vec<_> = adds.iter().map(|p| wire::parse_packet(p).unwrap()).collect();
        assert_eq!(parsed[0], (wire::SUB_ADD, "A".into(), "1".into()));
        assert_eq!(parsed[1], (wire::SUB_ADD, "B".into(), "2".into()));

        // an entry disappears from the middle: the tail is removed (last first) and re-added
        let p = diff_packets(&[a.clone(), b.clone(), c.clone()], &[a.clone(), c.clone()]);
        let parsed: Vec<_> = p.iter().map(|p| wire::parse_packet(p).unwrap()).collect();
        assert_eq!(parsed[0], (wire::SUB_REMOVE, "C".into(), "3".into()));
        assert_eq!(parsed[1], (wire::SUB_REMOVE, "B".into(), "2".into()));
        assert_eq!(parsed[2], (wire::SUB_ADD, "C".into(), "3".into()));
        assert_eq!(parsed.len(), 3);

        // everything removed
        let p = diff_packets(&[a.clone()], &[]);
        assert_eq!(wire::parse_packet(&p[0]).unwrap().0, wire::SUB_REMOVE);
        assert_eq!(p.len(), 1);
    }

    fn world_with_player(clan: u16) -> (WorldState, mpsc::UnboundedReceiver<Arc<Packet>>) {
        let world = WorldState::new();
        let (tx, rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);
        let ch = CharacterInfo { session_id: 1, name: "Tester".into(), knights_id: clan, ..Default::default() };
        world.register_ingame(1, ch, Position::default());
        (world, rx)
    }

    fn clan(id: u16, premium_time: u32) -> KnightsInfo {
        KnightsInfo {
            id,
            flag: 2,
            nation: 1,
            grade: 5,
            ranking: 0,
            name: "C".into(),
            chief: "Tester".into(),
            vice_chief_1: String::new(),
            vice_chief_2: String::new(),
            vice_chief_3: String::new(),
            members: 1,
            points: 0,
            clan_point_fund: 0,
            notice: String::new(),
            cape: 0xFFFF,
            cape_r: 0,
            cape_g: 0,
            cape_b: 0,
            mark_version: 0,
            mark_data: Vec::new(),
            alliance: 0,
            castellan_cape: false,
            cast_cape_id: -1,
            cast_cape_r: 0,
            cast_cape_g: 0,
            cast_cape_b: 0,
            cast_cape_time: 0,
            alliance_req: 0,
            clan_point_method: 0,
            premium_time,
            premium_in_use: 0,
            online_members: 0,
            online_np_count: 0,
            online_exp_count: 0,
        }
    }

    fn drain(rx: &mut mpsc::UnboundedReceiver<Arc<Packet>>) -> Vec<(u8, String, String)> {
        let mut v = Vec::new();
        while let Ok(p) = rx.try_recv() {
            v.push(wire::parse_packet(&p).expect("only info panel packets expected"));
        }
        v
    }

    #[test]
    fn session_sync_tracks_clan_premium_without_relog() {
        let now = 1_800_000_000u64;
        let (world, mut rx) = world_with_player(7);
        world.set_info_panel_entries(vec![entry(1, "Discord", "join us", false), entry(2, "VIP", "only premium", true)]);
        world.insert_knights(clan(7, 0));
        world.update_session(1, |h| h.info_panel = InfoPanelSession { ready_at: now + 5, shown: Vec::new() });

        // not ready yet: nothing is sent
        sync_session(&world, 1, now);
        assert!(drain(&mut rx).is_empty());

        // ready, no premium: only the plain entry
        sync_session(&world, 1, now + 6);
        let got = drain(&mut rx);
        assert_eq!(got, vec![(wire::SUB_ADD, "Discord".into(), "join us".into())]);
        // nothing changed: nothing sent
        sync_session(&world, 1, now + 7);
        assert!(drain(&mut rx).is_empty());

        // the clan premium is granted: CLAN PREMIUM + VIP appear, Discord stays untouched
        world.update_knights(7, |k| k.premium_time = (now + 9 * 86_400) as u32);
        sync_session(&world, 1, now + 8);
        let got = drain(&mut rx);
        assert_eq!(got.iter().filter(|g| g.0 == wire::SUB_ADD).count(), 3);
        assert_eq!(got.iter().filter(|g| g.0 == wire::SUB_REMOVE).count(), 1);
        assert!(got.iter().any(|g| g.0 == wire::SUB_ADD && g.1 == CLAN_PREMIUM_TITLE));
        assert!(got.iter().any(|g| g.0 == wire::SUB_ADD && g.1 == "VIP"));
        let shown = world.with_session(1, |h| h.info_panel.shown.clone()).unwrap();
        assert_eq!(shown.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), [CLAN_PREMIUM_TITLE, "Discord", "VIP"]);

        // expiry: back to the plain list, no relog
        world.update_knights(7, |k| k.premium_time = (now + 100) as u32);
        sync_session(&world, 1, now + 200);
        let got = drain(&mut rx);
        assert!(got.iter().any(|g| g.0 == wire::SUB_REMOVE && g.1 == CLAN_PREMIUM_TITLE));
        assert!(got.iter().any(|g| g.0 == wire::SUB_REMOVE && g.1 == "VIP"));
        let shown = world.with_session(1, |h| h.info_panel.shown.clone()).unwrap();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].0, "Discord");

        // leaving the clan while premium would still run: also gone
        world.update_knights(7, |k| k.premium_time = (now + 9_999_999) as u32);
        sync_session(&world, 1, now + 300);
        assert!(!drain(&mut rx).is_empty());
        world.update_character_stats(1, |c| c.knights_id = 0);
        sync_session(&world, 1, now + 301);
        let got = drain(&mut rx);
        assert!(got.iter().any(|g| g.0 == wire::SUB_REMOVE && g.1 == CLAN_PREMIUM_TITLE));
    }

    #[test]
    fn admin_edit_reaches_players_through_the_next_sync() {
        let now = 1_800_000_000u64;
        let (world, mut rx) = world_with_player(0);
        world.set_info_panel_entries(vec![entry(1, "One", "first", false)]);
        world.update_session(1, |h| h.info_panel = InfoPanelSession { ready_at: now, shown: Vec::new() });
        sync_session(&world, 1, now);
        assert_eq!(drain(&mut rx).len(), 1);
        // the admin changes the text and adds a second entry
        world.set_info_panel_entries(vec![entry(1, "One", "changed", false), entry(2, "Two", "second", false)]);
        sync_session(&world, 1, now + 1);
        let got = drain(&mut rx);
        assert_eq!(got[0], (wire::SUB_REMOVE, "One".into(), "first".into()));
        assert_eq!(got[1], (wire::SUB_ADD, "One".into(), "changed".into()));
        assert_eq!(got[2], (wire::SUB_ADD, "Two".into(), "second".into()));
        // everything disabled: the list is emptied
        world.set_info_panel_entries(Vec::new());
        sync_session(&world, 1, now + 2);
        assert_eq!(drain(&mut rx).iter().filter(|g| g.0 == wire::SUB_REMOVE).count(), 2);
    }

    #[test]
    fn entering_the_game_resets_the_state() {
        let (world, mut rx) = world_with_player(0);
        world.update_session(1, |h| {
            h.info_panel = InfoPanelSession { ready_at: 1, shown: vec![pair("Old", "x")] }
        });
        on_entered_game(&world, 1);
        let st = world.with_session(1, |h| h.info_panel.clone()).unwrap();
        assert!(st.shown.is_empty());
        assert!(st.ready_at > unix_now());
        // not ready for READY_DELAY_SECS: the tick sends nothing
        world.set_info_panel_entries(vec![entry(1, "One", "first", false)]);
        tick(&world);
        assert!(drain(&mut rx).is_empty());
    }

    #[test]
    fn unknown_session_is_ignored() {
        let world = WorldState::new();
        sync_session(&world, 99, 1);
        tick(&world);
    }
}
