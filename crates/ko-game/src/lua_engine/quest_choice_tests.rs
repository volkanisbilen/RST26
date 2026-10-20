// Regression tests for reward-choice quests (included into `bindings::tests`).
//
// Rows follow the client tables: `random_flag` 11/12 = choose one slot, the exp row is paid on top.

use ko_db::models::item_tables::ItemExchangeExpRow as ChoiceExpRow;
use ko_db::models::QuestHelperRow as ChoiceHelperRow;

fn choice_exp_row(n_index: i32, flag: i16, slots: &[(i32, i32)]) -> ChoiceExpRow {
    let s = |i: usize| slots.get(i).copied().unwrap_or((0, 0));
    ChoiceExpRow {
        n_index,
        random_flag: Some(flag),
        exchange_item_num1: Some(s(0).0),
        exchange_item_count1: Some(s(0).1),
        exchange_item_num2: Some(s(1).0),
        exchange_item_count2: Some(s(1).1),
        exchange_item_num3: Some(s(2).0),
        exchange_item_count3: Some(s(2).1),
        exchange_item_num4: Some(s(3).0),
        exchange_item_count4: Some(s(3).1),
        exchange_item_num5: Some(s(4).0),
        exchange_item_count5: Some(s(4).1),
        exchange_item_time1: Some(0),
        exchange_item_time2: Some(0),
        exchange_item_time3: Some(0),
        exchange_item_time4: Some(0),
        exchange_item_time5: Some(0),
    }
}

fn choice_helper(n_index: i32, quest: i16, status: i16) -> ChoiceHelperRow {
    ChoiceHelperRow {
        n_index,
        b_message_type: 2,
        b_level: 1,
        n_exp: 0,
        b_class: 5,
        b_nation: 3,
        b_quest_type: 1,
        b_zone: 21,
        s_npc_id: 1,
        s_event_data_index: quest,
        b_event_status: status,
        n_event_trigger_index: 0,
        n_event_complete_index: 0,
        n_exchange_index: 0,
        n_event_talk_index: 0,
        str_lua_filename: "x.lua".into(),
        s_quest_menu: -1,
        s_npc_main: 0,
        s_quest_solo: 0,
    }
}

/// Karus warrior beginner (class 101) with the three class items; returns (world, items).
fn choice_world() -> (Lua, Arc<WorldState>, mpsc::UnboundedReceiver<Arc<Packet>>, [u32; 3]) {
    let (lua, world, rx) = setup_lua_world_with_rx();
    let items = [389_110_000u32, 389_120_000, 389_130_000];
    for id in items {
        insert_test_item(&world, id, false);
    }
    world.update_session(1, |h| h.by_selected_reward = -1);
    (lua, world, rx, items)
}

