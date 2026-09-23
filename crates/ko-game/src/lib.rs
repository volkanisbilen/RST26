//! # ko-game
//!
//! Game logic, world state management, and packet handler dispatch
//! for the Knight Online Rust server.
//!
//! This crate implements:
//! - Game Server: WIZ_* packet handlers, client sessions (port 15001)
//! - Login Server: LS_* packet handlers, login sessions (port 15100)
//! - Shared packet I/O (framing, encryption)

pub mod attack_constants;
pub mod buff_constants;
pub mod clan_constants;
pub mod handler;
pub mod inventory_constants;
pub mod login_handler;
pub mod login_server;
pub mod login_session;
pub mod lua_engine;
pub mod magic_constants;
pub mod npc;
pub mod npc_type_constants;
pub mod object_event_constants;
pub mod packet_io;
pub mod pus_web;
pub mod race_constants;
pub mod rate_limiter;
pub mod server;
pub mod session;
pub mod state_change_constants;
pub mod systems;
pub mod world;
pub mod writer;
pub mod zone;
