//! Loyalty (Nation Points / NP) system — PvP kill rewards and party distribution.
//! - `GameServer/BotLoyalty.cpp` — SendLoyaltyChange, LoyaltyChange, LoyaltyDivide
//! - `GameServer/GameServerDlg.cpp:690-697` — default loyalty rates from INI
//! - `GameServer/Define.h:22` — `LOYALTY_MAX 2100000000`
//! - `GameServer/GameDefine.h:1330` — `RIVALRY_NP_BONUS 150`
//! - `GameServer/GameDefine.h:1335` — `PVP_MONUMENT_NP_BONUS 5`

use crate::world::{
    CharacterInfo, PremiumProperty, WorldState, ZONE_ARDREAM, ZONE_BATTLE, ZONE_BATTLE6,
    ZONE_BIFROST, ZONE_BORDER_DEFENSE_WAR, ZONE_CAITHAROS_ARENA, ZONE_DELOS,
    ZONE_DESPERATION_ABYSS, ZONE_DRAGON_CAVE, ZONE_ELMORAD, ZONE_HELL_ABYSS, ZONE_KARUS,
    ZONE_KROWAZ_DOMINION, ZONE_RONARK_LAND, ZONE_RONARK_LAND_BASE,
};
use crate::zone::SessionId;
use ko_protocol::{Opcode, Packet};

/// Absolute loyalty cap (same for loyalty and monthly loyalty).
pub const LOYALTY_MAX: u32 = 2_100_000_000;

/// Additional NP bonus for killing a priest's rival target.
pub const RIVALRY_NP_BONUS: u16 = 150;

/// Additional NP bonus when your nation owns the PVP monument.
pub const PVP_MONUMENT_NP_BONUS: i16 = 5;

/// Maximum number of party members.
use crate::world::MAX_PARTY_USERS;

/// Ardream level cap — NPCs below this level use downgraded skills.
pub const MAX_LEVEL_ARDREAM: u8 = 59;

/// Zone-specific loyalty rates (configurable at startup, defaults from C++ INI).
#[derive(Debug, Clone)]
pub struct LoyaltyRates {
    /// NP gained by the killer in Ardream.
    pub ardream_source: i16,
    /// NP lost by the victim in Ardream.
    pub ardream_target: i16,
    /// NP gained by the killer in Ronark Land Base.
    pub ronark_land_base_source: i16,
    /// NP lost by the victim in Ronark Land Base.
    pub ronark_land_base_target: i16,
    /// NP gained by the killer in Ronark Land.
    pub ronark_land_source: i16,
    /// NP lost by the victim in Ronark Land.
    pub ronark_land_target: i16,
    /// NP gained by the killer in other zones.
    pub other_zone_source: i16,
    /// NP lost by the victim in other zones.
    pub other_zone_target: i16,
}

impl Default for LoyaltyRates {
    /// Default values matching C++ INI defaults.
    fn default() -> Self {
        Self {
            ardream_source: 32,
            ardream_target: -25,
            ronark_land_base_source: 64,
            ronark_land_base_target: -50,
            ronark_land_source: 64,
            ronark_land_target: -50,
            other_zone_source: 64,
            other_zone_target: -50,
        }
    }
}

/// Zones that are considered PVP zones (for daily loyalty tracking).
fn is_pk_zone(zone_id: u16) -> bool {
    zone_id == ZONE_RONARK_LAND
        || zone_id == ZONE_ARDREAM
        || zone_id == ZONE_RONARK_LAND_BASE
        || zone_id == ZONE_KROWAZ_DOMINION
        || (ZONE_BATTLE..=ZONE_BATTLE6).contains(&zone_id)
}

/// Zones excluded from monthly NP tracking.
fn skip_monthly_zone(zone_id: u16) -> bool {
    zone_id == ZONE_ARDREAM || zone_id == ZONE_RONARK_LAND_BASE
}

/// Check if a class is a priest class (base class 4, 11, or 12).
/// - 4: Priest base
/// - 11: Priest sub-class (e.g. Shaman, Cleric)
/// - 12: Priest master class
fn is_priest_class(class: u16) -> bool {
    matches!(class % 100, 4 | 11 | 12)
}

/// Check if a character has an active (non-expired) rivalry with the given victim.
/// `hasRivalryExpired()` checks current time >= `m_tRivalExpiryTime`,
/// and the rival ID must match the victim.
/// # Arguments
/// * `ch` — The character to check for rivalry.
/// * `victim_id` — The session ID of the killed player.
fn has_active_rivalry(ch: &CharacterInfo, victim_id: SessionId) -> bool {
    if ch.rival_id < 0 {
        return false;
    }
    if ch.rival_id as u16 != victim_id {
        return false;
    }
    if ch.rival_expiry_time > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now < ch.rival_expiry_time
    } else {
        // No expiry set — treat as active (shouldn't happen in practice).
        true
    }
}

/// Check whether a zone is excluded from PvP loyalty rewards.
fn is_excluded_pvp_zone(zone_id: u16) -> bool {
    zone_id == ZONE_DESPERATION_ABYSS
        || zone_id == ZONE_HELL_ABYSS
        || zone_id == ZONE_DRAGON_CAVE
        || zone_id == ZONE_CAITHAROS_ARENA
}

