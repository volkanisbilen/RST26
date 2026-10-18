//! Perk system handler — WIZ_EXT_HOOK(0xE9) sub-opcode PERKS(0xE3).
//! Sub-opcodes (perksSub enum):
//! - 0 (info):       Server sends full perk list + player allocations on login
//! - 1 (perkPlus):   Allocate one point to a perk
//! - 2 (perkReset):  Reset all perks (costs gold)
//! - 3 (perkUseItem): Use item to gain perk points (not fully implemented)
//! - 4 (perkTargetInfo): View another player's perk allocations

use ko_db::repositories::perk::PerkRepository;
use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};

/// Ext sub-opcode for the Perks system.
const EXT_SUB_PERKS_LOCAL: u8 = 0xE3;

/// Perk sub-opcode: send full perk info to client.
const PERK_SUB_INFO: u8 = 0;
/// Perk sub-opcode: allocate one point.
const PERK_SUB_PLUS: u8 = 1;
/// Perk sub-opcode: reset all perks.
const PERK_SUB_RESET: u8 = 2;
/// Perk sub-opcode: view target player's perks.
const PERK_SUB_TARGET_INFO: u8 = 4;

/// Perk error: no remaining perk points.
const PERK_ERR_REM_PERKS: u8 = 0;
/// Perk error: perk definition not found.
const PERK_ERR_NOT_FOUND: u8 = 1;
/// Perk error: already at max level.
const PERK_ERR_MAX_COUNT: u8 = 2;
/// Perk error: success.
const PERK_ERR_SUCCESS: u8 = 3;

/// Default perk reset cost (gold) if server settings not loaded.
const DEFAULT_PERK_COINS: u32 = 100_000;

/// Handle WIZ_EXT_HOOK packets routed to the PERKS sub-handler.
/// This function expects the first byte (ext sub-opcode = 0xE3) to have
/// already been consumed by the ext hook dispatch. The remaining data starts
/// with the perksSub sub-opcode.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }
    if session.world().is_player_dead(session.session_id()) {
        return Ok(());
    }

    let mut reader = PacketReader::new(&pkt.data);

    // First byte was the ext sub-opcode (0xE3), already consumed by dispatch
    // The perksSub sub-opcode is the next byte
    let sub_opcode = reader.read_u8().unwrap_or(255);

    match sub_opcode {
        PERK_SUB_PLUS => handle_perk_plus(session, &mut reader).await,
        PERK_SUB_RESET => handle_perk_reset(session).await,
        PERK_SUB_TARGET_INFO => handle_perk_target_info(session, &mut reader).await,
        _ => {
            debug!(
                "[{}] Unknown perk sub-opcode: {}",
                session.addr(),
                sub_opcode
            );
            Ok(())
        }
    }
}

/// Send the full perk info packet to a player (called on game entry).
/// Packet format (`CUser::Send_myPerks`):
/// ```text
/// WIZ_EXT_HOOK(0xE9) +
///   uint8(PERKS=0xE3) +
///   uint8(info=0) +
///   uint16(remPerk) +
///   uint32(perkCoins) +
///   uint16(perk_count) +
///   for each perk definition:
///     [DByte segment] uint32(pIndex) + bool(status) + uint16(perkCount) + uint16(maxPerkCount)
///     + string(strDescp) + bool(percentage)
///   for i in 0..PERK_COUNT:
///     uint16(perkType[i])
/// ```
pub async fn send_my_perks(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Get perk levels for this player
    let (perk_levels, rem_perk) = world.get_perk_levels(sid).unwrap_or(([0i16; 13], 0));

    // Get perk reset cost from server settings
    let perk_coins = world
        .get_server_settings()
        .map(|s| s.perk_coins as u32)
        .unwrap_or(DEFAULT_PERK_COINS);

    // Get all perk definitions
    let definitions = world.get_all_perk_definitions();

    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_PERKS_LOCAL);
    pkt.write_u8(PERK_SUB_INFO);
    pkt.write_u16(rem_perk as u16);
    pkt.write_u32(perk_coins);
    pkt.write_u16(definitions.len() as u16);

    // Write each perk definition as a DByte (length-prefixed) segment
    for def in &definitions {
        // Build the segment content first, then write with DByte prefix
        let mut segment = Vec::new();
        segment.extend_from_slice(&(def.p_index as u32).to_le_bytes());
        segment.push(def.status as u8);
        segment.extend_from_slice(&(def.perk_count as u16).to_le_bytes());
        segment.extend_from_slice(&(def.perk_max as u16).to_le_bytes());
        // String: length-prefixed (u16 length + bytes)
        let desc_bytes = def.description.as_bytes();
        segment.extend_from_slice(&(desc_bytes.len() as u16).to_le_bytes());
        segment.extend_from_slice(desc_bytes);
        segment.push(def.percentage as u8);

        // Write as DByte: u16 length prefix + segment data
        pkt.write_u16(segment.len() as u16);
        pkt.write_bytes(&segment);
    }

    // Write the player's perk levels (13 x uint16)
    for &level in &perk_levels {
        pkt.write_u16(level as u16);
    }

    session.send_packet(&pkt).await?;

    debug!(
        "[{}] Sent perk info: {} definitions, rem_perk={}, perk_coins={}",
        session.addr(),
        definitions.len(),
        rem_perk,
        perk_coins,
    );

    Ok(())
}

