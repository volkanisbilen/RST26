//! Lua quest scripting engine for Knight Online.
//! The quest system executes Lua 5.1 scripts stored in `./Quests/` to drive
//! NPC dialog, item rewards, zone changes, and other quest logic. Each script
//! is loaded once and cached as compiled bytecode for fast re-execution.
//! ## Architecture
//! - A fresh `mlua::Lua` VM is created **per execution** for thread safety.
//! - Binding functions are registered as Lua globals; they look up the player
//!   via the `UID` global and delegate to `WorldState` methods.
//! - Globals `UID`, `STEP`, `EVENT` are set before each script runs (matching C++).
//! - Script errors are logged with context (filename, UID, zone) and never panic.

pub mod bindings;

use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use mlua::prelude::*;
use tracing::{debug, warn};

use crate::world::WorldState;
use crate::zone::SessionId;

/// Lua quest script directory (relative to the server binary working directory).
const LUA_SCRIPT_DIRECTORY: &str = "./Quests/";

/// Lua quest scripting engine.
/// Manages script bytecode caching and per-execution VM creation.
pub struct LuaEngine {
    /// Directory containing `.lua` quest scripts.
    script_dir: PathBuf,
    /// Bytecode cache: filename -> compiled bytecode (Vec<u8>).
    ///
    script_cache: DashMap<String, Vec<u8>>,
}

impl LuaEngine {
    /// Create a new Lua engine with the default quest script directory.
    ///
    /// Logs a warning if the `./Quests/` directory is not found in the current
    /// working directory. This typically means the server should be run from the
    /// directory containing the `Quests/` folder.
    pub fn new() -> Self {
        let dir = PathBuf::from(LUA_SCRIPT_DIRECTORY);
        if !dir.is_dir() {
            warn!(
                "Lua quest script directory '{}' not found! \
                 Quest scripts will fail to load. \
                 Ensure the server is run from a directory containing 'Quests/'.",
                LUA_SCRIPT_DIRECTORY,
            );
        } else {
            let count = std::fs::read_dir(&dir)
                .map(|d| {
                    d.filter(|e| {
                        e.as_ref()
                            .map(|e| e.path().extension().is_some_and(|ext| ext == "lua"))
                            .unwrap_or(false)
                    })
                    .count()
                })
                .unwrap_or(0);
            debug!(
                "Lua quest script directory '{}' found ({} .lua files)",
                LUA_SCRIPT_DIRECTORY, count
            );
        }
        Self {
            script_dir: dir,
            script_cache: DashMap::new(),
        }
    }

    /// Create a Lua engine with a custom script directory (for testing).
    #[cfg(test)]
    pub fn with_script_dir(dir: PathBuf) -> Self {
        Self {
            script_dir: dir,
            script_cache: DashMap::new(),
        }
    }

    /// Execute a Lua quest script.
    ///
    /// # Arguments
    ///
    /// * `world` - Shared world state for binding function lookups.
    /// * `session_id` - The player's session ID (set as `UID` global).
    /// * `event_id` - The quest event ID (set as `EVENT` global).
    /// * `selected_reward` - The selected reward/step (set as `STEP` global).
    /// * `filename` - The Lua script filename (e.g., `"18005_Bilbor.lua"`).
    ///
    /// # Returns
    ///
    /// `true` if the script executed successfully, `false` on error.
    pub fn execute(
        &self,
        world: &Arc<WorldState>,
        session_id: SessionId,
        event_id: i32,
        selected_reward: i8,
        filename: &str,
    ) -> bool {
        // Build full path
        let full_path = self.script_dir.join(filename);

        // Get or compile bytecode
        let bytecode = match self.get_or_compile(filename, &full_path) {
            Some(bc) => bc,
            None => return false,
        };

        // Create a fresh Lua VM for this execution (thread-safe, no mutex needed)
        let lua = Lua::new();

        // Store world reference as app data so binding functions can access it
        lua.set_app_data(Arc::clone(world));

        // Register all binding functions
        if let Err(e) = bindings::register_all(&lua) {
            warn!(
                "Failed to register Lua bindings for script '{}': {}",
                filename, e
            );
            return false;
        }

        // Set globals matching C++ behavior
        if let Err(e) = Self::set_globals(&lua, session_id, event_id, selected_reward) {
            warn!("Failed to set Lua globals for script '{}': {}", filename, e);
            return false;
        }

        // Load and execute the bytecode
        match lua.load(&bytecode).set_name(filename).exec() {
            Ok(()) => {
                debug!(
                    "Lua script '{}' executed successfully (UID={}, EVENT={}, STEP={})",
                    filename, session_id, event_id, selected_reward
                );
                true
            }
            Err(e) => {
                // Log error with context matching C++ error reporting
                let zone_id = world
                    .get_position(session_id)
                    .map(|p| p.zone_id)
                    .unwrap_or(0);
                let user_name = world
                    .get_character_info(session_id)
                    .map(|c| c.name)
                    .unwrap_or_default();

                warn!(
                    "Lua script error: FILE='{}', USER='{}', ZONE={}, UID={}, EVENT={}: {}",
                    filename, user_name, zone_id, session_id, event_id, e
                );
                false
            }
        }
    }

