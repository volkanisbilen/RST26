<?php
declare(strict_types=1);

require_once __DIR__ . '/db.php';

/**
 * Clears a stuck "online" marker for one of the account's own characters -- same rule ko-admin's
 * staff tool uses (crates/ko-admin/src/extra.rs::clear_online): if the game server's heartbeat is
 * fresh, queue a graceful kick for it to process; if the server looks dead, delete the row
 * directly so the player isn't stuck waiting on a server that isn't coming back to process it.
 *
 * @return array{ok:bool, message:string}
 */
function jstko_rescue_character(string $accountId, string $charName, bool $toMoradon = false): array
{
    $pdo = jstko_db();

    // Ownership check: the character must belong to this account.
    $stmt = $pdo->prepare(
        'SELECT 1 FROM account_char WHERE str_account_id = :id AND (
            lower(str_char_id1) = lower(:n) OR lower(str_char_id2) = lower(:n) OR
            lower(str_char_id3) = lower(:n) OR lower(str_char_id4) = lower(:n))'
    );
    $stmt->execute(['id' => $accountId, 'n' => $charName]);
    if (!$stmt->fetch()) {
        return ['ok' => false, 'message' => 'Bu karakter hesabınıza ait değil.'];
    }

    if ($toMoradon) {
        return jstko_rescue_move_to_moradon($charName);
    }

    $stmt = $pdo->prepare('SELECT 1 FROM currentuser WHERE lower(str_char_id) = lower(:n)');
    $stmt->execute(['n' => $charName]);
    if (!$stmt->fetch()) {
        return ['ok' => false, 'message' => 'Bu karakter zaten askıda değil, doğrudan giriş yapabilirsiniz.'];
    }

    $age = $pdo->query('SELECT EXTRACT(EPOCH FROM now()-at)::int FROM admin_heartbeat WHERE id = 1')->fetchColumn();

    if ($age !== false && $age !== null && (int)$age < 30) {
        // Server is alive: ask it to kick the character properly instead of yanking the row.
        $stmt = $pdo->prepare("INSERT INTO admin_command (command, args) VALUES ('kick', :args)");
        $stmt->execute(['args' => json_encode(['name' => $charName])]);
        return ['ok' => true, 'message' => 'Karakteriniz oyundan güvenli şekilde çıkarılıyor, birkaç saniye içinde tekrar giriş yapabilirsiniz.'];
    }

    $pdo->prepare('DELETE FROM currentuser WHERE lower(str_char_id) = lower(:n)')->execute(['n' => $charName]);
    return ['ok' => true, 'message' => 'Karakteriniz askıdan kurtarıldı, şimdi giriş yapabilirsiniz.'];
}

/**
 * Moves a character to Moradon (for characters stuck in a zone). With the game server running the server does it
 * (kicks the character if it is online so its save does not overwrite the move, then writes the new position);
 * with the server down the position is written here and the stuck online marker is removed.
 */
function jstko_rescue_move_to_moradon(string $charName): array
{
    $pdo = jstko_db();
    $age = $pdo->query('SELECT EXTRACT(EPOCH FROM now()-at)::int FROM admin_heartbeat WHERE id = 1')->fetchColumn();
    if ($age !== false && $age !== null && (int)$age < 30) {
        $stmt = $pdo->prepare("INSERT INTO admin_command (command, args) VALUES ('rescue_move', :args)");
        $stmt->execute(['args' => json_encode(['name' => $charName])]);
        return ['ok' => true, 'message' => 'Karakteriniz Moradon’a taşınıyor (oyundaysa oyundan çıkarılır). Birkaç saniye sonra tekrar giriş yapabilirsiniz.'];
    }
    // server is down: Moradon start point (x 267, z 303), stored x100 like the game does
    $pdo->prepare('UPDATE userdata SET zone = 21, px = 26700, py = 0, pz = 30300 WHERE lower(str_user_id) = lower(:n)')->execute(['n' => $charName]);
    $pdo->prepare('DELETE FROM currentuser WHERE lower(str_char_id) = lower(:n)')->execute(['n' => $charName]);
    return ['ok' => true, 'message' => 'Karakteriniz Moradon’a taşındı, şimdi giriş yapabilirsiniz.'];
}

/** The account's own character names (up to 4 slots), with their current "stuck" status. */
function jstko_account_characters(string $accountId): array
{
    $pdo = jstko_db();
    $stmt = $pdo->prepare('SELECT str_char_id1, str_char_id2, str_char_id3, str_char_id4 FROM account_char WHERE str_account_id = :id');
    $stmt->execute(['id' => $accountId]);
    $row = $stmt->fetch();
    if (!$row) {
        return [];
    }
    $names = array_filter([$row['str_char_id1'], $row['str_char_id2'], $row['str_char_id3'], $row['str_char_id4']]);
    if (!$names) {
        return [];
    }
    $stmt = $pdo->prepare(
        "SELECT u.str_user_id name, u.nation, u.class, u.level, u.zone,
                EXISTS(SELECT 1 FROM currentuser c WHERE lower(c.str_char_id)=lower(u.str_user_id)) stuck
         FROM userdata u WHERE lower(u.str_user_id) IN (" . implode(',', array_fill(0, count($names), 'lower(?)')) . ")"
    );
    $stmt->execute(array_values($names));
    return $stmt->fetchAll();
}