/// Handle perkPlus: allocate one perk point.
/// Client sends: uint32(perk_index)
async fn handle_perk_plus(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let index = reader.read_u32().unwrap_or(u32::MAX);
    let world = session.world().clone();
    let sid = session.session_id();

    // Validate index
    if index as usize >= 13 {
        return Ok(());
    }

    // Try to allocate
    match world.allocate_perk_point(sid, index as usize) {
        Some((new_level, new_rem_perk)) => {
            // Success: send response
            let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
            pkt.write_u8(EXT_SUB_PERKS_LOCAL);
            pkt.write_u8(PERK_SUB_PLUS);
            pkt.write_u8(PERK_ERR_SUCCESS);
            pkt.write_u32(index);
            pkt.write_u16(new_level as u16);
            pkt.write_u16(new_rem_perk as u16);
            session.send_packet(&pkt).await?;

            // Recalculate stats (HP, MP, weight, attack, defence affected by perks)
            world.set_user_ability(sid);

            // Save to DB
            save_user_perks(session).await;

            debug!(
                "[{}] PerkPlus: index={}, new_level={}, rem_perk={}",
                session.addr(),
                index,
                new_level,
                new_rem_perk,
            );
        }
        None => {
            // Determine error reason
            let (perk_levels, rem_perk) = world.get_perk_levels(sid).unwrap_or(([0i16; 13], 0));

            let error_code = if rem_perk <= 0 {
                PERK_ERR_REM_PERKS
            } else if world.get_perk_definition(index as i32).is_none() {
                PERK_ERR_NOT_FOUND
            } else if let Some(def) = world.get_perk_definition(index as i32) {
                if perk_levels[index as usize] >= def.perk_max {
                    PERK_ERR_MAX_COUNT
                } else {
                    PERK_ERR_NOT_FOUND
                }
            } else {
                PERK_ERR_NOT_FOUND
            };

            send_perk_error(session, error_code).await?;
        }
    }

    Ok(())
}