    /// Set the Lua globals `UID`, `STEP`, `EVENT` before script execution.
    ///
    fn set_globals(
        lua: &Lua,
        session_id: SessionId,
        event_id: i32,
        selected_reward: i8,
    ) -> LuaResult<()> {
        let globals = lua.globals();
        globals.set("UID", session_id as i32)?;
        globals.set("STEP", selected_reward)?;
        globals.set("EVENT", event_id)?;
        Ok(())
    }

    /// Get compiled bytecode from cache, or compile and cache the script.
    ///
    fn get_or_compile(&self, filename: &str, full_path: &std::path::Path) -> Option<Vec<u8>> {
        // Check cache first
        if let Some(cached) = self.script_cache.get(filename) {
            return Some(cached.clone());
        }

        // Read and compile the script
        let source = match std::fs::read_to_string(full_path) {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    "Failed to load Lua script '{}' (path: {:?}): {}",
                    filename, full_path, e
                );
                return None;
            }
        };

        // Strip UTF-8 BOM if present (some quest files have BOM from Windows editors)
        let source = match source.strip_prefix('\u{FEFF}') {
            Some(stripped) => stripped.to_string(),
            None => source,
        };

        // Compile to bytecode using a temporary Lua state
        let compile_lua = Lua::new();
        let bytecode = match compile_lua
            .load(source.as_str())
            .set_name(filename)
            .into_function()
        {
            Ok(func) => {
                let v = func.dump(false);
                if v.is_empty() {
                    warn!("Failed to dump Lua script '{}' to bytecode", filename);
                    return None;
                }
                v
            }
            Err(e) => {
                warn!("Lua compile error in '{}': {}", filename, e);
                return None;
            }
        };

        // Cache the bytecode
        self.script_cache
            .insert(filename.to_string(), bytecode.clone());
        debug!("Compiled and cached Lua script '{}'", filename);

        Some(bytecode)
    }

    /// Clear the script bytecode cache.
    ///
    /// This forces all Lua scripts to be re-read and re-compiled on their
    /// next execution. Useful for hot-reloading quest scripts during development.
    pub fn invalidate_cache(&self) -> usize {
        let count = self.script_cache.len();
        self.script_cache.clear();
        count
    }
}

