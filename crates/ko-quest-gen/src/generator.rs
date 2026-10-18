//! Generates Lua EVENT handler code from analyzed quest data.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;
use std::path::Path;

use ko_quest_audit::tbl_loader::TblData;

use crate::analyzer::{HandlerType, MissingHandler};

/// Generate Lua code for a missing handler.
pub fn generate_handler(handler: &MissingHandler, tbl: &TblData) -> String {
    let mut code = String::new();

    // Add comment with context
    let _ = writeln!(
        code,
        "-- [AUTO-GEN] quest={} status={} n_index={}",
        handler.source_row.s_event_data_index,
        handler.source_row.b_event_status,
        handler.source_row.n_index,
    );

    let _ = writeln!(code, "if (EVENT == {}) then", handler.event_id);

    match &handler.handler_type {
        HandlerType::IntroAccept { save_event_index } => {
            let _ = writeln!(code, "\tSaveEvent(UID, {});", save_event_index);
        }

        HandlerType::QuestOffer {
            quest_id,
            event_talk,
            quest_menu,
            accept_event,
        } => {
            let accept_text = if *quest_menu > 0 { *quest_menu } else { 22 };
            let _ = writeln!(
                code,
                "\tSelectMsg(UID, 4, {}, {}, NPC, {}, {}, 23, -1);",
                quest_id, event_talk, accept_text, accept_event,
            );
        }

        HandlerType::QuestAccept { save_event_index } => {
            let _ = writeln!(code, "\tSaveEvent(UID, {});", save_event_index);
        }

        HandlerType::ProgressCheck {
            quest_id,
            event_talk,
            exchange_index,
            has_monster_quest,
            complete_event,
            incomplete_event,
        } => {
            if *has_monster_quest {
                let _ = writeln!(code, "\tMonsterSub = ExistMonsterQuestSub(UID);");
                let _ = writeln!(code, "\tif (MonsterSub == 0) then");
                let _ = writeln!(
                    code,
                    "\t\tSelectMsg(UID, 4, {}, {}, NPC, 22, {}, 23, -1);",
                    quest_id, event_talk, complete_event,
                );
                let _ = writeln!(code, "\telse");
                let _ = writeln!(
                    code,
                    "\t\tSelectMsg(UID, 2, {}, {}, NPC, 18, {});",
                    quest_id, event_talk, incomplete_event,
                );
                let _ = writeln!(code, "\tend");
            } else if *exchange_index > 0 {
                if let Some(exchange) = tbl.item_exchanges.get(exchange_index) {
                    if let Some(&(item_id, count)) = exchange.origin_items.first() {
                        let _ = writeln!(code, "\tItemA = HowmuchItem(UID, {});", item_id);
                        let _ = writeln!(code, "\tif (ItemA < {}) then", count);
                        let _ = writeln!(
                            code,
                            "\t\tSelectMsg(UID, 2, {}, {}, NPC, 18, {});",
                            quest_id, event_talk, incomplete_event,
                        );
                        let _ = writeln!(code, "\telse");
                        let _ = writeln!(
                            code,
                            "\t\tSelectMsg(UID, 4, {}, {}, NPC, 41, {}, 27, -1);",
                            quest_id, event_talk, complete_event,
                        );
                        let _ = writeln!(code, "\tend");
                    } else {
                        let _ = writeln!(
                            code,
                            "\tSelectMsg(UID, 4, {}, {}, NPC, 41, {}, 27, -1);",
                            quest_id, event_talk, complete_event,
                        );
                    }
                } else {
                    let _ = writeln!(
                        code,
                        "\tSelectMsg(UID, 2, {}, {}, NPC, 10, -1);",
                        quest_id, event_talk,
                    );
                }
            } else {
                let _ = writeln!(
                    code,
                    "\tSelectMsg(UID, 2, {}, {}, NPC, 10, -1);",
                    quest_id, event_talk,
                );
            }
        }

        HandlerType::QuestComplete {
            quest_id,
            exchange_index,
            save_event_index,
        } => {
            let _ = writeln!(
                code,
                "\tQuestStatusCheck = GetQuestStatus(UID, {})",
                quest_id
            );
            let _ = writeln!(code, "\tif(QuestStatusCheck == 2) then");
            let _ = writeln!(code, "\t\tSelectMsg(UID, 2, -1, 8779, NPC, 10, -1);");
            let _ = writeln!(code, "\telse");
            if *exchange_index > 0 {
                let _ = writeln!(
                    code,
                    "\t\tif (RunQuestExchange(UID, {})) then",
                    exchange_index
                );
                let _ = writeln!(code, "\t\t\tSaveEvent(UID, {});", save_event_index);
                let _ = writeln!(code, "\t\tend");
            } else {
                let _ = writeln!(code, "\t\tSaveEvent(UID, {});", save_event_index);
            }
            let _ = writeln!(code, "\tend");
        }

        HandlerType::CompletionConfirm {
            quest_id,
            exchange_index,
            save_event_index,
        } => {
            // Status=2 handler — quest exchange was already run.
            // If exchange_index > 0, this may be a repeatable exchange path.
            let _ = writeln!(
                code,
                "\tQuestStatusCheck = GetQuestStatus(UID, {})",
                quest_id
            );
            let _ = writeln!(code, "\tif(QuestStatusCheck == 2) then");
            let _ = writeln!(code, "\t\tSelectMsg(UID, 2, -1, 8779, NPC, 10, -1);");
            let _ = writeln!(code, "\telse");
            if *exchange_index > 0 {
                let _ = writeln!(
                    code,
                    "\t\tif (RunQuestExchange(UID, {})) then",
                    exchange_index
                );
                let _ = writeln!(code, "\t\t\tSaveEvent(UID, {});", save_event_index);
                let _ = writeln!(code, "\t\tend");
            } else {
                let _ = writeln!(code, "\t\tSaveEvent(UID, {});", save_event_index);
            }
            let _ = writeln!(code, "\tend");
        }

        HandlerType::SubStep {
            quest_id,
            event_talk,
        } => {
            if *event_talk > 0 {
                let _ = writeln!(
                    code,
                    "\tSelectMsg(UID, 2, {}, {}, NPC, 10, -1);",
                    quest_id, event_talk,
                );
            } else {
                let _ = writeln!(code, "\tSelectMsg(UID, 2, -1, 331, NPC, 10, -1);");
            }
        }

        HandlerType::QuestDone {
            quest_id,
            event_talk,
        } => {
            if *event_talk > 0 {
                let _ = writeln!(
                    code,
                    "\tSelectMsg(UID, 2, {}, {}, NPC, 10, -1);",
                    quest_id, event_talk,
                );
            } else {
                let _ = writeln!(code, "\tSelectMsg(UID, 2, -1, 331, NPC, 10, -1);");
            }
        }

        HandlerType::ShowMap { zone } => {
            let _ = writeln!(code, "\tShowMap(UID, {});", zone);
        }

        HandlerType::SearchQuestDispatch { npc_id } => {
            let _ = writeln!(code, "\tSearchQuest(UID, {});", npc_id);
        }
    }

    let _ = writeln!(code, "end");
    code
}