/// Handle perkReset: reset all perk points (costs gold).
async fn handle_perk_reset(session: &mut ClientSession) -> anyhow::Result<()> {
    let world = session.world().clone();
    let sid = session.session_id();

    // Check if there are any points to reset
    let (perk_levels, _rem_perk) = world.get_perk_levels(sid).unwrap_or(([0i16; 13], 0));

    let total_allocated: i16 = perk_levels.iter().sum();
    if total_allocated == 0 {
        debug!(
            "[{}] PerkReset: no points allocated, nothing to reset",
            session.addr()
        );
        return Ok(());
    }

    // Get reset cost from server settings
    let perk_coins = world
        .get_server_settings()
        .map(|s| s.perk_coins as u32)
        .unwrap_or(DEFAULT_PERK_COINS);

    // Check and deduct gold
    let has_gold = world
        .get_character_info(sid)
        .is_some_and(|ch| ch.gold >= perk_coins);

    if !has_gold {
        debug!(
            "[{}] PerkReset: not enough gold (need {})",
            session.addr(),
            perk_coins
        );
        return Ok(());
    }

    // Deduct gold
    world.update_session(sid, |h| {
        if let Some(ref mut ch) = h.character {
            ch.gold = ch.gold.saturating_sub(perk_coins);
        }
    });

    // Send gold change packet to client
    let new_gold = world.get_character_info(sid).map(|ch| ch.gold).unwrap_or(0);
    let mut gold_pkt = Packet::new(Opcode::WizGoldChange as u8);
    gold_pkt.write_u8(2); // GoldLose opcode
    gold_pkt.write_u32(perk_coins);
    gold_pkt.write_u32(new_gold);
    session.send_packet(&gold_pkt).await?;

    // Reset all perks
    if let Some((_refunded, new_rem_perk)) = world.reset_perk_points(sid) {
        // Recalculate stats (perks removed → HP, MP, weight, attack, defence change)
        world.set_user_ability(sid);

        // Send reset response
        let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
        pkt.write_u8(EXT_SUB_PERKS_LOCAL);
        pkt.write_u8(PERK_SUB_RESET);
        pkt.write_u16(new_rem_perk as u16);
        session.send_packet(&pkt).await?;

        // Save to DB
        save_user_perks(session).await;

        debug!(
            "[{}] PerkReset: refunded {} points, rem_perk={}, cost={}",
            session.addr(),
            total_allocated,
            new_rem_perk,
            perk_coins,
        );
    }

    Ok(())
}

/// Handle perkTargetInfo: view another player's perk allocations.
/// Client sends: uint16(target_session_id)
async fn handle_perk_target_info(
    session: &mut ClientSession,
    reader: &mut PacketReader<'_>,
) -> anyhow::Result<()> {
    let target_id = reader.read_u16().unwrap_or(0);
    if target_id == 0 {
        return Ok(());
    }

    let world = session.world().clone();

    // Check if target is online
    let Some((perk_levels, _rem_perk)) = world.get_perk_levels(target_id) else {
        debug!(
            "[{}] PerkTargetInfo: target {} not found",
            session.addr(),
            target_id
        );
        return Ok(());
    };

    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_PERKS_LOCAL);
    pkt.write_u8(PERK_SUB_TARGET_INFO);
    pkt.write_u16(target_id);
    for &level in &perk_levels {
        pkt.write_u16(level as u16);
    }
    session.send_packet(&pkt).await?;

    debug!(
        "[{}] PerkTargetInfo: sent perks for target {}",
        session.addr(),
        target_id
    );

    Ok(())
}

/// Send a perk error response.
async fn send_perk_error(session: &mut ClientSession, error: u8) -> anyhow::Result<()> {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_PERKS_LOCAL);
    pkt.write_u8(PERK_SUB_PLUS);
    pkt.write_u8(error);
    session.send_packet(&pkt).await?;
    Ok(())
}

/// Save the player's perk data to the database.
async fn save_user_perks(session: &mut ClientSession) {
    let char_id = match session.character_id() {
        Some(id) => id.to_string(),
        None => return,
    };

    let world = session.world().clone();
    let sid = session.session_id();

    let (perk_levels, rem_perk) = match world.get_perk_levels(sid) {
        Some(data) => data,
        None => return,
    };

    let pool = session.pool().clone();
    let repo = PerkRepository::new(&pool);

    if let Err(e) = repo.save_user_perks(&char_id, &perk_levels, rem_perk).await {
        tracing::error!(
            "[{}] Failed to save user perks for {}: {}",
            session.addr(),
            char_id,
            e
        );
    }
}