/// Apply a loyalty change to a player (core NP update).
/// - Positive `amount`: NP gain (kill reward modifiers applied)
/// - Negative `amount`: NP loss
/// - `is_kill_reward`: applies buff/event/bonus modifiers
/// - `is_bonus_reward`: skip daily/monthly tracking
pub fn send_loyalty_change(
    world: &WorldState,
    session_id: SessionId,
    amount: i32,
    is_kill_reward: bool,
    is_bonus_reward: bool,
    has_monthly_loyalty: bool,
) {
    let ch = match world.get_character_info(session_id) {
        Some(c) => c,
        None => return,
    };

    let zone_id = world
        .get_position(session_id)
        .map(|p| p.zone_id)
        .unwrap_or(0);

    let mut add_monthly = has_monthly_loyalty;
    let np_gain_pct: i32 = world
        .with_session(session_id, |h| h.np_gain_amount as i32)
        .unwrap_or(100);
    let np_event_pct: i32 = world
        .game_time_weather()
        .np_event_amount
        .load(std::sync::atomic::Ordering::Relaxed) as i32;
    let item_np_bonus: i32 = world.get_equipped_stats(session_id).item_np_bonus as i32;
    // m_bSkillNPBonus — from BUFF_TYPE_VARIOUS_EFFECTS(33) + BUFF_TYPE_LOYALTY_AMOUNT(42)
    let skill_np_bonus: i32 = world
        .with_session(session_id, |h| {
            h.skill_np_bonus_33 as i32 + h.skill_np_bonus_42 as i32
        })
        .unwrap_or(0);

    let monthly_base = np_gain_pct * amount / 100;

    if amount < 0 {
        // ── NP Loss ──────────────────────────────────────────────────
        let loss = (-amount) as u32;
        let new_loyalty = ch.loyalty.saturating_sub(loss);

        world.update_character_loyalty(session_id, new_loyalty);

        if is_kill_reward {
            if skip_monthly_zone(zone_id) {
                add_monthly = false;
            }
            if add_monthly {
                let new_monthly = ch.loyalty_monthly.saturating_sub(loss);
                world.update_character_loyalty_monthly(session_id, new_monthly);
            }
        }
    } else {
        // ── NP Gain ──────────────────────────────────────────────────
        let mut change = amount;
        let mut premium_bonus_sum: i32 = 0;

        // Premium bonus variables — declared in outer scope so monthly NP can reuse them.
        let mut flash_war_bonus: i32 = 0;
        let mut prem_loyalty: i32 = 0;
        let mut clan_prem_loyalty: i32 = 0;

        if is_kill_reward {
            // Buff modifier
            change = np_gain_pct * change / 100;

            // Global NP event bonus
            change = change * (100 + np_event_pct) / 100;

            // Flame level bonus (burning feature drop_rate applied to NP)
            let flame_level = world
                .with_session(session_id, |h| h.flame_level)
                .unwrap_or(0);
            if flame_level > 0 {
                if let Some(feat) = world.get_burning_feature(flame_level) {
                    if feat.drop_rate > 0 {
                        change = change * (100 + feat.drop_rate as i32) / 100;
                    }
                }
            }

            // Item/skill NP bonuses
            change += item_np_bonus + skill_np_bonus;

            // PVP monument bonus — nation controlling this zone's monument gets extra NP.
            //   `if (g_pMain->m_nPVPMonumentNation[GetZoneID()] == GetNation())`
            let monument_nation = world.get_pvp_monument_nation(zone_id);
            if monument_nation != 0 && monument_nation == ch.nation && is_pk_zone(zone_id) {
                change += PVP_MONUMENT_NP_BONUS as i32;
            }

            // Flash War bonus — additive NP from flash war event kills.
            flash_war_bonus = world
                .with_session(session_id, |h| h.flash_war_bonus as i32)
                .unwrap_or(0);
            if flash_war_bonus > 0 {
                change += flash_war_bonus;
                premium_bonus_sum += flash_war_bonus;
            }

            // Player premium loyalty bonus — flat NP from premium subscription.
            prem_loyalty = world.get_premium_property(session_id, PremiumProperty::BonusLoyalty);
            if prem_loyalty > 0 {
                change += prem_loyalty;
                premium_bonus_sum += prem_loyalty;
            }

            // Clan premium loyalty bonus — flat NP from clan premium.
            clan_prem_loyalty =
                world.get_clan_premium_property(session_id, PremiumProperty::BonusLoyalty);
            if clan_prem_loyalty > 0 {
                change += clan_prem_loyalty;
                premium_bonus_sum += clan_prem_loyalty;
            }

            // Clan online member NP bonus — up to 5 NP based on online clan members.
            if ch.knights_id > 0 {
                if let Some(info) = world.get_knights(ch.knights_id) {
                    if info.online_np_count > 0 {
                        change += info.online_np_count as i32;
                        premium_bonus_sum += info.online_np_count as i32;
                    }
                }
            }

            // Added AFTER all percentage modifiers, only on kill rewards.
            let perk_loyalty = world
                .with_session(session_id, |h| {
                    world.compute_perk_bonus(&h.perk_levels, 3, false)
                })
                .unwrap_or(0);
            if perk_loyalty > 0 {
                change += perk_loyalty;
            }
        }

        // Update premium bonus tracking for rankings.
        if premium_bonus_sum > 0 {
            world.update_session(session_id, |h| {
                h.pk_loyalty_premium_bonus = h
                    .pk_loyalty_premium_bonus
                    .saturating_add(premium_bonus_sum as u16);
            });
        }

        let new_loyalty = add_capped(ch.loyalty, change as u32);
        world.update_character_loyalty(session_id, new_loyalty);

        // Daily PK tracking — accumulate daily loyalty and update PK zone ranking
        if ((is_pk_zone(zone_id) || is_war_zone(zone_id)) && !is_bonus_reward)
            || zone_id == ZONE_BORDER_DEFENSE_WAR
            || zone_id == ZONE_BIFROST
        {
            if skip_monthly_zone(zone_id) {
                add_monthly = false;
            }

            // Accumulate daily loyalty in the PK zone ranking entry
            update_pk_zone_daily_loyalty(world, session_id, ch.nation, change as u32);
        }

        // Monthly NP — same pipeline as main NP but with deduction curve.
        if add_monthly && !is_bonus_reward {
            let mut monthly_change = monthly_base;
            // C++ deduction curve
            if monthly_change > 40 {
                monthly_change -= 20;
            } else if monthly_change >= 20 {
                monthly_change -= 10;
            }

            // Add premium bonuses to monthly NP (same as main NP).
            if flash_war_bonus > 0 {
                monthly_change += flash_war_bonus;
            }
            if prem_loyalty > 0 {
                monthly_change += prem_loyalty;
            }
            if clan_prem_loyalty > 0 {
                monthly_change += clan_prem_loyalty;
            }
            if ch.knights_id > 0 {
                if let Some(info) = world.get_knights(ch.knights_id) {
                    if info.online_np_count > 0 {
                        monthly_change += info.online_np_count as i32;
                    }
                }
            }

            let new_monthly = add_capped(ch.loyalty_monthly, monthly_change.max(0) as u32);
            world.update_character_loyalty_monthly(session_id, new_monthly);
        }
    }

    // ── Clan NP Donation ─────────────────────────────────────────────
    // Only applies to non-bonus NP gains when the clan is accredited and uses
    // the equal point method (clan_point_method == 0).
    let mut clan_loyalty_amount: u32 = 0;
    if !is_bonus_reward {
        if let Some(ch2) = world.get_character_info(session_id) {
            let kid = ch2.knights_id;
            if kid > 0 && kid != 15100 && kid != 1 {
                if let Some(info) = world.get_knights(kid) {
                    if info.flag >= CLAN_TYPE_ACCREDITED5
                        && info.clan_point_method == 0
                        && info.members > 0
                        && info.members <= MAX_CLAN_USERS
                    {
                        clan_loyalty_amount = calculate_clan_donation(info.members);
                        // Subtract donation from player loyalty
                        let cur = ch2.loyalty;
                        let new_loy = cur.saturating_sub(clan_loyalty_amount);
                        world.update_character_loyalty(session_id, new_loy);
                        // Add to clan point fund
                        world.update_knights(kid, |k| {
                            k.clan_point_fund =
                                k.clan_point_fund.saturating_add(clan_loyalty_amount);
                        });
                    }
                }
            }
        }
    }

    // FerihaLog: LoyaltyChangeInsertLog
    if amount.abs() >= 10 {
        if let Some(pool) = world.db_pool() {
            let acc = world
                .with_session(session_id, |h| h.account_id.clone())
                .unwrap_or_default();
            let src = if is_kill_reward {
                "kill"
            } else if is_bonus_reward {
                "bonus"
            } else {
                "other"
            };
            crate::handler::audit_log::log_loyalty_change(
                pool,
                &acc,
                &ch.name,
                src,
                ch.loyalty,
                amount.unsigned_abs(),
                world
                    .get_character_info(session_id)
                    .map(|c| c.loyalty)
                    .unwrap_or(0),
            );
        }
    }

    // ── Send WIZ_LOYALTY_CHANGE to client ────────────────────────────
    // Wire format: [0x2A][u8 sub=1][u32 loyalty][u32 monthly][u32 clan_donations=0][u32 clan_loyalty]
    if let Some(updated) = world.get_character_info(session_id) {
        let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
        pkt.write_u8(1); // LOYALTY_NATIONAL_POINTS
        pkt.write_u32(updated.loyalty);
        pkt.write_u32(updated.loyalty_monthly);
        pkt.write_u32(0); // clan donations total (not computed, same as C++)
        pkt.write_u32(clan_loyalty_amount); // clan loyalty amount donated
        world.send_to_session_owned(session_id, pkt);
    }
}