fn pick(world: &WorldState, index: i8) {
    world.update_session(1, |h| {
        h.by_selected_reward = index;
        h.select_msg_flag = 5;
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_choice_row_pays_only_the_picked_item() {
    for index in 0..3i8 {
        let (lua, world, _rx, items) = choice_world();
        let mut row = quest_exchange_row(
            9101,
            &[],
            &[(items[0] as i32, 1), (items[1] as i32, 1), (items[2] as i32, 1)],
        );
        row.random_flag = 12;
        world.insert_item_exchange(9101, row);
        pick(&world, index);
        let ok: bool = lua.load("return RunQuestExchange(1, 9101)").eval().unwrap();
        assert!(ok, "pick {index}");
        for (n, id) in items.iter().enumerate() {
            let expected = u32::from(n as i8 == index);
            assert_eq!(count_in_bag(&world, *id), expected, "pick {index} item {n}");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_choice_row_with_exp_row_pays_item_and_exp() {
    let (lua, world, _rx, items) = choice_world();
    let mut row = quest_exchange_row(
        9102,
        &[],
        &[(items[0] as i32, 1), (items[1] as i32, 1), (items[2] as i32, 1)],
    );
    row.random_flag = 11;
    world.insert_item_exchange(9102, row);
    // exp row: one exp slot, flag 10 (the client lists the exp part for every pick)
    world.insert_test_item_exchange_exp(choice_exp_row(9102, 10, &[(ITEM_EXP as i32, 675_000)]));
    let exp_before = world.get_character_info(1).unwrap().exp;
    pick(&world, 2);
    let ok: bool = lua.load("return RunQuestExchange(1, 9102)").eval().unwrap();
    assert!(ok);
    assert_eq!(count_in_bag(&world, items[0]), 0);
    assert_eq!(count_in_bag(&world, items[1]), 0);
    assert_eq!(count_in_bag(&world, items[2]), 1);
    let ch = world.get_character_info(1).unwrap();
    assert!(
        ch.exp > exp_before || ch.level > 30,
        "the exp row must be paid together with the item (exp {} -> {})",
        exp_before,
        ch.exp
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_exp_only_choice_gives_exp_and_no_item() {
    let (lua, world, _rx, items) = choice_world();
    // item row without items, exp row: choose between two national point amounts
    world.insert_item_exchange(9103, quest_exchange_row(9103, &[], &[]));
    world.insert_test_item_exchange_exp(choice_exp_row(
        9103,
        11,
        &[(ITEM_COUNT as i32, 50), (ITEM_COUNT as i32, 100)],
    ));
    let np_before = world.get_character_info(1).unwrap().loyalty;
    pick(&world, 1);
    let ok: bool = lua.load("return RunQuestExchange(1, 9103)").eval().unwrap();
    assert!(ok);
    assert_eq!(world.get_character_info(1).unwrap().loyalty, np_before + 100);
    assert_eq!(count_in_bag(&world, items[0]), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_fixed_row_with_item_and_exp_pays_both() {
    let (lua, world, _rx, items) = choice_world();
    let row = quest_exchange_row(9104, &[], &[(items[0] as i32, 1), (ITEM_EXP as i32, 500_000)]);
    world.insert_item_exchange(9104, row);
    let exp_before = world.get_character_info(1).unwrap().exp;
    let ok: bool = lua.load("return RunQuestExchange(1, 9104)").eval().unwrap();
    assert!(ok);
    assert_eq!(count_in_bag(&world, items[0]), 1);
    let ch = world.get_character_info(1).unwrap();
    assert!(ch.exp > exp_before || ch.level > 30);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_choice_without_pick_pays_the_class_variant_only() {
    let (lua, world, _rx, items) = choice_world();
    // item[1] is for rogues only (class 2): the warrior has to get item[0] / item[2]
    let mut it = world.get_item(items[1]).unwrap();
    it.class = Some(2);
    world.insert_item(items[1], it);
    let mut it = world.get_item(items[2]).unwrap();
    it.class = Some(3);
    world.insert_item(items[2], it);
    let mut it = world.get_item(items[0]).unwrap();
    it.class = Some(1);
    world.insert_item(items[0], it);
    let mut row = quest_exchange_row(
        9105,
        &[],
        &[(items[0] as i32, 1), (items[1] as i32, 1), (items[2] as i32, 1)],
    );
    row.random_flag = 12;
    world.insert_item_exchange(9105, row);
    // a plain dialog: no pick was sent
    let ok: bool = lua.load("return RunQuestExchange(1, 9105)").eval().unwrap();
    assert!(ok);
    assert_eq!(count_in_bag(&world, items[0]), 1);
    assert_eq!(count_in_bag(&world, items[1]), 0);
    assert_eq!(count_in_bag(&world, items[2]), 0);
    // a pick that points at another class gives the player's class variant instead
    let (lua, world, _rx, items) = choice_world();
    let mut it = world.get_item(items[0]).unwrap();
    it.class = Some(1);
    world.insert_item(items[0], it);
    let mut it = world.get_item(items[1]).unwrap();
    it.class = Some(2);
    world.insert_item(items[1], it);
    let mut row = quest_exchange_row(9106, &[], &[(items[0] as i32, 1), (items[1] as i32, 1)]);
    row.random_flag = 12;
    world.insert_item_exchange(9106, row);
    pick(&world, 1);
    let ok: bool = lua.load("return RunQuestExchange(1, 9106)").eval().unwrap();
    assert!(ok);
    assert_eq!(count_in_bag(&world, items[0]), 1);
    assert_eq!(count_in_bag(&world, items[1]), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_refused_item_reports_and_keeps_the_quest_open() {
    let (lua, world, mut rx, items) = choice_world();
    let origin = 389_140_000u32;
    insert_test_item(&world, origin, false);
    fill_bag_with_junk(&world);
    let mut inv = world.get_inventory(1);
    inv[WorldState::SLOT_MAX].item_id = origin;
    inv[WorldState::SLOT_MAX].count = 1;
    world.set_inventory(1, inv);
    // turn-in 1 item, reward: choose one item + exp; the freed slot holds the item, so make the
    // reward need two slots (two fixed items) to force the refusal
    let row = quest_exchange_row(
        9107,
        &[(origin as i32, 1)],
        &[(items[0] as i32, 1), (items[1] as i32, 1), (ITEM_EXP as i32, 1000)],
    );
    world.insert_item_exchange(9107, row);
    world.insert_test_quest_helper(choice_helper(9107, 700, 2));
    let exp_before = world.get_character_info(1).unwrap().exp;
    lua.load("if RunQuestExchange(1, 9107) then SaveEvent(1, 9107) else SaveEvent(1, 9107) end")
        .exec()
        .unwrap();
    // the quest was not completed, nothing was taken or given
    let state = world.with_session(1, |h| h.quests.get(&700).map(|q| q.quest_state));
    assert_ne!(state.flatten(), Some(2), "quest must stay open");
    assert_eq!(count_in_bag(&world, origin), 1, "turn-in item kept");
    assert_eq!(count_in_bag(&world, items[0]), 0);
    assert_eq!(world.get_character_info(1).unwrap().exp, exp_before);
    assert!(
        has_no_slot_error(&drain_packets(&mut rx)),
        "the player must be told the bag is full"
    );
}