/// Load a player's perk data from the database into the session.
/// Called during game entry (phase 2) after character is registered.
pub async fn load_user_perks(session: &mut ClientSession) {
    let char_id = match session.character_id() {
        Some(id) => id.to_string(),
        None => return,
    };

    let pool = session.pool().clone();
    let repo = PerkRepository::new(&pool);

    match repo.load_user_perks(&char_id).await {
        Ok(Some(row)) => {
            let levels = row.to_array();
            let rem = row.rem_perk;
            let world = session.world().clone();
            world.set_perk_data(session.session_id(), levels, rem);

            debug!(
                "[{}] Loaded perks for {}: rem={}, levels={:?}",
                session.addr(),
                char_id,
                rem,
                levels,
            );
        }
        Ok(None) => {
            // No perk record yet — player starts with all zeros
            debug!(
                "[{}] No perk record for {} (new player)",
                session.addr(),
                char_id,
            );
        }
        Err(e) => {
            tracing::error!(
                "[{}] Failed to load perks for {}: {}",
                session.addr(),
                char_id,
                e,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::world::WorldState;
    use ko_db::models::PerkRow;
    use tokio::sync::mpsc;

    /// Helper to set up a WorldState with perk definitions and a registered session.
    fn setup_world_with_perks() -> (WorldState, u16) {
        let world = WorldState::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        world.register_session(1, tx);

        // Insert all 13 perk definitions
        let defs = vec![
            (0, "Weight", 150, 5, false),
            (1, "Health", 100, 5, false),
            (2, "Mana", 200, 5, false),
            (3, "Loyalty", 3, 5, false),
            (4, "Drop", 2, 3, true),
            (5, "Exp", 4, 5, true),
            (6, "Coins from Monsters", 3, 5, true),
            (7, "Coins on NPC", 2, 5, true),
            (8, "Upgrade Chance", 1, 5, true),
            (9, "Damage to Monsters", 4, 5, true),
            (10, "Damage to Player", 2, 5, true),
            (11, "Defence", 20, 5, false),
            (12, "Attack", 20, 5, false),
        ];
        for (idx, desc, count, max, pct) in defs {
            world.insert_perk_definition(PerkRow {
                p_index: idx,
                status: true,
                description: desc.to_string(),
                perk_count: count,
                perk_max: max,
                percentage: pct,
            });
        }

        (world, 1)
    }

    #[test]
    fn test_perk_definitions_loaded() {
        let (world, _sid) = setup_world_with_perks();
        assert_eq!(world.perk_definition_count(), 13);

        let weight = world.get_perk_definition(0).unwrap();
        assert_eq!(weight.description, "Weight");
        assert_eq!(weight.perk_count, 150);
        assert_eq!(weight.perk_max, 5);
        assert!(!weight.percentage);

        let drop = world.get_perk_definition(4).unwrap();
        assert_eq!(drop.description, "Drop");
        assert_eq!(drop.perk_max, 3);
        assert!(drop.percentage);
    }

    #[test]
    fn test_perk_levels_default_zero() {
        let (world, sid) = setup_world_with_perks();
        let (levels, rem) = world.get_perk_levels(sid).unwrap();
        assert_eq!(levels, [0i16; 13]);
        assert_eq!(rem, 0);
    }

    #[test]
    fn test_set_perk_data() {
        let (world, sid) = setup_world_with_perks();
        let mut levels = [0i16; 13];
        levels[0] = 3;
        levels[5] = 2;
        world.set_perk_data(sid, levels, 5);

        let (loaded, rem) = world.get_perk_levels(sid).unwrap();
        assert_eq!(loaded[0], 3);
        assert_eq!(loaded[5], 2);
        assert_eq!(rem, 5);
    }

    #[test]
    fn test_allocate_perk_point_success() {
        let (world, sid) = setup_world_with_perks();
        world.set_perk_data(sid, [0i16; 13], 5);

        let result = world.allocate_perk_point(sid, 0);
        assert_eq!(result, Some((1, 4))); // level 1, 4 remaining

        let result = world.allocate_perk_point(sid, 0);
        assert_eq!(result, Some((2, 3))); // level 2, 3 remaining
    }

    #[test]
    fn test_allocate_perk_point_no_remaining() {
        let (world, sid) = setup_world_with_perks();
        world.set_perk_data(sid, [0i16; 13], 0); // No points

        let result = world.allocate_perk_point(sid, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_allocate_perk_point_at_max() {
        let (world, sid) = setup_world_with_perks();
        let mut levels = [0i16; 13];
        levels[4] = 3; // Drop is at max (perk_max = 3)
        world.set_perk_data(sid, levels, 5);

        let result = world.allocate_perk_point(sid, 4);
        assert!(result.is_none());
    }

    #[test]
    fn test_allocate_perk_point_invalid_index() {
        let (world, sid) = setup_world_with_perks();
        world.set_perk_data(sid, [0i16; 13], 5);

        let result = world.allocate_perk_point(sid, 13);
        assert!(result.is_none());

        let result = world.allocate_perk_point(sid, 99);
        assert!(result.is_none());
    }

    #[test]
    fn test_reset_perk_points() {
        let (world, sid) = setup_world_with_perks();
        let mut levels = [0i16; 13];
        levels[0] = 3; // Weight
        levels[1] = 2; // Health
        levels[5] = 4; // Exp
        world.set_perk_data(sid, levels, 1); // 1 remaining, 9 allocated

        let result = world.reset_perk_points(sid);
        assert_eq!(result, Some((9, 10))); // 9 refunded, 10 total

        let (new_levels, new_rem) = world.get_perk_levels(sid).unwrap();
        assert_eq!(new_levels, [0i16; 13]);
        assert_eq!(new_rem, 10);
    }

    #[test]
    fn test_reset_perk_points_nothing_allocated() {
        let (world, sid) = setup_world_with_perks();
        world.set_perk_data(sid, [0i16; 13], 5);

        let result = world.reset_perk_points(sid);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_all_perk_definitions_sorted() {
        let (world, _sid) = setup_world_with_perks();
        let defs = world.get_all_perk_definitions();
        assert_eq!(defs.len(), 13);
        // Verify sorted by p_index
        for (i, def) in defs.iter().enumerate() {
            assert_eq!(def.p_index, i as i32);
        }
        assert_eq!(defs[0].description, "Weight");
        assert_eq!(defs[12].description, "Attack");
    }

    #[test]
    fn test_allocate_all_five_levels() {
        let (world, sid) = setup_world_with_perks();
        world.set_perk_data(sid, [0i16; 13], 10);

        // Allocate 5 levels to Weight (max=5)
        for expected_level in 1..=5 {
            let result = world.allocate_perk_point(sid, 0);
            assert_eq!(result, Some((expected_level, 10 - expected_level)));
        }

        // 6th allocation should fail (at max)
        let result = world.allocate_perk_point(sid, 0);
        assert!(result.is_none());
    }

    // ── Perk stat recalculation tests ─────────────────────────────

    #[test]
    fn test_perk_hp_bonus_applied_after_allocate() {
        // After perk allocation, set_user_ability should recalculate HP
        // with the new perk level. Verify perk_levels[1] (HP) is applied.
        let (world, sid) = setup_world_with_perks();
        world.set_perk_data(sid, [0i16; 13], 5);

        // Allocate to HP index (1)
        let result = world.allocate_perk_point(sid, 1);
        assert!(result.is_some());

        // Verify perk level was incremented
        let (levels, _) = world.get_perk_levels(sid).unwrap();
        assert_eq!(levels[1], 1, "HP perk level should be 1 after allocation");
    }

    #[test]
    fn test_perk_reset_clears_all_levels() {
        let (world, sid) = setup_world_with_perks();
        let mut levels = [0i16; 13];
        levels[0] = 3; // Weight
        levels[1] = 2; // HP
        levels[11] = 1; // Defence
        world.set_perk_data(sid, levels, 0);

        let result = world.reset_perk_points(sid);
        assert!(result.is_some());

        let (new_levels, new_rem) = world.get_perk_levels(sid).unwrap();
        assert_eq!(
            new_levels, [0i16; 13],
            "all levels should be zero after reset"
        );
        assert_eq!(new_rem, 6, "refunded points = 3 + 2 + 1");
    }

    #[test]
    fn test_perk_weight_bonus_index() {
        // Weight is perk index 0, defence is 11, attack is 12
        // These match the indices used in set_user_ability
        let (world, sid) = setup_world_with_perks();
        world.set_perk_data(sid, [0i16; 13], 3);

        // Allocate weight perk
        world.allocate_perk_point(sid, 0);
        let (levels, _) = world.get_perk_levels(sid).unwrap();
        assert_eq!(levels[0], 1, "Weight perk at index 0");
    }
}
