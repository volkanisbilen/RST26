//! HopeGuard "Drop Box" panel (hook `WIZ_EXT_HOOK` 0xE9 / sub `DROPBOX` 0xF1): the items of the last
//! box a monster dropped are listed on the right side of the screen; a click takes an item with the
//! game's own `WIZ_ITEM_GET`. The panel itself keeps the 60 s countdown.
//!
//! S2C:
//! - op 1 `[u32 bundle][u16 seconds][u8 n]` + n x `[u8 slot][u32 item][u16 count][u32 icon]` — new box
//! - op 2 `[u32 bundle][u8 slot]` — an item was taken
//! - op 3 `[u32 bundle]` — the box is gone

use ko_protocol::{Opcode, Packet};

use crate::world::{GroundBundle, WorldState};
use crate::zone::SessionId;

pub const EXT_SUB_DROPBOX: u8 = 0xF1;
const BOX_SECONDS: u16 = 60;

/// Who sees the box: the looter, or the whole party of the looter.
fn recipients(world: &WorldState, looter: SessionId) -> Vec<SessionId> {
    world
        .get_party_id(looter)
        .and_then(|pid| world.get_party(pid))
        .map(|p| p.active_members())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| vec![looter])
}

fn icon_of(world: &WorldState, item_id: u32) -> u32 {
    world
        .get_item(item_id)
        .and_then(|i| i.item_icon_id1)
        .unwrap_or(0)
        .max(0) as u32
}

/// A monster dropped a box for `bundle.looter` (and the party).
pub fn notify_new_box(world: &WorldState, bundle: &GroundBundle) {
    let items: Vec<_> = bundle
        .items
        .iter()
        .filter(|i| i.item_id != 0 && i.count > 0)
        .collect();
    if items.is_empty() {
        return;
    }
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_DROPBOX);
    pkt.write_u8(1);
    pkt.write_u32(bundle.bundle_id);
    pkt.write_u16(BOX_SECONDS);
    pkt.write_u8(items.len() as u8);
    for item in items {
        pkt.write_u8(item.slot_id as u8);
        pkt.write_u32(item.item_id);
        pkt.write_u16(item.count);
        pkt.write_u32(icon_of(world, item.item_id));
    }
    for sid in recipients(world, bundle.looter) {
        world.send_to_session(sid, &pkt);
    }
}

/// An item of the box was picked up (by anybody): remove it from the panels.
pub fn notify_taken(world: &WorldState, bundle_id: u32, slot_id: u16, looter: SessionId) {
    let mut pkt = Packet::new(Opcode::EXT_HOOK_S2C);
    pkt.write_u8(EXT_SUB_DROPBOX);
    pkt.write_u8(2);
    pkt.write_u32(bundle_id);
    pkt.write_u8(slot_id as u8);
    for sid in recipients(world, looter) {
        world.send_to_session(sid, &pkt);
    }
}
