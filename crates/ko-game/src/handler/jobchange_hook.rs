//! JstKO Job Change panel (hook `WIZ_EXT_HOOK` 0xE9 / sub `JOBCHANGE` 0xF7).
//!
//! - C2S `0 [u8 type]`: the player right-clicked a Job Change scroll (type 0 = 700112000, 1 = 700113000)
//! - S2C `0 [u8 type][u8 job]`: open the panel; `job` = the player's class group
//!   (1 warrior, 2 rogue, 3 mage, 4 priest, 5 kurian / portu)
//! - C2S `1 [u8 type][u8 job]`: change to job 1..4 (same checks and effects as the native job change)
//! - S2C `2 [u8 result]`: 1 done, 2 no scroll, 3 already this class, 4 items equipped, 5 failed
//!
//! The daily reward hook shares sub-opcode 0xF7; its packets start with a different op, so the
//! dispatcher in `handler/mod.rs` routes by the first payload byte.

use ko_protocol::{Opcode, Packet, PacketReader};

use crate::session::ClientSession;

pub const EXT_SUB_JOBCHANGE: u8 = 0xF7;

/// Job change packets are `[0][type]` and `[1][type][job]` (3 bytes); the legacy daily reward claim
/// is `[1][u32 item]` (5 bytes).
pub fn is_jobchange_packet(data: &[u8]) -> bool {
    matches!(data, [0, ..]) || (data.first() == Some(&1) && data.len() == 3)
}

pub async fn handle(session: &mut ClientSession, pkt: Packet) -> anyhow::Result<()> {
    let mut reader = PacketReader::new(&pkt.data);
    let op = reader.read_u8().unwrap_or(255);
    let change_type = reader.read_u8().unwrap_or(0);
    let world = session.world().clone();
    let sid = session.session_id();
    match op {
        0 => {
            let Some(ch) = world.get_character_info(sid) else {
                return Ok(());
            };
            if change_type > 1 {
                return Ok(());
            }
            let scroll = super::class_change::job_change_scroll(change_type);
            if !super::class_change::has_item_in_inventory(&world, sid, scroll) {
                let mut out = Packet::new(Opcode::EXT_HOOK_S2C);
                out.write_u8(EXT_SUB_JOBCHANGE);
                out.write_u8(2);
                out.write_u8(2);
                return session.send_packet(&out).await;
            }
            let job = match super::class_change::get_class_type(ch.class) {
                1 | 5 | 6 => 1,
                2 | 7 | 8 => 2,
                3 | 9 | 10 => 3,
                4 | 11 | 12 => 4,
                _ => 5,
            };
            let mut out = Packet::new(Opcode::EXT_HOOK_S2C);
            out.write_u8(EXT_SUB_JOBCHANGE);
            out.write_u8(0);
            out.write_u8(change_type);
            out.write_u8(job);
            session.send_packet(&out).await
        }
        1 => {
            let job = reader.read_u8().unwrap_or(0);
            super::class_change::handle_job_change(session, change_type, job, true).await
        }
        _ => Ok(()),
    }
}
