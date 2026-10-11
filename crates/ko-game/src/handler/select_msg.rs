//! WIZ_SELECT_MSG (0x55) handler — NPC dialog menu selection.
//! ## Flow
//! 1. Server sends `SelectMsg()` to client with menu buttons (from Lua quest scripts)
//!    - Stores event IDs in `m_iSelMsgEvent[12]` and sets `m_bSelectMsgFlag`
//! 2. Client sends `RecvSelectMsg` with the selected button index
//!    - Packet: `[u8 menu_id] [sbyte_string lua_filename] [i8 selected_reward]`
//! 3. Server looks up `m_iSelMsgEvent[menu_id]` and runs the quest event via Lua engine
//! The handler validates the selection and dispatches to `quest::quest_v2_run_event`.

use ko_protocol::{Opcode, Packet, PacketReader};
use tracing::debug;

use crate::session::{ClientSession, SessionState};

/// Maximum number of dialog button events.
const MAX_MESSAGE_EVENT: usize = 12;

/// Handle WIZ_SELECT_MSG from the client.
pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    if session.state() != SessionState::InGame {
        return Ok(());
    }

    let world = session.world().clone();
    let sid = session.session_id();

    // Validate state: not trading, not merchanting, not fishing, not mining, not dead
    if world.is_player_dead(sid) || world.is_trading(sid) || world.is_merchanting(sid) {
        return Ok(());
    }

    // Check mining/fishing state
    if world.is_mining(sid) || world.is_fishing(sid) {
        return Ok(());
    }

    // Parse packet: SByte mode
    // [u8 menu_id] [sbyte_string lua_filename] [i8 selected_reward]
    let mut reader = PacketReader::new(&pkt.data);
    let menu_id = match reader.read_u8() {
        Some(v) => v,
        None => return Ok(()),
    };
    let lua_filename = match reader.read_sbyte_string() {
        Some(s) => s,
        None => return Ok(()),
    };
    let selected_reward = reader.read_u8().unwrap_or(0xFF) as i8;

    debug!(
        "[{}] WIZ_SELECT_MSG: menu_id={}, lua='{}', reward={}",
        session.addr(),
        menu_id,
        lua_filename,
        selected_reward,
    );

    // Validate menu_id range
    if menu_id as usize >= MAX_MESSAGE_EVENT {
        // Clear stored events on invalid selection
        world.update_session(sid, |h| {
            h.select_msg_events = [-1; 12];
        });
        return Ok(());
    }

    // Get stored quest state
    let (quest_helper_id, select_msg_flag, selected_event) = world
        .with_session(sid, |h| {
            (
                h.quest_helper_id,
                h.select_msg_flag,
                h.select_msg_events[menu_id as usize],
            )
        })
        .unwrap_or((0, 0, -1));

    // Handle special case: selected_reward == -1 && flag == 5
    let (effective_menu_id, effective_reward) = if selected_reward == -1 && select_msg_flag == 5 {
        (0u8, menu_id as i8)
    } else {
        (menu_id, selected_reward)
    };

    // Look up the event ID for the effective menu selection
    let effective_event = if effective_menu_id != menu_id {
        world
            .with_session(sid, |h| h.select_msg_events[effective_menu_id as usize])
            .unwrap_or(-1)
    } else {
        selected_event
    };

    // Store selected reward and clear stored events in a single lock
    world.update_session(sid, |h| {
        h.by_selected_reward = effective_reward;
        h.select_msg_events = [-1; 12];
    });

    if effective_event < 0 {
        debug!(
            "[{}] WIZ_SELECT_MSG: no event for menu_id={} (event={})",
            session.addr(),
            effective_menu_id,
            effective_event,
        );
        return Ok(());
    }

    if matches!(
        effective_event,
        super::native_events::AKARA_ALTAR_EVENT | super::native_events::AKARA_POST_UP_EVENT
    ) {
        super::native_events::handle_akara_menu_event(session, effective_event).await?;
        return Ok(());
    }

    if super::daily_quest::is_daily_quest_manager_event(effective_event) {
        super::daily_quest::handle_daily_quest_manager_event(session, effective_event).await?;
        return Ok(());
    }

    // Normal Lua selections must have an active quest helper. Akara's target
    // model emits no WIZ_CLIENT_EVENT, so its two native menu events are
    // deliberately dispatched above without fabricating a quest helper.
    if quest_helper_id == 0 {
        return Ok(());
    }

    // Look up the quest helper to get the Lua filename
    let helper = match world.get_quest_helper(quest_helper_id) {
        Some(h) => h,
        None => {
            debug!(
                "[{}] WIZ_SELECT_MSG: quest_helper={} not found",
                session.addr(),
                quest_helper_id,
            );
            return Ok(());
        }
    };

    // Run the quest Lua script with the selected event
    tracing::info!(
        "[{}] WIZ_SELECT_MSG: running quest event={} reward={} helper={} lua='{}'",
        session.addr(),
        effective_event,
        effective_reward,
        quest_helper_id,
        helper.str_lua_filename,
    );
    super::quest::quest_v2_run_event(&world, sid, &helper, effective_event, effective_reward);

    Ok(())
}