impl Default for LuaEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_engine_creation() {
        let engine = LuaEngine::new();
        assert_eq!(engine.script_dir, PathBuf::from("./Quests/"));
        assert!(engine.script_cache.is_empty());
    }

    #[test]
    fn test_lua_engine_custom_dir() {
        let engine = LuaEngine::with_script_dir(PathBuf::from("/tmp/quests"));
        assert_eq!(engine.script_dir, PathBuf::from("/tmp/quests"));
    }

    #[test]
    fn test_lua_engine_missing_script() {
        let engine = LuaEngine::with_script_dir(PathBuf::from("./nonexistent_dir/"));
        let result = engine.get_or_compile(
            "missing.lua",
            &PathBuf::from("./nonexistent_dir/missing.lua"),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_set_globals() {
        let lua = Lua::new();
        LuaEngine::set_globals(&lua, 42, 100, 3).unwrap();

        let globals = lua.globals();
        assert_eq!(globals.get::<i32>("UID").unwrap(), 42);
        assert_eq!(globals.get::<i32>("EVENT").unwrap(), 100);
        assert_eq!(globals.get::<i8>("STEP").unwrap(), 3);
    }

    #[test]
    fn test_bytecode_cache() {
        let dir = std::env::temp_dir().join("ko_lua_test_cache");
        std::fs::create_dir_all(&dir).unwrap();
        let script_path = dir.join("test_cache.lua");
        std::fs::write(&script_path, "-- test\nlocal x = 1\n").unwrap();

        let engine = LuaEngine::with_script_dir(dir.clone());

        // First call compiles and caches
        let bc1 = engine.get_or_compile("test_cache.lua", &script_path);
        assert!(bc1.is_some());
        assert_eq!(engine.script_cache.len(), 1);

        // Second call returns cached
        let bc2 = engine.get_or_compile("test_cache.lua", &script_path);
        assert!(bc2.is_some());
        assert_eq!(bc1.unwrap(), bc2.unwrap());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compile_invalid_script() {
        let dir = std::env::temp_dir().join("ko_lua_test_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let script_path = dir.join("bad.lua");
        std::fs::write(&script_path, "this is not valid lua {{{{").unwrap();

        let engine = LuaEngine::with_script_dir(dir.clone());

        let result = engine.get_or_compile("bad.lua", &script_path);
        assert!(result.is_none());
        assert!(engine.script_cache.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_all_workspace_quest_scripts_compile() {
        let quest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("Quests");
        let lua = Lua::new();
        let mut failures = Vec::new();

        for entry in std::fs::read_dir(&quest_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("lua") {
                continue;
            }

            let source = std::fs::read_to_string(&path).unwrap();
            if let Err(error) = lua
                .load(&source)
                .set_name(path.display().to_string())
                .into_function()
            {
                failures.push(format!("{}: {error}", path.display()));
            }
        }

        assert!(
            failures.is_empty(),
            "quest Lua syntax errors:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn test_lua_basic_execution() {
        let lua = Lua::new();
        lua.globals().set("UID", 1).unwrap();
        lua.globals().set("STEP", -1i8).unwrap();
        lua.globals().set("EVENT", 500).unwrap();

        let result = lua
            .load("local uid = UID; local step = STEP; local event = EVENT")
            .exec();
        assert!(result.is_ok());
    }

    #[test]
    fn test_lua_function_registration() {
        let lua = Lua::new();

        let check_level = lua.create_function(|_, uid: i32| Ok(uid * 2)).unwrap();
        lua.globals().set("CheckLevel", check_level).unwrap();

        let result: i32 = lua.load("return CheckLevel(21)").eval().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_default_trait() {
        let engine = LuaEngine::default();
        assert_eq!(engine.script_dir, PathBuf::from("./Quests/"));
    }

    #[test]
    fn test_invalidate_cache_empty() {
        let engine = LuaEngine::new();
        assert_eq!(engine.invalidate_cache(), 0);
    }

    #[test]
    fn test_invalidate_cache_with_scripts() {
        let dir = std::env::temp_dir().join("ko_lua_test_invalidate");
        std::fs::create_dir_all(&dir).unwrap();
        let script1 = dir.join("s1.lua");
        let script2 = dir.join("s2.lua");
        std::fs::write(&script1, "local x = 1").unwrap();
        std::fs::write(&script2, "local y = 2").unwrap();

        let engine = LuaEngine::with_script_dir(dir.clone());

        // Compile two scripts
        assert!(engine.get_or_compile("s1.lua", &script1).is_some());
        assert!(engine.get_or_compile("s2.lua", &script2).is_some());
        assert_eq!(engine.script_cache.len(), 2);

        // Invalidate clears all and returns count
        let cleared = engine.invalidate_cache();
        assert_eq!(cleared, 2);
        assert!(engine.script_cache.is_empty());

        // Re-compile works after invalidation
        assert!(engine.get_or_compile("s1.lua", &script1).is_some());
        assert_eq!(engine.script_cache.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn all_quest_lua_files_compile() {
        let quest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("Quests");
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(&quest_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("lua") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let lua = Lua::new();
            if let Err(error) = lua
                .load(&source)
                .set_name(path.to_string_lossy())
                .into_function()
            {
                failures.push(format!("{}: {}", path.display(), error));
            }
        }
        assert!(
            failures.is_empty(),
            "Lua compile failures:\n{}",
            failures.join("\n")
        );
    }
}