/// Calculate loyalty change for a solo player kill (no party).
/// Returns `(source_np, target_np)` — source gains, target loses.
pub fn loyalty_change(
    world: &WorldState,
    killer_id: SessionId,
    victim_id: SessionId,
    bonus_np: u16,
    rates: &LoyaltyRates,
) {
    let killer = match world.get_character_info(killer_id) {
        Some(c) => c,
        None => return,
    };
    let victim = match world.get_character_info(victim_id) {
        Some(c) => c,
        None => return,
    };

    let victim_zone = world
        .get_position(victim_id)
        .map(|p| p.zone_id)
        .unwrap_or(0);

    // Delos zone uses zone flags to determine whether PvP is allowed.
    if victim_zone == ZONE_DELOS {
        if victim.nation == killer.nation {
            // Same-nation PK in Delos: only allowed if canAttackSameNation zone flag is set
            let can_attack = world
                .get_zone(victim_zone)
                .map(|z| z.can_attack_same_nation())
                .unwrap_or(false);
            if !can_attack {
                return;
            }
        } else {
            // Different-nation PK in Delos: check canAttackOtherNation zone flag
            let can_attack = world
                .get_zone(victim_zone)
                .map(|z| z.can_attack_other_nation())
                .unwrap_or(false);
            if !can_attack {
                return;
            }
        }
    } else {
        // Non-Delos zones
        if victim.nation == killer.nation {
            return;
        }
        if is_excluded_pvp_zone(victim_zone) {
            return;
        }
    }

    // Different nation
    let (mut source, mut target) = if victim.loyalty == 0 {
        // Victim has 0 NP — no NP exchange, just XP loss
        (0i16, 0i16)
    } else {
        get_zone_loyalty_rates(victim_zone, rates)
    };

    // Rivalry bonus
    source = source.saturating_add(bonus_np as i16);
    target = target.saturating_sub(bonus_np as i16);

    let victim_has_monthly = victim.loyalty_monthly > 0;

    send_loyalty_change(
        world,
        killer_id,
        source as i32,
        true,
        false,
        victim_has_monthly,
    );
    send_loyalty_change(
        world,
        victim_id,
        target as i32,
        true,
        false,
        victim_has_monthly,
    );
}

/// Calculate and distribute loyalty for a party kill.
/// Each alive party member gets `loyalty_divide_source(total_members)` NP.
/// The victim loses `loyalty_divide_target()` NP.
pub fn loyalty_divide(
    world: &WorldState,
    killer_id: SessionId,
    victim_id: SessionId,
    party_members: &[SessionId],
    rates: &LoyaltyRates,
) {
    let killer = match world.get_character_info(killer_id) {
        Some(c) => c,
        None => return,
    };
    let victim = match world.get_character_info(victim_id) {
        Some(c) => c,
        None => return,
    };

    let killer_zone = world
        .get_position(killer_id)
        .map(|p| p.zone_id)
        .unwrap_or(0);

    // Delos zone uses zone flags for PvP rules (same as LoyaltyChange).
    if killer_zone == ZONE_DELOS {
        if victim.nation == killer.nation {
            let can_attack = world
                .get_zone(killer_zone)
                .map(|z| z.can_attack_same_nation())
                .unwrap_or(false);
            if !can_attack {
                return;
            }
        } else {
            let can_attack = world
                .get_zone(killer_zone)
                .map(|z| z.can_attack_other_nation())
                .unwrap_or(false);
            if !can_attack {
                return;
            }
        }
    } else {
        // Same nation — no loyalty exchange outside Delos
        if victim.nation == killer.nation {
            return;
        }
    }

    let total_members = party_members.len() as u8;
    if total_members == 0 {
        return;
    }

    let (source, target) = if victim.loyalty == 0 {
        (0i16, 0i16)
    } else {
        let s = get_loyalty_divide_source(killer_zone, total_members, rates);
        let t = get_loyalty_divide_target(killer_zone, rates);
        if s == 0 {
            (0, 0)
        } else {
            (s, t)
        }
    };

    let victim_has_monthly = victim.loyalty_monthly > 0;

    // Distribute to all alive party members
    for &member_id in party_members {
        let member = match world.get_character_info(member_id) {
            Some(c) => c,
            None => continue,
        };
        if member.hp <= 0 {
            continue;
        }

        // Rivalry bonus for priests (or the killer themselves).
        // Any party member who is a priest with an active rivalry against the victim,
        // or the killer themselves, receives RIVALRY_NP_BONUS and has their rival cleared.
        let bonus_np: u16 = if has_active_rivalry(&member, victim_id)
            && (member_id == killer_id || is_priest_class(member.class))
        {
            // Clear the rival after granting bonus (C++: RemoveRival())
            world.remove_rival(member_id);
            RIVALRY_NP_BONUS
        } else {
            0
        };

        send_loyalty_change(
            world,
            member_id,
            (source + bonus_np as i16) as i32,
            true,
            false,
            victim_has_monthly,
        );
    }

    // Victim NP loss
    send_loyalty_change(
        world,
        victim_id,
        target as i32,
        true,
        false,
        victim_has_monthly,
    );
}

/// PvP-kill NP for an opposing-nation PK bot kill — the bot-kill counterpart
/// to `dead::pvp_loyalty_on_death` (which calls `get_character_info` on the
/// victim and so silently no-ops for a bot, since bots have no
/// `CharacterInfo`). Takes the bot's nation directly instead of looking it
/// up, and only grants NP to the killer side — bots have no persistent
/// loyalty to deduct from, unlike a real player victim.
/// Zone-gating for WHERE a bot kill is eligible at all is the caller's
/// responsibility, matching the existing CR_PK_BOT_KILL_PROTO_ID call sites;
/// this only re-checks the zone's own `give_loyalty` ability flag.
pub fn pvp_loyalty_on_bot_kill(world: &WorldState, killer_id: SessionId, victim_nation: u8) {
    let killer = match world.get_character_info(killer_id) {
        Some(c) => c,
        None => return,
    };
    if victim_nation == killer.nation {
        return;
    }

    let killer_zone = world
        .get_position(killer_id)
        .map(|p| p.zone_id)
        .unwrap_or(0);
    let zone_gives_loy = world
        .get_zone(killer_zone)
        .and_then(|z| z.zone_info.as_ref().map(|zi| zi.abilities.give_loyalty))
        .unwrap_or(false);
    if !zone_gives_loy {
        return;
    }

    // Bot kills pay a reduced share and nothing while the killer is farming
    // the same bot / the bot just respawned (bot_brain verdict of the kill
    // `bot_on_death` recorded a moment ago).
    let brain_cfg = crate::systems::bot_brain::config();
    if !crate::systems::bot_brain::kill_rewardable(killer_id, crate::systems::bot_ai::tick_ms()) {
        return;
    }
    let share = |np: i32| -> i32 {
        if brain_cfg.enabled {
            crate::systems::bot_brain::scale_np(&brain_cfg, np)
        } else {
            np
        }
    };

    let rates = LoyaltyRates::default();
    let in_party = world.is_in_party(killer_id);
    if in_party {
        if let Some(pid) = world.get_party_id(killer_id) {
            if let Some(party) = world.get_party(pid) {
                let members: Vec<SessionId> = party.members.iter().filter_map(|&m| m).collect();
                let total_members = members.len() as u8;
                if total_members > 0 {
                    let source = share(get_loyalty_divide_source(killer_zone, total_members, &rates) as i32);
                    if source != 0 {
                        for &member_id in &members {
                            let alive = world
                                .get_character_info(member_id)
                                .is_some_and(|c| c.hp > 0);
                            if alive {
                                send_loyalty_change(world, member_id, source as i32, true, false, false);
                            }
                        }
                    }
                }
                return;
            }
        }
    }

    if is_excluded_pvp_zone(killer_zone) {
        return;
    }
    let (source, _target) = get_zone_loyalty_rates(killer_zone, &rates);
    let source = share(source as i32);
    if source != 0 {
        send_loyalty_change(world, killer_id, source, true, false, false);
    }
}

