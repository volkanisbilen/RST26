//! Background game systems — regen ticks, time, weather, buff expiry, NPC AI.
//!
//! Each system runs as a background tokio task, started when the game server
//! initialises. They iterate the shared `WorldState` periodically.

pub mod auto_harvest;
pub mod bdw;
pub mod bot_ai;
pub mod bot_brain;
pub mod bot_waypoints;
pub mod buff_tick;
pub mod chaos;
pub mod chaos_arena;
pub mod chaos_stone_tick;
pub mod character_save;
pub mod concurrent_update;
pub mod daily_reset;
pub mod dot_tick;
pub mod event_bots;
pub mod event_bots_juraid;
pub mod event_nav;
pub mod event_room;
pub mod event_system;
pub mod expiry_tick;
pub mod field_bots;
pub mod flash;
pub mod heartbeat_probe;
pub mod juraid;
pub mod knights_save;
pub mod letter_admin_notify;
pub mod loyalty;
pub mod manes_survival;
pub mod monster_stone;
pub mod nav_trace;
pub mod npc_ai;
pub mod offline_merchant;
pub mod pathfind;
pub mod pet_attack_tick;
pub mod pet_tick;
pub mod regen;
pub mod sp_regen;
pub mod time_weather;
pub mod timed_notice;
pub mod voice_relay;
pub mod war;
pub mod zone_level_kick_tick;
pub mod zone_rewards;
