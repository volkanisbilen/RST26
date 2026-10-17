-- Enable 3 box items for the Chaotic Generator (effect2 = 251 marks a valid "piece" item,
-- see bifrost_piece_exchange() in item_upgrade.rs / handler::item_upgrade) and seed their
-- reward pools in item_exchange (random_flag = 1, read by WorldState::get_bifrost_exchanges).
--
-- 810377000 ("Dragon Box", the other item with that name) is NOT enabled here: it has
-- countable = 0, and the Chaotic Generator requires the origin item to be countable.
--
-- n_index has no sequence (the seed data uses several disjoint numeric ranges), so indexes
-- are computed from the table's real current max rather than guessed.
DO $$
DECLARE
    next_idx BIGINT;
BEGIN
    UPDATE item SET effect2 = 251 WHERE num IN (811053000, 810341000, 999915000);

    -- Akara Accessory Box: every base (+0, no upgrade) Akara accessory item, equal weight.
    SELECT COALESCE(MAX(n_index), 0) + 1 INTO next_idx FROM item_exchange;
    INSERT INTO item_exchange (n_index, random_flag, origin_item_num1, origin_item_count1, exchange_item_num1, exchange_item_count1)
    SELECT next_idx + ord - 1, 1, 811053000, 1, reward, 5
    FROM unnest(ARRAY[
        810891000,810892000,810904000,810898000,810899000,810912000,810905000,810907000,
        810896000,810895000,810911000,810902000,810897000,810903000,810910000,810909000,
        810906000,810894000,810889000,810893000,810908000,810901000,
        1310410109,1320510810,1320310800,1310512120,1340610106,1310510104,1320110780,1340410113,
        1310610106,1399310150,1399306110,1399312170,1340610107,1320310790,1310513121,1310514122,
        1340710108,1310310008,1310511119,1310510110,1310410105,1310310007,1399308130,1399309140,
        1310310006,1310410107,1399311160,1310610102,1340410115,1320610820,1340410123,1310310005,
        1310610108,1399307120
    ]) WITH ORDINALITY AS t(reward, ord);

    -- Dragon Box (810341000): the base "_000" (no stat bonus) Dragon Scale Armor pieces
    -- across all race variants, equal weight. Noah Dragon Scale excluded (different item
    -- family per user decision).
    SELECT COALESCE(MAX(n_index), 0) + 1 INTO next_idx FROM item_exchange;
    INSERT INTO item_exchange (n_index, random_flag, origin_item_num1, origin_item_count1, exchange_item_num1, exchange_item_count1)
    SELECT next_idx + ord - 1, 1, 810341000, 1, reward, 5
    FROM unnest(ARRAY[
        507002000,922002000,917002000,517002000,
        922001000,517001000,507001000,917001000,
        507005000,917005000,517005000,922005000,
        922004000,517004000,917004000,507004000,
        507003000,922003000,917003000,517003000
    ]) WITH ORDINALITY AS t(reward, ord);

    -- Black Lion Weapon Box (999915000): mostly a Blessed Upgrade Scroll filler (90%
    -- weight) so the 10 base weapons stay rare (1% each, 10% combined).
    SELECT COALESCE(MAX(n_index), 0) + 1 INTO next_idx FROM item_exchange;
    INSERT INTO item_exchange (n_index, random_flag, origin_item_num1, origin_item_count1, exchange_item_num1, exchange_item_count1)
    VALUES
        (next_idx,    1, 999915000, 1, 379021000, 4500),
        (next_idx+1,  1, 999915000, 1, 1113367900, 50),
        (next_idx+2,  1, 999915000, 1, 1113363900, 50),
        (next_idx+3,  1, 999915000, 1, 1113360900, 50),
        (next_idx+4,  1, 999915000, 1, 2113364760, 50),
        (next_idx+5,  1, 999915000, 1, 2113364780, 50),
        (next_idx+6,  1, 999915000, 1, 1113361900, 50),
        (next_idx+7,  1, 999915000, 1, 2113364800, 50),
        (next_idx+8,  1, 999915000, 1, 1113365900, 50),
        (next_idx+9,  1, 999915000, 1, 1113366900, 50),
        (next_idx+10, 1, 999915000, 1, 1113362900, 50);
END $$;