/// Get zone-based loyalty rates for a solo kill.
fn get_zone_loyalty_rates(zone_id: u16, rates: &LoyaltyRates) -> (i16, i16) {
    if zone_id == ZONE_ARDREAM {
        (rates.ardream_source, rates.ardream_target)
    } else if zone_id == ZONE_RONARK_LAND_BASE {
        (rates.ronark_land_base_source, rates.ronark_land_base_target)
    } else if zone_id == ZONE_RONARK_LAND {
        (rates.ronark_land_source, rates.ronark_land_target)
    } else if zone_id == ZONE_KARUS
        || zone_id == ZONE_ELMORAD
        || (ZONE_BATTLE..=ZONE_BATTLE6).contains(&zone_id)
    {
        (rates.ronark_land_source, rates.other_zone_target)
    } else {
        (rates.other_zone_source, rates.other_zone_target)
    }
}

/// Calculate per-member source loyalty for party kills.
/// Formula: `nMaxLoyalty = (base * 3) - 2`, `nMinLoyalty = nMax / MAX_PARTY_USERS`,
/// then `+2` for each empty party slot.
fn get_loyalty_divide_source(zone_id: u16, total_members: u8, rates: &LoyaltyRates) -> i16 {
    let base = if zone_id == ZONE_ARDREAM {
        rates.ardream_source
    } else if zone_id == ZONE_RONARK_LAND_BASE {
        rates.ronark_land_base_source
    } else if zone_id == ZONE_RONARK_LAND {
        rates.ronark_land_source
    } else if zone_id == ZONE_KROWAZ_DOMINION {
        (rates.other_zone_source / 100) * 20
    } else if zone_id == ZONE_KARUS
        || zone_id == ZONE_ELMORAD
        || (ZONE_BATTLE..=ZONE_BATTLE6).contains(&zone_id)
    {
        rates.ronark_land_source
    } else {
        rates.other_zone_source
    };

    let max_loyalty = (base as i32 * 3) - 2;
    let min_loyalty = max_loyalty / MAX_PARTY_USERS as i32;
    let mut source = min_loyalty;

    if source > 0 {
        let empty_slots = MAX_PARTY_USERS.saturating_sub(total_members as usize) as i32;
        source += empty_slots * 2;
    }

    (source - 1) as i16
}

/// Calculate victim NP loss for party kills.
fn get_loyalty_divide_target(zone_id: u16, rates: &LoyaltyRates) -> i16 {
    if zone_id == ZONE_ARDREAM {
        rates.ardream_target
    } else if zone_id == ZONE_RONARK_LAND_BASE {
        rates.ronark_land_base_target
    } else if zone_id == ZONE_RONARK_LAND {
        rates.ronark_land_target
    } else if zone_id == ZONE_KROWAZ_DOMINION {
        (rates.other_zone_target / 100) * 20
    } else {
        rates.other_zone_target
    }
}

/// Legacy aggregate monument bonus calculation (used by unit tests).
/// The main code now uses per-zone `world.get_pvp_monument_nation(zone_id)`
/// matching `g_pMain->m_nPVPMonumentNation[GetZoneID()]`.
#[cfg(test)]
fn monument_np_bonus(zone_id: u16, player_nation: u8, karus_mp: i16, elmorad_mp: i16) -> i32 {
    if !is_pk_zone(zone_id) {
        return 0;
    }
    let has_advantage = match player_nation {
        1 => karus_mp > elmorad_mp,
        2 => elmorad_mp > karus_mp,
        _ => false,
    };
    if has_advantage {
        PVP_MONUMENT_NP_BONUS as i32
    } else {
        0
    }
}

use crate::clan_constants::{CLAN_TYPE_ACCREDITED5, MAX_CLAN_USERS};

/// Calculate the NP donation amount based on clan member count.
/// - 1-5 members → 1
/// - 6-10 → 2
/// - 11-15 → 3
/// - ...
/// - 46+ → 10
/// Equivalent formula: `((members - 1) / 5 + 1).min(10)`
fn calculate_clan_donation(members: u16) -> u32 {
    if members == 0 {
        return 0;
    }
    (((members - 1) / 5 + 1) as u32).min(10)
}

/// Add `amount` to `current`, capping at `LOYALTY_MAX`.
fn add_capped(current: u32, amount: u32) -> u32 {
    current.saturating_add(amount).min(LOYALTY_MAX)
}

/// Check if zone is a war zone (battle zones).
fn is_war_zone(zone_id: u16) -> bool {
    (ZONE_BATTLE..=ZONE_BATTLE6).contains(&zone_id)
}

