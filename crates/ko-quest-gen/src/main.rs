//! Quest event handler generator — generates missing Lua EVENT handlers
//! from quest_helper data (MSSQL dump + TBL files).
//!
//! Usage:
//!   ko-quest-gen --data-dir <client_data> --quest-dir <lua_dir> --mssql-helper <dump_file>
//!
//! Examples:
//!   # Dry-run: show what would be generated
//!   ko-quest-gen -d Data -q Quests -m docs/mssql_quest_helper_full.txt --dry-run
//!
//!   # Apply generated handlers to Lua files
//!   ko-quest-gen -d Data -q Quests -m docs/mssql_quest_helper_full.txt
//!
//!   # Generate for a specific NPC only
//!   ko-quest-gen -d Data -q Quests -m docs/mssql_quest_helper_full.txt --npc 11810

// Tool crate — many fields are kept for future use and debugging output.
#![allow(dead_code)]

mod analyzer;
mod generator;
mod mssql_parser;

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "ko-quest-gen", about = "Quest event handler generator")]
struct Args {
    /// Client Data directory containing TBL files.
    #[arg(short = 'd', long, env = "KO_DATA_DIR")]
    data_dir: PathBuf,

    /// Quest Lua scripts directory.
    #[arg(short = 'q', long, env = "KO_QUEST_DIR")]
    quest_dir: PathBuf,

    /// Path to MSSQL quest_helper dump (pipe-delimited). Optional when
    /// --tbl-only is used.
    #[arg(short = 'm', long)]
    mssql_helper: Option<PathBuf>,

    /// Use Quest_Helper.tbl as the authoritative source. This is the correct
    /// mode for the v2615 client; old MSSQL rows may target older clients.
    #[arg(long)]
    tbl_only: bool,

    /// Path to MSSQL item_exchange dump (pipe-delimited, optional).
    #[arg(long)]
    mssql_exchange: Option<PathBuf>,

    /// Only generate for specific NPC. Can be repeated.
    #[arg(long)]
    npc: Vec<i32>,

    /// Dry-run: show generated code without writing files.
    #[arg(long)]
    dry_run: bool,

    /// Show detailed info about each generated handler.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "ko_quest_gen=info,ko_quest_audit=info,ko_tbl_import=warn"
                    .parse()
                    .unwrap()
            }),
        )
        .with_target(false)
        .init();

    let args = Args::parse();

    // Validate directories
    if !args.data_dir.is_dir() {
        anyhow::bail!("Data directory not found: {}", args.data_dir.display());
    }
    if !args.quest_dir.is_dir() {
        anyhow::bail!("Quest directory not found: {}", args.quest_dir.display());
    }
    if !args.tbl_only {
        let Some(path) = args.mssql_helper.as_ref() else {
            anyhow::bail!("--mssql-helper is required unless --tbl-only is used");
        };
        if !path.is_file() {
            anyhow::bail!("MSSQL helper dump not found: {}", path.display());
        }
    }

    // 1. Load TBL data (for quest_talk, quest_menu, item_exchange lookups)
    tracing::info!("Loading TBL files from {}...", args.data_dir.display());
    let tbl = ko_quest_audit::tbl_loader::TblData::load(&args.data_dir)?;

    // 2. Parse MSSQL quest_helper dump + merge TBL-only rows. For v2615 the
    // client TBL is authoritative, so --tbl-only intentionally starts empty.
    let mut mssql_rows = if args.tbl_only {
        tracing::info!("Using Quest_Helper.tbl as authoritative v2615 source");
        Vec::new()
    } else {
        tracing::info!("Parsing MSSQL dump...");
        mssql_parser::parse_quest_helper_dump(args.mssql_helper.as_ref().unwrap())?
    };

    // Merge TBL quest_helper rows that are NOT in the MSSQL dump.
    // Uses (npc_id, quest_id, status) as key to avoid duplicates.
    let mssql_keys: std::collections::HashSet<(i32, i32, i32)> = mssql_rows
        .iter()
        .map(|r| (r.s_npc_id, r.s_event_data_index, r.b_event_status))
        .collect();
    let mut tbl_only_count = 0;
    for th in &tbl.quest_helpers {
        let key = (th.npc_id, th.event_data_index, th.event_status);
        if !mssql_keys.contains(&key) {
            mssql_rows.push(mssql_parser::MssqlQuestHelper {
                n_index: th.index,
                b_message_type: th.msg_type,
                b_level: th.min_level,
                n_exp: th.exp,
                b_class: th.class,
                b_nation: th.nation,
                b_quest_type: th.quest_type,
                b_zone: th.zone,
                s_npc_id: th.npc_id,
                s_event_data_index: th.event_data_index,
                b_event_status: th.event_status,
                n_event_trigger_index: th.event_trigger,
                n_event_complete_index: th.event_complete,
                n_exchange_index: th.exchange_index,
                n_event_talk_index: th.event_talk,
                str_lua_filename: th.lua_filename.clone(),
                s_quest_menu: th.quest_menu,
                s_npc_main: th.npc_main,
                s_quest_solo: th.quest_solo,
            });
            tbl_only_count += 1;
        }
    }
    if tbl_only_count > 0 {
        tracing::info!(rows = tbl_only_count, "Merged TBL-only quest_helper rows");
    }

    // Filter by NPC if requested
    if !args.npc.is_empty() {
        mssql_rows.retain(|r| args.npc.contains(&r.s_npc_id));
        tracing::info!(
            rows = mssql_rows.len(),
            "Filtered to {} NPCs",
            args.npc.len()
        );
    }

    // 3. Parse existing Lua files
    tracing::info!("Parsing Lua files from {}...", args.quest_dir.display());
    let lua_files = ko_quest_audit::lua_parser::load_all_lua(&args.quest_dir)?;

    // 4. Analyze and find missing handlers
    tracing::info!("Analyzing quest chains...");
    let missing = analyzer::find_missing_handlers(&mssql_rows, &lua_files, &tbl);

    if missing.is_empty() {
        println!("No missing handlers found — all quest EVENTs are covered!");
        return Ok(());
    }

    // Summary
    let mut by_file: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for h in &missing {
        *by_file.entry(h.lua_filename.clone()).or_default() += 1;
    }

    println!("\n=== MISSING HANDLER SUMMARY ===");
    println!("Total missing handlers: {}", missing.len());
    println!("Affected files: {}", by_file.len());

    if args.verbose {
        println!("\nPer-file breakdown:");
        let mut sorted: Vec<_> = by_file.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (file, count) in &sorted {
            println!("  {} → {} handlers", file, count);
        }
        println!();

        for h in &missing {
            println!(
                "  [{}] EVENT {} — {:?} (quest={}, status={})",
                h.lua_filename,
                h.event_id,
                std::mem::discriminant(&h.handler_type),
                h.source_row.s_event_data_index,
                h.source_row.b_event_status,
            );
        }
    }

    // 5. Generate code
    tracing::info!("Generating Lua code...");
    let generated = generator::generate_all(&missing, &tbl);

    // 6. Apply or dry-run
    let mode = if args.dry_run { "DRY-RUN" } else { "APPLY" };
    tracing::info!(mode, "Writing generated handlers...");
    let modified = generator::apply_generated(&args.quest_dir, &generated, args.dry_run)?;

    println!("\n=== RESULT ===");
    println!("{}: {} files with generated handlers", mode, modified);

    Ok(())
}
