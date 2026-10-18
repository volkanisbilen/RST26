//! WIZ_STATE_CHANGE `bType` constants.
//! Each constant maps to a client state handler.

/// bType 1 - HP regen status (standing/sitting/mining/fishing/flashing).
pub const STATE_CHANGE_HPTYPE: u8 = 1;

/// bType 2 - Need party flag (not looking / seeking / looking for member).
pub const STATE_CHANGE_NEEDPARTY: u8 = 2;

/// bType 3 - Fixed abnormal/view state (normal, giant, dwarf, blinking).
pub const STATE_CHANGE_ABNORMAL: u8 = 3;

/// bType 5 - GM visibility toggle.
pub const STATE_CHANGE_GM_VISIBILITY: u8 = 5;

/// bType 6 - Party leader symbol ('P' icon above head).
pub const STATE_CHANGE_PARTY_LEADER: u8 = 6;

/// bType 7 - Invisibility type.
pub const STATE_CHANGE_INVISIBILITY: u8 = 7;

/// bType 11 - Team colour (red/blue/none for BDW, events).
pub const STATE_CHANGE_TEAM_COLOUR: u8 = 11;

/// bType 12 - Devil/weapons disabled visual (no server-side logic).
pub const STATE_CHANGE_WEAPONS_DISABLED: u8 = 12;

/// bType 19 (0x13) - v2615 Type6 transformation model.
///
/// The unpacked v2615 client resolves this value as a skill ID and rebuilds
/// the player model. A zero value removes the active transformation.
pub const STATE_CHANGE_TRANSFORMATION: u8 = 0x13;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2615_transformation_state_type_is_0x13() {
        assert_eq!(STATE_CHANGE_TRANSFORMATION, 0x13);
    }
}