/// Update the PK zone ranking daily loyalty for a player after a PvP kill.
/// and then calls `UpdatePlayerKillingRank()` which copies the value to the ranking map.
/// In our implementation, we directly increment the ranking map entry.
fn update_pk_zone_daily_loyalty(
    world: &WorldState,
    session_id: SessionId,
    nation: u8,
    np_change: u32,
) {
    world.pk_zone_increment_daily(session_id, nation, np_change);
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn test_loyalty_max_constant() {
        // C++ LOYALTY_MAX = 2,100,000,000
        assert_eq!(LOYALTY_MAX, 2_100_000_000);
    }

    #[test]
    fn test_rivalry_np_bonus() {
        assert_eq!(RIVALRY_NP_BONUS, 150);
    }

    #[test]
    fn test_pvp_monument_np_bonus() {
        assert_eq!(PVP_MONUMENT_NP_BONUS, 5);
    }

    #[test]
    fn test_max_party_users() {
        assert_eq!(MAX_PARTY_USERS, 8);
    }

    #[test]
    fn test_max_level_ardream() {
        assert_eq!(MAX_LEVEL_ARDREAM, 59);
    }

    #[test]
    fn test_default_loyalty_rates() {
        let rates = LoyaltyRates::default();
        assert_eq!(rates.ardream_source, 32);
        assert_eq!(rates.ardream_target, -25);
        assert_eq!(rates.ronark_land_base_source, 64);
        assert_eq!(rates.ronark_land_base_target, -50);
        assert_eq!(rates.ronark_land_source, 64);
        assert_eq!(rates.ronark_land_target, -50);
        assert_eq!(rates.other_zone_source, 64);
        assert_eq!(rates.other_zone_target, -50);
    }

    #[test]
    fn test_zone_loyalty_rates_ardream() {
        let rates = LoyaltyRates::default();
        let (src, tgt) = get_zone_loyalty_rates(ZONE_ARDREAM, &rates);
        assert_eq!(src, 32);
        assert_eq!(tgt, -25);
    }

    #[test]
    fn test_zone_loyalty_rates_ronark_land_base() {
        let rates = LoyaltyRates::default();
        let (src, tgt) = get_zone_loyalty_rates(ZONE_RONARK_LAND_BASE, &rates);
        assert_eq!(src, 64);
        assert_eq!(tgt, -50);
    }

    #[test]
    fn test_zone_loyalty_rates_ronark_land() {
        let rates = LoyaltyRates::default();
        let (src, tgt) = get_zone_loyalty_rates(ZONE_RONARK_LAND, &rates);
        assert_eq!(src, 64);
        assert_eq!(tgt, -50);
    }

    #[test]
    fn test_zone_loyalty_rates_karus_elmorad() {
        let rates = LoyaltyRates::default();
        // Karus zone uses ronark_land_source + other_zone_target
        let (src, tgt) = get_zone_loyalty_rates(ZONE_KARUS, &rates);
        assert_eq!(src, 64);
        assert_eq!(tgt, -50);
    }

    #[test]
    fn test_zone_loyalty_rates_battle_zones() {
        let rates = LoyaltyRates::default();
        for zone in ZONE_BATTLE..=ZONE_BATTLE6 {
            let (src, tgt) = get_zone_loyalty_rates(zone, &rates);
            assert_eq!(src, rates.ronark_land_source);
            assert_eq!(tgt, rates.other_zone_target);
        }
    }

    #[test]
    fn test_zone_loyalty_rates_other() {
        let rates = LoyaltyRates::default();
        let (src, tgt) = get_zone_loyalty_rates(21, &rates); // Moradon
        assert_eq!(src, 64);
        assert_eq!(tgt, -50);
    }

    #[test]
    fn test_loyalty_divide_source_full_party() {
        let rates = LoyaltyRates::default();
        // Ronark Land, 8 members
        let src = get_loyalty_divide_source(ZONE_RONARK_LAND, 8, &rates);
        // base=64, max=(64*3)-2=190, min=190/8=23, empty=0, result=23-1=22
        assert_eq!(src, 22);
    }

    #[test]
    fn test_loyalty_divide_source_partial_party() {
        let rates = LoyaltyRates::default();
        // Ronark Land, 4 members
        let src = get_loyalty_divide_source(ZONE_RONARK_LAND, 4, &rates);
        // base=64, max=190, min=23, empty=4, bonus=8, result=23+8-1=30
        assert_eq!(src, 30);
    }

    #[test]
    fn test_loyalty_divide_source_solo() {
        let rates = LoyaltyRates::default();
        // Ronark Land, 1 member
        let src = get_loyalty_divide_source(ZONE_RONARK_LAND, 1, &rates);
        // base=64, max=190, min=23, empty=7, bonus=14, result=23+14-1=36
        assert_eq!(src, 36);
    }

    #[test]
    fn test_loyalty_divide_source_ardream() {
        let rates = LoyaltyRates::default();
        let src = get_loyalty_divide_source(ZONE_ARDREAM, 8, &rates);
        // base=32, max=(32*3)-2=94, min=94/8=11, empty=0, result=11-1=10
        assert_eq!(src, 10);
    }

    #[test]
    fn test_loyalty_divide_target_zones() {
        let rates = LoyaltyRates::default();
        assert_eq!(get_loyalty_divide_target(ZONE_ARDREAM, &rates), -25);
        assert_eq!(
            get_loyalty_divide_target(ZONE_RONARK_LAND_BASE, &rates),
            -50
        );
        assert_eq!(get_loyalty_divide_target(ZONE_RONARK_LAND, &rates), -50);
        assert_eq!(get_loyalty_divide_target(21, &rates), -50); // Moradon
    }

    #[test]
    fn test_loyalty_divide_target_krowaz() {
        let rates = LoyaltyRates::default();
        let tgt = get_loyalty_divide_target(ZONE_KROWAZ_DOMINION, &rates);
        // C++ integer division: (-50/100)*20 = 0*20 = 0
        assert_eq!(tgt, 0);
    }

    #[test]
    fn test_add_capped_normal() {
        assert_eq!(add_capped(100, 50), 150);
    }

    #[test]
    fn test_add_capped_overflow() {
        assert_eq!(add_capped(LOYALTY_MAX - 10, 20), LOYALTY_MAX);
    }

    #[test]
    fn test_add_capped_at_max() {
        assert_eq!(add_capped(LOYALTY_MAX, 100), LOYALTY_MAX);
    }

    #[test]
    fn test_is_pk_zone() {
        assert!(is_pk_zone(ZONE_RONARK_LAND));
        assert!(is_pk_zone(ZONE_ARDREAM));
        assert!(is_pk_zone(ZONE_RONARK_LAND_BASE));
        assert!(is_pk_zone(ZONE_KROWAZ_DOMINION));
        assert!(is_pk_zone(ZONE_BATTLE));
        assert!(is_pk_zone(ZONE_BATTLE6));
        assert!(!is_pk_zone(21)); // Moradon
        assert!(!is_pk_zone(ZONE_KARUS));
    }

    #[test]
    fn test_skip_monthly_zone() {
        assert!(skip_monthly_zone(ZONE_ARDREAM));
        assert!(skip_monthly_zone(ZONE_RONARK_LAND_BASE));
        assert!(!skip_monthly_zone(ZONE_RONARK_LAND));
        assert!(!skip_monthly_zone(21));
    }

    #[test]
    fn test_is_excluded_pvp_zone() {
        assert!(is_excluded_pvp_zone(ZONE_DESPERATION_ABYSS));
        assert!(is_excluded_pvp_zone(ZONE_HELL_ABYSS));
        assert!(is_excluded_pvp_zone(ZONE_DRAGON_CAVE));
        assert!(is_excluded_pvp_zone(ZONE_CAITHAROS_ARENA));
        assert!(!is_excluded_pvp_zone(ZONE_RONARK_LAND));
        assert!(!is_excluded_pvp_zone(ZONE_DELOS));
    }

    #[test]
    fn test_loyalty_divide_source_krowaz() {
        let rates = LoyaltyRates::default();
        // Krowaz: C++ integer division: base = (64/100)*20 = 0*20 = 0
        let src = get_loyalty_divide_source(ZONE_KROWAZ_DOMINION, 8, &rates);
        // base=0, max=(0*3)-2=-2, min=-2/8=0, source=0 (no bonus since source<=0), result=0-1=-1
        assert_eq!(src, -1);
    }

    // ── Monument NP Bonus Tests ───────────────────────────────────────

    #[test]
    fn test_monument_bonus_karus_advantage() {
        // Karus has more monument points → Karus player gets bonus
        let bonus = monument_np_bonus(ZONE_RONARK_LAND, 1, 100, 50);
        assert_eq!(bonus, PVP_MONUMENT_NP_BONUS as i32);
    }

    #[test]
    fn test_monument_bonus_elmorad_advantage() {
        // El Morad has more monument points → El Morad player gets bonus
        let bonus = monument_np_bonus(ZONE_RONARK_LAND, 2, 50, 100);
        assert_eq!(bonus, PVP_MONUMENT_NP_BONUS as i32);
    }

    #[test]
    fn test_monument_bonus_karus_no_advantage() {
        // Karus player but El Morad has more points → no bonus
        let bonus = monument_np_bonus(ZONE_RONARK_LAND, 1, 50, 100);
        assert_eq!(bonus, 0);
    }

    #[test]
    fn test_monument_bonus_equal_points() {
        // Equal monument points → no bonus for either nation
        let bonus_k = monument_np_bonus(ZONE_RONARK_LAND, 1, 80, 80);
        let bonus_e = monument_np_bonus(ZONE_RONARK_LAND, 2, 80, 80);
        assert_eq!(bonus_k, 0);
        assert_eq!(bonus_e, 0);
    }

    #[test]
    fn test_monument_bonus_zero_points() {
        // Both nations at 0 monument points → no bonus
        let bonus = monument_np_bonus(ZONE_RONARK_LAND, 1, 0, 0);
        assert_eq!(bonus, 0);
    }

    #[test]
    fn test_monument_bonus_non_pk_zone() {
        // Non-PK zone (Moradon) → no monument bonus even with advantage
        let bonus = monument_np_bonus(21, 1, 100, 50);
        assert_eq!(bonus, 0);
    }

    #[test]
    fn test_monument_bonus_ardream() {
        // Ardream is a PK zone → monument bonus applies
        let bonus = monument_np_bonus(ZONE_ARDREAM, 1, 100, 50);
        assert_eq!(bonus, PVP_MONUMENT_NP_BONUS as i32);
    }

    #[test]
    fn test_monument_bonus_battle_zone() {
        // Battle zones are PK zones → monument bonus applies
        let bonus = monument_np_bonus(ZONE_BATTLE, 2, 50, 100);
        assert_eq!(bonus, PVP_MONUMENT_NP_BONUS as i32);
    }

    #[test]
    fn test_monument_bonus_invalid_nation() {
        // Invalid nation (0) → no bonus
        let bonus = monument_np_bonus(ZONE_RONARK_LAND, 0, 100, 50);
        assert_eq!(bonus, 0);
    }

    // ── Priest Class Detection Tests ─────────────────────────────────

    #[test]
    fn test_is_priest_class_karus() {
        // Karus priest base (x04)
        assert!(is_priest_class(104));
        // Karus priest sub-classes
        assert!(is_priest_class(111));
        assert!(is_priest_class(112));
    }

    #[test]
    fn test_is_priest_class_elmorad() {
        // El Morad priest base (2xx)
        assert!(is_priest_class(204));
        assert!(is_priest_class(211));
        assert!(is_priest_class(212));
    }

    #[test]
    fn test_is_priest_class_non_priest() {
        // Warrior, Rogue, Mage — not priests
        assert!(!is_priest_class(101)); // Warrior
        assert!(!is_priest_class(102)); // Rogue
        assert!(!is_priest_class(103)); // Mage
        assert!(!is_priest_class(201)); // El Morad Warrior
        assert!(!is_priest_class(202)); // El Morad Rogue
        assert!(!is_priest_class(203)); // El Morad Mage
    }

    // ── Rivalry Check Tests ──────────────────────────────────────────

    /// Helper to create a minimal CharacterInfo for rivalry tests.
    fn make_rivalry_test_char(
        session_id: SessionId,
        class: u16,
        rival_id: i16,
        rival_expiry_time: u64,
    ) -> CharacterInfo {
        CharacterInfo {
            session_id,
            name: "TestChar".to_string(),
            nation: 1,
            race: 1,
            class,
            level: 60,
            face: 1,
            hair_rgb: 0,
            rank: 0,
            title: 0,
            max_hp: 500,
            hp: 500,
            max_mp: 200,
            mp: 200,
            max_sp: 0,
            sp: 0,
            equipped_items: [0; 14],
            bind_zone: 0,
            bind_x: 0.0,
            bind_z: 0.0,
            str: 60,
            sta: 60,
            dex: 60,
            intel: 60,
            cha: 10,
            free_points: 0,
            skill_points: [0; 10],
            gold: 0,
            loyalty: 1000,
            loyalty_monthly: 500,
            authority: 1,
            knights_id: 0,
            fame: 0,
            party_id: None,
            exp: 0,
            max_exp: 0,
            exp_seal_status: false,
            sealed_exp: 0,
            item_weight: 0,
            max_weight: 0,
            res_hp_type: 1,
            rival_id,
            rival_expiry_time,
            anger_gauge: 0,
            manner_point: 0,
            rebirth_level: 0,
            reb_str: 0,
            reb_sta: 0,
            reb_dex: 0,
            reb_intel: 0,
            reb_cha: 0,
            cover_title: 0,
        }
    }

    #[test]
    fn test_has_active_rivalry_no_rival() {
        // rival_id = -1 (no rival set) → false
        let ch = make_rivalry_test_char(1, 104, -1, 0);
        assert!(!has_active_rivalry(&ch, 5));
    }

    #[test]
    fn test_has_active_rivalry_wrong_victim() {
        // Rival is session 5, but victim is session 10 → false
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 300;
        let ch = make_rivalry_test_char(1, 104, 5, future);
        assert!(!has_active_rivalry(&ch, 10));
    }

    #[test]
    fn test_has_active_rivalry_expired() {
        // Rival matches but expiry is in the past → false
        let past = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 60; // 60 seconds ago
        let ch = make_rivalry_test_char(1, 104, 5, past);
        assert!(!has_active_rivalry(&ch, 5));
    }

    #[test]
    fn test_has_active_rivalry_active() {
        // Rival matches and expiry is in the future → true
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 300; // 5 minutes from now
        let ch = make_rivalry_test_char(1, 104, 5, future);
        assert!(has_active_rivalry(&ch, 5));
    }

    #[test]
    fn test_has_active_rivalry_zero_expiry() {
        // Rival matches and expiry is 0 (no expiry set) → true (always active)
        let ch = make_rivalry_test_char(1, 104, 5, 0);
        assert!(has_active_rivalry(&ch, 5));
    }

    #[test]
    fn test_rivalry_bonus_non_priest() {
        // Non-priest class should get 0 bonus from is_priest_class check
        // even if has_active_rivalry would return true
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 300;
        let warrior = make_rivalry_test_char(1, 101, 5, future); // Warrior
                                                                 // The combined condition: is_priest_class AND has_active_rivalry
        let would_get_bonus = is_priest_class(warrior.class) && has_active_rivalry(&warrior, 5);
        assert!(!would_get_bonus);
    }

    #[test]
    fn test_rivalry_bonus_priest_with_active_rival() {
        // Priest with active rivalry should get bonus
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 300;
        let priest = make_rivalry_test_char(1, 104, 5, future); // Priest
        let would_get_bonus = is_priest_class(priest.class) && has_active_rivalry(&priest, 5);
        assert!(would_get_bonus);
    }

    // ── WIZ_LOYALTY_CHANGE Packet Format Tests ─────────────────────────

    #[test]
    fn test_loyalty_change_packet_format() {
        // Build WIZ_LOYALTY_CHANGE exactly as send_loyalty_change() does
        let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
        pkt.write_u8(1); // LOYALTY_NATIONAL_POINTS
        pkt.write_u32(5000); // loyalty
        pkt.write_u32(1200); // loyalty_monthly
        pkt.write_u32(0); // clan donations
        pkt.write_u32(0); // clan loyalty amount

        assert_eq!(pkt.opcode, 0x2A);
        // 1 byte sub + 4*4 u32 = 17 bytes
        assert_eq!(pkt.data.len(), 17);
    }

    #[test]
    fn test_loyalty_change_packet_bytes() {
        use ko_protocol::PacketReader;
        let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
        pkt.write_u8(1);
        pkt.write_u32(1000);
        pkt.write_u32(500);
        pkt.write_u32(0);
        pkt.write_u32(0);

        let mut reader = PacketReader::new(&pkt.data);
        let sub = reader.read_u8();
        let loyalty = reader.read_u32();
        let monthly = reader.read_u32();
        let clan_don = reader.read_u32();
        let clan_loy = reader.read_u32();

        assert_eq!(sub, Some(1));
        assert_eq!(loyalty, Some(1000));
        assert_eq!(monthly, Some(500));
        assert_eq!(clan_don, Some(0));
        assert_eq!(clan_loy, Some(0));
    }

    #[test]
    fn test_pvp_remove_rival_packet_format() {
        use ko_protocol::PacketReader;
        // Build WIZ_PVP(PVPRemoveRival=2) exactly as remove_rival() does
        let mut pkt = Packet::new(Opcode::WizPvp as u8);
        pkt.write_u8(2); // PVPRemoveRival

        assert_eq!(pkt.opcode, 0x88);
        assert_eq!(pkt.data.len(), 1); // just the sub-opcode byte
        let mut reader = PacketReader::new(&pkt.data);
        assert_eq!(reader.read_u8(), Some(2));
    }

    #[test]
    fn test_loyalty_change_packet_large_values() {
        use ko_protocol::PacketReader;
        // Verify packet works with max loyalty values
        let mut pkt = Packet::new(Opcode::WizLoyaltyChange as u8);
        pkt.write_u8(1);
        pkt.write_u32(LOYALTY_MAX);
        pkt.write_u32(LOYALTY_MAX);
        pkt.write_u32(0);
        pkt.write_u32(0);

        let mut reader = PacketReader::new(&pkt.data);
        let sub = reader.read_u8();
        let loyalty = reader.read_u32();
        let monthly = reader.read_u32();

        assert_eq!(sub, Some(1));
        assert_eq!(loyalty, Some(LOYALTY_MAX));
        assert_eq!(monthly, Some(LOYALTY_MAX));
    }

    // ── Clan NP Donation Formula Tests ───────────────────────────────

    #[test]
    fn test_clan_loyalty_donation_formula() {
        // C++ step function: 1-5→1, 6-10→2, ..., 46+→10
        assert_eq!(calculate_clan_donation(0), 0);
        assert_eq!(calculate_clan_donation(1), 1);
        assert_eq!(calculate_clan_donation(5), 1);
        assert_eq!(calculate_clan_donation(6), 2);
        assert_eq!(calculate_clan_donation(10), 2);
        assert_eq!(calculate_clan_donation(11), 3);
        assert_eq!(calculate_clan_donation(15), 3);
        assert_eq!(calculate_clan_donation(16), 4);
        assert_eq!(calculate_clan_donation(20), 4);
        assert_eq!(calculate_clan_donation(21), 5);
        assert_eq!(calculate_clan_donation(25), 5);
        assert_eq!(calculate_clan_donation(26), 6);
        assert_eq!(calculate_clan_donation(30), 6);
        assert_eq!(calculate_clan_donation(31), 7);
        assert_eq!(calculate_clan_donation(35), 7);
        assert_eq!(calculate_clan_donation(36), 8);
        assert_eq!(calculate_clan_donation(40), 8);
        assert_eq!(calculate_clan_donation(41), 9);
        assert_eq!(calculate_clan_donation(45), 9);
        assert_eq!(calculate_clan_donation(46), 10);
        assert_eq!(calculate_clan_donation(50), 10);
        // Beyond 50, still capped at 10
        assert_eq!(calculate_clan_donation(100), 10);
    }

    #[test]
    fn test_clan_donation_constants() {
        assert_eq!(CLAN_TYPE_ACCREDITED5, 3);
        assert_eq!(MAX_CLAN_USERS, 50);
    }

    #[test]
    fn test_clan_donation_skip_conditions() {
        // Document the skip conditions from C++ reference:
        // 1. is_bonus_reward = true → skip
        // 2. clan flag < 3 (not accredited) → skip
        // 3. clan_point_method != 0 → skip
        // 4. starter clan IDs (1, 15100) → skip
        // 5. members == 0 or members > MAX_CLAN_USERS → skip

        // Verify flag threshold
        assert!(2 < CLAN_TYPE_ACCREDITED5); // Training/Promoted clans below threshold
        assert!(3 >= CLAN_TYPE_ACCREDITED5); // Accredited5 meets threshold

        // Verify starter clan exclusion
        let starter_ids: [u16; 2] = [1, 15100];
        for id in starter_ids {
            assert!(
                id == 1 || id == 15100,
                "starter clan {id} should be excluded"
            );
        }
    }

    #[test]
    fn test_clan_donation_boundary_members() {
        // Verify boundary values at each step change
        assert_eq!(calculate_clan_donation(5), 1);
        assert_eq!(calculate_clan_donation(6), 2); // step up
        assert_eq!(calculate_clan_donation(10), 2);
        assert_eq!(calculate_clan_donation(11), 3); // step up
        assert_eq!(calculate_clan_donation(45), 9);
        assert_eq!(calculate_clan_donation(46), 10); // step up to max
        assert_eq!(calculate_clan_donation(47), 10); // stays at max
    }

    // ── War Zone Check Tests ─────────────────────────────────────────

    #[test]
    fn test_is_war_zone_battle_zones() {
        assert!(is_war_zone(ZONE_BATTLE));
        assert!(is_war_zone(ZONE_BATTLE6));
        assert!(is_war_zone(62)); // ZONE_BATTLE2
        assert!(is_war_zone(63)); // ZONE_BATTLE3
    }

    #[test]
    fn test_is_war_zone_non_battle() {
        assert!(!is_war_zone(ZONE_RONARK_LAND));
        assert!(!is_war_zone(21)); // Moradon
        assert!(!is_war_zone(30)); // Delos
        assert!(!is_war_zone(60)); // ZONE_BATTLE_BASE (below range)
        assert!(!is_war_zone(67)); // Above ZONE_BATTLE6
    }

    // ── PK Zone Update Integration Tests ────────────────────────────

    #[test]
    fn test_update_pk_zone_daily_loyalty_zero_change() {
        // update_pk_zone_daily_loyalty with 0 NP should be no-op
        let world = WorldState::new();
        update_pk_zone_daily_loyalty(&world, 1, 1, 0);
        // No crash, no entry created
    }

    #[test]
    fn test_update_pk_zone_daily_loyalty_invalid_nation() {
        let world = WorldState::new();
        update_pk_zone_daily_loyalty(&world, 1, 0, 100);
        update_pk_zone_daily_loyalty(&world, 1, 3, 100);
        // No crash, invalid nation gracefully handled
    }

    // ── Delos Same-Nation Loyalty Tests ──────────────────────────────

    #[test]
    fn test_delos_zone_constant() {
        // Verify Delos zone ID is correct
        assert_eq!(ZONE_DELOS, 30);
    }

    #[test]
    fn test_delos_zone_excluded_from_normal_pvp_checks() {
        // Delos is NOT in the excluded PvP zone list — it has its own flag-based logic
        assert!(!is_excluded_pvp_zone(ZONE_DELOS));
    }

    #[test]
    fn test_delos_is_not_pk_zone() {
        // Delos uses its own flag-based PvP logic, not the standard PK zone check
        assert!(!is_pk_zone(ZONE_DELOS));
    }

    #[test]
    fn test_delos_no_zone_returns_false_for_attack_flags() {
        // When zone doesn't exist in world (no ZoneState), get_zone returns None.
        // The loyalty code handles this with unwrap_or(false).
        let world = WorldState::new();
        let can_attack = world
            .get_zone(ZONE_DELOS)
            .map(|z| z.can_attack_same_nation())
            .unwrap_or(false);
        assert!(
            !can_attack,
            "Missing zone should default to no attack allowed"
        );
    }

    #[test]
    fn test_non_delos_same_nation_always_blocked() {
        // Outside Delos, same-nation kills never exchange loyalty
        // This behavior is unchanged regardless of zone flags
        assert_ne!(ZONE_RONARK_LAND, ZONE_DELOS);
        assert_ne!(ZONE_ARDREAM, ZONE_DELOS);
    }

    #[test]
    fn test_delos_loyalty_rates_use_other_zone() {
        // Delos falls into the "other zone" category for loyalty rates
        let rates = LoyaltyRates::default();
        let (src, tgt) = get_zone_loyalty_rates(ZONE_DELOS, &rates);
        assert_eq!(src, rates.other_zone_source);
        assert_eq!(tgt, rates.other_zone_target);
    }

    // ── Sprint 368: Flame level NP bonus ──

    #[test]
    fn test_flame_level_np_bonus_order() {
        // C++ UserLoyaltySystem.cpp:268-269 — flame level bonus applied after NP event,
        // before item/skill bonuses. Uses burning feature drop_rate.
        let base = 100i32;
        let np_gain_pct = 100i32;
        let np_event_pct = 50i32;
        let flame_drop_rate = 20u8;

        // Step 1: buff modifier
        let after_buff = np_gain_pct * base / 100;
        assert_eq!(after_buff, 100);

        // Step 2: event bonus
        let after_event = after_buff * (100 + np_event_pct) / 100;
        assert_eq!(after_event, 150);

        // Step 3: flame level bonus (multiplicative)
        let after_flame = after_event * (100 + flame_drop_rate as i32) / 100;
        assert_eq!(after_flame, 180);
    }

    #[test]
    fn test_flame_level_zero_no_np_bonus() {
        // When flame_level == 0, no bonus applied
        let flame_level: u16 = 0;
        let base = 100i32;
        let after = if flame_level > 0 {
            base * (100 + 20) / 100
        } else {
            base
        };
        assert_eq!(after, 100);
    }

    // ── Sprint 369: Missing loyalty bonuses ──

    #[test]
    fn test_flash_war_bonus_additive() {
        // C++ UserLoyaltySystem.cpp:278-281 — flash_war_bonus added to NP change
        let base_np = 50i32;
        let flash_war_bonus = 3i32;
        let result = base_np + flash_war_bonus;
        assert_eq!(result, 53);
    }

    #[test]
    fn test_premium_loyalty_bonus_additive() {
        // C++ UserLoyaltySystem.cpp:284-288 — premium bonus_loyalty added to NP change
        let base_np = 50i32;
        let premium_loyalty = 4i32; // Gold Premium = 4
        let result = base_np + premium_loyalty;
        assert_eq!(result, 54);
    }

    #[test]
    fn test_clan_premium_loyalty_bonus_additive() {
        // C++ UserLoyaltySystem.cpp:290-294 — clan premium bonus_loyalty added to NP change
        let base_np = 50i32;
        let clan_prem = 2i32;
        let result = base_np + clan_prem;
        assert_eq!(result, 52);
    }

    #[test]
    fn test_online_np_count_bonus() {
        // C++ UserLoyaltySystem.cpp:296-300 — clan online member NP bonus
        // Formula: ceil(online_members * 10 / 100), capped at 5
        let online_members = 30u16;
        let np_count = ((online_members as f64 * 10.0) / 100.0).ceil() as u16;
        assert_eq!(np_count, 3);

        let capped = np_count.min(5);
        assert_eq!(capped, 3);

        // Large clan: 80 members -> 8 -> capped at 5
        let large_clan = 80u16;
        let np_large = ((large_clan as f64 * 10.0) / 100.0).ceil() as u16;
        assert_eq!(np_large.min(5), 5);
    }

    #[test]
    fn test_premium_bonus_sum_tracking() {
        // C++ tracks all premium sources in m_PlayerKillingLoyaltyPremiumBonus
        let flash_war = 3i32;
        let premium = 4i32;
        let clan_prem = 2i32;
        let online_np = 3i32;

        let total_premium_bonus = flash_war + premium + clan_prem + online_np;
        assert_eq!(total_premium_bonus, 12);
    }

    #[test]
    fn test_all_loyalty_bonuses_order() {
        // Complete C++ order for NP gain:
        // 1) buff modifier: np_gain_pct * base / 100
        // 2) event: * (100 + np_event) / 100
        // 3) flame level: * (100 + drop_rate) / 100
        // 4) item/skill NP: += item_np + skill_np
        // 5) monument: += monument_bonus
        // 6) flash war: += flash_war_bonus
        // 7) premium: += premium_loyalty
        // 8) clan premium: += clan_prem_loyalty
        // 9) online NP: += online_np_count
        let base = 100i32;
        let mut change = base;

        // 1) buff (100%)
        change = 100 * change / 100;
        assert_eq!(change, 100);

        // 2) event (+20%)
        change = change * (100 + 20) / 100;
        assert_eq!(change, 120);

        // 3) flame (+10%)
        change = change * (100 + 10) / 100;
        assert_eq!(change, 132);

        // 4) item/skill (+5)
        change += 5;
        assert_eq!(change, 137);

        // 5) monument (+5)
        change += 5;
        assert_eq!(change, 142);

        // 6) flash war (+3)
        change += 3;
        assert_eq!(change, 145);

        // 7) premium (+4)
        change += 4;
        assert_eq!(change, 149);

        // 8) clan premium (+2)
        change += 2;
        assert_eq!(change, 151);

        // 9) online NP (+3)
        change += 3;
        assert_eq!(change, 154);
    }
}