/// Group missing handlers by Lua filename and generate combined output.
pub fn generate_all(handlers: &[MissingHandler], tbl: &TblData) -> BTreeMap<String, String> {
    let mut by_file: BTreeMap<String, Vec<&MissingHandler>> = BTreeMap::new();
    for h in handlers {
        by_file.entry(h.lua_filename.clone()).or_default().push(h);
    }

    let mut result = BTreeMap::new();
    for (filename, file_handlers) in &by_file {
        let mut code = String::new();
        let _ = writeln!(code);
        let _ = writeln!(
            code,
            "-- ═══════════════════════════════════════════════════════════════════"
        );
        let _ = writeln!(code, "-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)");
        let _ = writeln!(
            code,
            "-- ═══════════════════════════════════════════════════════════════════"
        );
        let _ = writeln!(code);

        // Sort handlers by event_id
        let mut sorted: Vec<&&MissingHandler> = file_handlers.iter().collect();
        sorted.sort_by_key(|h| h.event_id);

        // Deduplicate by event_id
        let mut seen_events = HashSet::new();
        for handler in sorted {
            if seen_events.insert(handler.event_id) {
                let handler_code = generate_handler(handler, tbl);
                let _ = writeln!(code, "{}", handler_code);
            }
        }

        result.insert(filename.clone(), code);
    }

    tracing::info!(
        files = result.len(),
        "Generated code for {} files",
        result.len()
    );
    result
}

/// Apply generated code to existing Lua files (append mode).
/// Returns the number of files modified.
pub fn apply_generated(
    quest_dir: &Path,
    generated: &BTreeMap<String, String>,
    dry_run: bool,
) -> anyhow::Result<usize> {
    let mut modified = 0;

    for (filename, code) in generated {
        let path = quest_dir.join(filename);
        if !path.exists() {
            tracing::warn!(file = %filename, "Lua file does not exist, skipping");
            continue;
        }

        if dry_run {
            println!(
                "--- {} ({} new handlers) ---",
                filename,
                count_handlers(code)
            );
            print!("{}", code);
            modified += 1;
            continue;
        }

        // Read existing content
        let existing = std::fs::read_to_string(&path)?;

        // Append only events proven missing. Keeping handlers from earlier
        // passes makes shared-trigger generation convergent instead of making
        // successive runs replace (and lose) one another's event groups.
        let mut new_content = existing;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(code);

        std::fs::write(&path, &new_content)?;
        let handler_count = count_handlers(code);
        tracing::info!(file = %filename, handlers = handler_count, "Appended generated handlers");
        modified += 1;
    }

    Ok(modified)
}

/// Count the number of EVENT handlers in generated code.
fn count_handlers(code: &str) -> usize {
    code.lines()
        .filter(|l| l.starts_with("if (EVENT =="))
        .count()
}

/// Strip existing auto-generated block from Lua content.
/// Returns content before the auto-gen marker (trimmed).
fn strip_autogen_block(content: &str) -> String {
    // Find the blank line + separator that starts the auto-gen block
    if let Some(pos) = content.find("\n-- ═══════════════════════════════════════════════════════════════════\n-- AUTO-GENERATED EVENT HANDLERS") {
        let trimmed = content[..pos].trim_end();
        format!("{}\n", trimmed)
    } else if content.contains("AUTO-GENERATED EVENT HANDLERS") {
        // Fallback: find just the marker line
        let mut lines: Vec<&str> = Vec::new();
        let mut hit_marker = false;
        for line in content.lines() {
            if line.contains("AUTO-GENERATED EVENT HANDLERS") {
                hit_marker = true;
            }
            // Keep lines before the separator preceding the marker
            if !hit_marker {
                lines.push(line);
            }
        }
        // Remove trailing separator line if present
        while lines.last().is_some_and(|l| l.starts_with("-- ═══")) {
            lines.pop();
        }
        // Remove trailing empty lines
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        let mut result = lines.join("\n");
        result.push('\n');
        result
    } else {
        content.to_string()
    }
}