/// Send a SelectMsg dialog menu to a client.
/// Builds and sends the WIZ_SELECT_MSG packet and stores the event IDs
/// so `RecvSelectMsg` can look them up when the player selects an option.
#[allow(clippy::too_many_arguments)]
pub fn send_select_msg(
    world: &crate::world::WorldState,
    sid: crate::zone::SessionId,
    flag: u8,
    quest_id: i32,
    header_text: i32,
    button_texts: &[i32; MAX_MESSAGE_EVENT],
    button_events: &[i32; MAX_MESSAGE_EVENT],
    lua_filename: &str,
) {
    // Get event SID for the packet
    let event_sid = world.with_session(sid, |h| h.event_sid as u32).unwrap_or(0);

    // Build the packet
    // C++ format: [u32 event_sid] [u8 flag] [i32 quest_id] [i32 header_text] [i32 * 12 button_texts] [sbyte_string lua_filename]
    let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
    pkt.write_u32(event_sid);
    pkt.write_u8(flag);
    pkt.write_i32(quest_id);
    pkt.write_i32(header_text);
    for &text_id in button_texts.iter() {
        pkt.write_i32(text_id);
    }
    pkt.write_sbyte_string(lua_filename);

    world.send_to_session_owned(sid, pkt);

    // Store state for when the client responds
    world.update_session(sid, |h| {
        h.select_msg_flag = flag;
        h.select_msg_events = *button_events;
    });
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use ko_protocol::Packet;

    #[test]
    fn test_max_message_event_constant() {
        assert_eq!(MAX_MESSAGE_EVENT, 12);
    }

    #[test]
    fn test_recv_select_msg_packet_format() {
        // Build a client -> server packet matching C++ RecvSelectMsg format
        let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
        pkt.write_u8(3); // menu_id
        pkt.write_sbyte_string("quest_script.lua"); // lua filename
        pkt.write_u8(0xFF); // selected_reward (-1 as u8)

        assert_eq!(pkt.opcode, Opcode::WizSelectMsg as u8);

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8().unwrap(), 3);
        assert_eq!(reader.read_sbyte_string().unwrap(), "quest_script.lua");
        assert_eq!(reader.read_u8().unwrap() as i8, -1);
    }

    #[test]
    fn test_send_select_msg_packet_format() {
        // Build a server -> client SelectMsg packet
        let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
        let event_sid: u32 = 10042;
        let flag: u8 = 1;
        let quest_id: i32 = 500;
        let header_text: i32 = 1001;

        pkt.write_u32(event_sid);
        pkt.write_u8(flag);
        pkt.write_i32(quest_id);
        pkt.write_i32(header_text);
        // 12 button texts
        for i in 0..12 {
            pkt.write_i32(if i < 3 { 2000 + i } else { -1 });
        }
        pkt.write_sbyte_string("quest_helper.lua");

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u32().unwrap(), 10042);
        assert_eq!(reader.read_u8().unwrap(), 1);
        assert_eq!(reader.read_u32().unwrap() as i32, 500);
        assert_eq!(reader.read_u32().unwrap() as i32, 1001);
        // Read 12 button texts
        assert_eq!(reader.read_u32().unwrap() as i32, 2000);
        assert_eq!(reader.read_u32().unwrap() as i32, 2001);
        assert_eq!(reader.read_u32().unwrap() as i32, 2002);
        for _ in 3..12 {
            assert_eq!(reader.read_u32().unwrap() as i32, -1);
        }
        assert_eq!(reader.read_sbyte_string().unwrap(), "quest_helper.lua");
    }

    #[test]
    fn test_menu_id_bounds() {
        // menu_id must be < MAX_MESSAGE_EVENT (12)
        assert!(11 < MAX_MESSAGE_EVENT);
        assert!((12 >= MAX_MESSAGE_EVENT));
    }

    #[test]
    fn test_special_case_flag5_reward_minus1() {
        // C++ special case: when selected_reward == -1 && flag == 5,
        // effective_menu_id becomes 0 and effective_reward becomes menu_id
        let menu_id: u8 = 7;
        let selected_reward: i8 = -1;
        let select_msg_flag: u8 = 5;

        let (effective_menu_id, effective_reward) = if selected_reward == -1 && select_msg_flag == 5
        {
            (0u8, menu_id as i8)
        } else {
            (menu_id, selected_reward)
        };

        assert_eq!(effective_menu_id, 0);
        assert_eq!(effective_reward, 7);
    }

    #[test]
    fn test_normal_case_no_flag5() {
        let menu_id: u8 = 3;
        let selected_reward: i8 = 2;
        let select_msg_flag: u8 = 1;

        let (effective_menu_id, effective_reward) = if selected_reward == -1 && select_msg_flag == 5
        {
            (0u8, menu_id as i8)
        } else {
            (menu_id, selected_reward)
        };

        assert_eq!(effective_menu_id, 3);
        assert_eq!(effective_reward, 2);
    }

    // ── Sprint 924: Additional coverage ──────────────────────────────

    /// S2C packet size: u32(4)+u8(1)+i32(4)+i32(4)+12*i32(48)+sbyte(1+len).
    #[test]
    fn test_select_msg_s2c_data_length() {
        let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
        pkt.write_u32(100); // event_sid
        pkt.write_u8(1); // flag
        pkt.write_i32(500); // quest_id
        pkt.write_i32(1001); // header_text
        for _ in 0..12 {
            pkt.write_i32(-1);
        }
        pkt.write_sbyte_string("test.lua");
        // 4+1+4+4+48+(1+8) = 70
        assert_eq!(pkt.data.len(), 70);
    }

    /// Empty lua filename in C2S packet.
    #[test]
    fn test_empty_lua_filename() {
        let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
        pkt.write_u8(0); // menu_id
        pkt.write_sbyte_string(""); // empty filename
        pkt.write_u8(0xFF); // reward

        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(0));
        assert_eq!(reader.read_sbyte_string(), Some("".to_string()));
        assert_eq!(reader.read_u8(), Some(0xFF));
        assert_eq!(reader.remaining(), 0);
    }

    /// All 12 button texts set (non -1).
    #[test]
    fn test_all_buttons_active() {
        let mut pkt = Packet::new(Opcode::WizSelectMsg as u8);
        pkt.write_u32(0);
        pkt.write_u8(1);
        pkt.write_i32(1);
        pkt.write_i32(100);
        for i in 0..12i32 {
            pkt.write_i32(200 + i);
        }
        pkt.write_sbyte_string("q.lua");

        let mut r = PacketReader::new(&pkt.data);
        r.read_u32();
        r.read_u8();
        r.read_u32();
        r.read_u32();
        for i in 0..12 {
            assert_eq!(r.read_u32().map(|v| v as i32), Some(200 + i));
        }
    }

    /// Events array should be initialized to -1 (no event).
    #[test]
    fn test_select_msg_events_init() {
        let events: [i32; 12] = [-1; 12];
        for e in &events {
            assert_eq!(*e, -1);
        }
    }

    /// menu_id=0 is valid (first button).
    #[test]
    fn test_recv_zero_menu_id_valid() {
        assert!(0 < MAX_MESSAGE_EVENT);
        // menu_id=11 is last valid
        assert!(11 < MAX_MESSAGE_EVENT);
        // menu_id=12 is out of bounds
        assert!(12 >= MAX_MESSAGE_EVENT);
    }
}
