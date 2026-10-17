//! Reward center: one generic editor for every reward table of the game. The tables and their columns are a fixed
//! registry (nothing from the request is ever used as an identifier); values go through `jsonb_populate_record`,
//! so the column types of the database decide what is valid. After every change the game server is told to reload
//! the table (admin command queue), so a change is live at once.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::legacy::{audit, auth, queue_command, ApiError};
use crate::App;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/jstkoadminpanel/api/rewards/systems", get(systems))
        .route("/jstkoadminpanel/api/rewards/{key}", get(rows))
        .route("/jstkoadminpanel/api/rewards/{key}/save", post(save))
        .route("/jstkoadminpanel/api/rewards/{key}/delete", post(delete))
}

type R = Result<Json<Value>, ApiError>;

/// kind: item (item id, name + icon are shown), int, text, bool, zone, flag
struct Col(&'static str, &'static str, &'static str);

struct Sys {
    key: &'static str,
    title: &'static str,
    group: &'static str,
    note: &'static str,
    table: &'static str,
    pk: &'static [&'static str],
    /// the single pk column is a database serial: left out of inserts
    serial: bool,
    /// the single integer pk gets MAX+1 when a new row is added
    next_pk: bool,
    /// rows are grouped / filtered by this column (event, rank …)
    group_col: &'static str,
    reload: &'static str,
    order: &'static str,
    cols: &'static [Col],
}

const SYSTEMS: &[Sys] = &[
    Sys {
        key: "event_rules", title: "Ultima / Fire & Ice (Golem) / Manes Survival ödülleri", group: "Savaş & Etkinlik",
        note: "event_key: ultima | golem | manes.  cond — ultima: survive (boss'u yenip hayatta kalan), stage1 / stage2 / stage3 (o safhada elenen), box (boss kutusu; şans % = en yüksek yüzde, gerçek şans hasar payı x3).  golem: win / lose / draw (kazanan / kaybeden / beraberlik grubu), belt (katılanlar arasında şans); pick = grup içinden rastgele kaç kişi alsın (0 = hepsi); şans %.  manes: rank (rank_from..rank_to arası sıra, rank_to 0 = sonuna kadar). days > 0 ise eşya o kadar gün sonra biter. Kapalı (Aktif işareti kalkık) satırlar verilmez. Bir etkinliğin tüm satırları silinirse sunucudaki eski varsayılan ödüller kullanılır.",
        table: "event_reward_rule", pk: &["id"], serial: true, next_pk: false, group_col: "event_key", reload: "reward_rules", order: "event_key, cond, id",
        cols: &[Col("id", "No", "int"), Col("event_key", "Etkinlik", "text"), Col("cond", "Durum", "text"), Col("rank_from", "Sıra (başlangıç)", "int"), Col("rank_to", "Sıra (son)", "int"),
            Col("pick", "Rastgele kişi", "int"), Col("chance", "Şans %", "int"), Col("item_id", "Eşya", "item"), Col("item_count", "Adet", "int"), Col("days", "Süre (gün)", "int"),
            Col("enabled", "Aktif", "bool"), Col("note", "Not", "text")],
    },
    Sys {
        key: "pus_items", title: "Power Up Store ürünleri", group: "Mağaza & Çarklar",
        note: "Mağazadaki ürünler. category = kategori numarası (aşağıdaki kategori tablosundan), price_type 0 = KC, 1 = TL, buy_count = bir alımda verilen adet, süre (gün) 0 = süresiz (kalıcı), >0 ise eşya o kadar gün sonra düşer. Kaydedince mağaza hemen güncellenir (oyuncular mağazayı yeniden açınca yeni listeyi görür).",
        table: "pus_items", pk: &["id"], serial: false, next_pk: true, group_col: "category", reload: "pus", order: "category, id",
        cols: &[Col("id", "No", "int"), Col("item_id", "Eşya", "item"), Col("item_name", "Ad", "text"), Col("price", "Fiyat", "int"), Col("price_type", "Fiyat tipi (0 KC / 1 TL)", "int"),
            Col("buy_count", "Alım adedi", "int"), Col("category", "Kategori", "int"), Col("send_type", "Gönderim tipi", "int"), Col("duration_days", "Süre (gün, 0=süresiz)", "int")],
    },
    Sys {
        key: "pus_category", title: "Power Up Store kategorileri", group: "Mağaza & Çarklar",
        note: "Mağazadaki sekmeler (en çok 5 tanesi sol menüde görünür). category_id ürünlerdeki category ile aynı olmalı; status 1 = açık.",
        table: "pus_category", pk: &["id"], serial: false, next_pk: true, group_col: "", reload: "pus", order: "id",
        cols: &[Col("id", "No", "int"), Col("category_name", "Ad", "text"), Col("description", "Açıklama", "text"), Col("category_id", "Kategori no", "int"), Col("status", "Durum", "int")],
    },
    Sys {
        key: "event_rewards", title: "Etkinlik ödülleri (BDW / Chaos / Juraid / Forgotten Temple …)", group: "Savaş & Etkinlik",
        note: "local_id: 9 = Border Defence War, 10 = Chaos, 11 = Juraid Mountain, 13 = Forgotten Temple, 14 = Beef. Kazanan (is_winner) ve kaybeden için ayrı satır yazılır; her satır 3 eşya + exp / NP / KC / coin verir.",
        table: "event_rewards", pk: &["s_index"], serial: true, next_pk: false, group_col: "local_id", reload: "event_rewards", order: "local_id, is_winner DESC, s_index",
        cols: &[Col("s_index", "No", "int"), Col("status", "Aktif", "bool"), Col("local_id", "Etkinlik", "event"), Col("is_winner", "Kazanan", "bool"), Col("description", "Açıklama", "text"),
            Col("item_id1", "Eşya 1", "item"), Col("item_count1", "Adet 1", "int"), Col("item_expiration1", "Süre (gün) 1", "int"),
            Col("item_id2", "Eşya 2", "item"), Col("item_count2", "Adet 2", "int"), Col("item_expiration2", "Süre (gün) 2", "int"),
            Col("item_id3", "Eşya 3", "item"), Col("item_count3", "Adet 3", "int"), Col("item_expiration3", "Süre (gün) 3", "int"),
            Col("experience", "Exp", "int"), Col("loyalty", "NP", "int"), Col("cash", "KC", "int"), Col("noah", "Coin", "int")],
    },
    Sys {
        key: "chaos_rewards", title: "Chaos Expansion sıra ödülleri", group: "Savaş & Etkinlik",
        note: "Sıraya (rank_id) göre 5 eşya + exp / NP / KC / coin.",
        table: "event_chaos_rewards", pk: &["rank_id"], serial: false, next_pk: false, group_col: "", reload: "chaos_rewards", order: "rank_id",
        cols: &[Col("rank_id", "Sıra", "int"),
            Col("item_id1", "Eşya 1", "item"), Col("item_count1", "Adet 1", "int"), Col("item_expiration1", "Süre 1", "int"),
            Col("item_id2", "Eşya 2", "item"), Col("item_count2", "Adet 2", "int"), Col("item_expiration2", "Süre 2", "int"),
            Col("item_id3", "Eşya 3", "item"), Col("item_count3", "Adet 3", "int"), Col("item_expiration3", "Süre 3", "int"),
            Col("item_id4", "Eşya 4", "item"), Col("item_count4", "Adet 4", "int"), Col("item_expiration4", "Süre 4", "int"),
            Col("item_id5", "Eşya 5", "item"), Col("item_count5", "Adet 5", "int"), Col("item_expiration5", "Süre 5", "int"),
            Col("experience", "Exp", "int"), Col("loyalty", "NP", "int"), Col("cash", "KC", "int"), Col("noah", "Coin", "int")],
    },
    Sys {
        key: "collection_race", title: "Collection Race ödülleri", group: "Savaş & Etkinlik",
        note: "event_id = Collection Race etkinliğinin numarası (Etkinlikler > Collection Race sayfasındaki sıra). Ödüller etkinlik bittiğinde veritabanından okunur.",
        table: "collection_race_reward", pk: &["idx"], serial: false, next_pk: true, group_col: "event_id", reload: "", order: "event_id, idx",
        cols: &[Col("idx", "No", "int"), Col("event_id", "Etkinlik No", "int"), Col("description", "Açıklama", "text"), Col("item_id", "Eşya", "item"),
            Col("item_count", "Adet", "int"), Col("rate", "Oran %", "int"), Col("item_time", "Süre", "int"), Col("item_flag", "Flag", "int"), Col("item_session", "Session", "int")],
    },
    Sys {
        key: "cinderella", title: "Cinderella (Fun War) sıra ödülleri", group: "Savaş & Etkinlik",
        note: "Sıraya göre exp / KC / NP / coin; eşyalar aşağıdaki Cinderella eşya tablosunda.",
        table: "cindwar_reward", pk: &["rank_id"], serial: false, next_pk: false, group_col: "", reload: "cinderella", order: "rank_id",
        cols: &[Col("rank_id", "Sıra", "int"), Col("exp_count", "Exp", "int"), Col("cash_count", "KC", "int"), Col("loyalty_count", "NP", "int"), Col("money_count", "Coin", "int")],
    },
    Sys {
        key: "cinderella_items", title: "Cinderella ödül eşyaları", group: "Savaş & Etkinlik",
        note: "rank_id + slot çifti tekil olmalı.",
        table: "cindwar_reward_item", pk: &["rank_id", "slot"], serial: false, next_pk: false, group_col: "rank_id", reload: "cinderella", order: "rank_id, slot",
        cols: &[Col("rank_id", "Sıra", "int"), Col("slot", "Slot", "int"), Col("item_id", "Eşya", "item"), Col("item_count", "Adet", "int"), Col("item_duration", "Süre", "int"), Col("item_expiration", "Bitiş", "int")],
    },
    Sys {
        key: "zone_kill", title: "Bölge öldürme ödülleri", group: "Bölge & Günlük",
        note: "Bir bölgede (zone) düşman öldürünce kill_count sayısında verilen eşya. nation 0 = herkes.",
        table: "zone_kill_reward", pk: &["idx"], serial: false, next_pk: true, group_col: "zone_id", reload: "zone_rewards", order: "zone_id, idx",
        cols: &[Col("idx", "No", "int"), Col("zone_id", "Bölge", "zone"), Col("nation", "Irk (0 hepsi)", "int"), Col("party_required", "Party şartı", "int"), Col("all_party_reward", "Tüm party", "bool"),
            Col("kill_count", "Kill", "int"), Col("item_name", "Ad", "text"), Col("item_id", "Eşya", "item"), Col("item_count", "Adet", "int"), Col("item_duration", "Süre", "int"),
            Col("item_flag", "Flag", "int"), Col("item_expiration", "Bitiş", "int"), Col("drop_rate", "Oran", "int"), Col("give_to_warehouse", "Depoya ver", "bool"),
            Col("status", "Aktif (1)", "int"), Col("is_priest", "Rahip", "bool"), Col("priest_rate", "Rahip oranı", "int")],
    },
    Sys {
        key: "zone_online", title: "Bölgede online kalma ödülleri", group: "Bölge & Günlük",
        note: "Bölgede her `minute` dakikada bir verilen ödül; pre_ ile başlayanlar premium oyuncular içindir.",
        table: "zone_online_reward", pk: &["zone_id"], serial: false, next_pk: false, group_col: "", reload: "zone_rewards", order: "zone_id",
        cols: &[Col("zone_id", "Bölge", "zone"), Col("item_id", "Eşya", "item"), Col("item_count", "Adet", "int"), Col("item_time", "Süre", "int"), Col("minute", "Dakika", "int"),
            Col("loyalty", "NP", "int"), Col("cash", "KC", "int"), Col("tl", "TL", "int"),
            Col("pre_item_id", "Premium eşya", "item"), Col("pre_item_count", "Prem. adet", "int"), Col("pre_item_time", "Prem. süre", "int"), Col("pre_minute", "Prem. dakika", "int"),
            Col("pre_loyalty", "Prem. NP", "int"), Col("pre_cash", "Prem. KC", "int"), Col("pre_tl", "Prem. TL", "int")],
    },
    Sys {
        key: "daily_quests", title: "Günlük görevler (Daily Quest)", group: "Bölge & Günlük",
        note: "kill_type / mob_id: öldürülecek yaratıklar (4 adede kadar, 0 = boş), kill_count: kaç adet. time_type: 0 günlük. replay_time: tekrar süresi. Ödüller reward_N + count_N.",
        table: "daily_quests", pk: &["id"], serial: false, next_pk: true, group_col: "", reload: "daily_quests", order: "id",
        cols: &[Col("id", "No", "int"), Col("quest_name", "Görev adı", "text"), Col("quest_id", "Quest ID", "int"), Col("time_type", "Süre tipi", "int"), Col("kill_type", "Kill tipi", "int"),
            Col("mob_id_1", "Yaratık 1", "int"), Col("mob_id_2", "Yaratık 2", "int"), Col("mob_id_3", "Yaratık 3", "int"), Col("mob_id_4", "Yaratık 4", "int"), Col("kill_count", "Adet", "int"),
            Col("reward_1", "Ödül 1", "item"), Col("count_1", "Adet 1", "int"), Col("reward_2", "Ödül 2", "item"), Col("count_2", "Adet 2", "int"),
            Col("reward_3", "Ödül 3", "item"), Col("count_3", "Adet 3", "int"), Col("reward_4", "Ödül 4", "item"), Col("count_4", "Adet 4", "int"),
            Col("zone_id", "Bölge", "zone"), Col("min_level", "Min level", "int"), Col("max_level", "Max level", "int"), Col("replay_time", "Tekrar (saat)", "int"), Col("random_id", "Random", "int")],
    },
    Sys {
        key: "daily_reward", title: "Günlük giriş (attendance) ödülleri", group: "Bölge & Günlük",
        note: "day_index 0-24: ayın giriş günlerine göre ödül.",
        table: "daily_reward", pk: &["day_index"], serial: false, next_pk: false, group_col: "", reload: "", order: "day_index",
        cols: &[Col("day_index", "Gün", "int"), Col("item_id", "Eşya", "item")],
    },
    Sys {
        key: "daily_reward_cum", title: "Günlük giriş toplam ödülü", group: "Bölge & Günlük",
        note: "Tek satır (id = 1): ayı tamamlayanın aldığı 3 eşya.",
        table: "daily_reward_cumulative", pk: &["id"], serial: false, next_pk: false, group_col: "", reload: "", order: "id",
        cols: &[Col("id", "id", "int"), Col("item1", "Eşya 1", "item"), Col("item2", "Eşya 2", "item"), Col("item3", "Eşya 3", "item")],
    },
    Sys {
        key: "manner_store", title: "Manner Store (itibar mağazası)", group: "Mağaza & Çarklar",
        note: "tab = mağaza sayfası, currency 0 = manner puanı, 1 = NP; days > 0 ise eşya o kadar gün sonra biter.",
        table: "manner_store_items", pk: &["id"], serial: true, next_pk: false, group_col: "tab", reload: "", order: "tab, id",
        cols: &[Col("id", "No", "int"), Col("tab", "Sayfa", "int"), Col("item_id", "Eşya", "item"), Col("item_count", "Adet", "int"), Col("price", "Fiyat", "int"),
            Col("currency", "Para birimi", "int"), Col("days", "Süre (gün)", "int"), Col("enabled", "Aktif", "bool")],
    },
    Sys {
        key: "send_messages", title: "Giriş duyuruları", group: "Sunucu & Duyurular",
        note: "Oyuncular oyuna girdiğinde sırayla gösterilen duyuru mesajları (send_type 1 = girişte gösterilir, şu an tek kullanılan tür). chat_type mesajın renk/kanalını belirler (2 = bildirim rengi). Kaydedince sunucu hemen yeni listeyi kullanır.",
        table: "send_messages", pk: &["id"], serial: true, next_pk: false, group_col: "", reload: "send_messages", order: "id",
        cols: &[Col("id", "No", "int"), Col("message", "Mesaj", "text"), Col("sender", "Gönderen", "text"),
            Col("chat_type", "Sohbet tipi", "int"), Col("send_type", "Gönderim tipi", "int"), Col("send_hour_minute", "Saat kodu", "int")],
    },
    Sys {
        key: "native_coin", title: "Coin etkinliği ödülleri", group: "Mağaza & Çarklar",
        note: "required_points puana ulaşana verilir (reward_index 0-9).",
        table: "native_coin_reward", pk: &["reward_index"], serial: false, next_pk: false, group_col: "", reload: "", order: "reward_index",
        cols: &[Col("reward_index", "Sıra", "int"), Col("required_points", "Gereken puan", "int"), Col("item_id", "Eşya", "item"), Col("item_count", "Adet", "int")],
    },
    Sys {
        key: "native_jigsaw", title: "Puzzle (Jigsaw) etkinliği ödülleri", group: "Mağaza & Çarklar",
        note: "reward_index 0-8, required_total parça sayısı.",
        table: "native_jigsaw_reward", pk: &["reward_index"], serial: false, next_pk: false, group_col: "", reload: "", order: "reward_index",
        cols: &[Col("reward_index", "Sıra", "int"), Col("required_total", "Gereken", "int"), Col("item_id", "Eşya", "item"), Col("item_count", "Adet", "int")],
    },
    Sys {
        key: "native_roulette", title: "Rulet / Marble çarkı ödülleri", group: "Mağaza & Çarklar",
        note: "roulette_type 7 veya 8, slot 0-14, weight = çıkma ağırlığı.",
        table: "native_roulette_reward", pk: &["roulette_type", "slot"], serial: false, next_pk: false, group_col: "roulette_type", reload: "", order: "roulette_type, slot",
        cols: &[Col("roulette_type", "Çark", "int"), Col("slot", "Slot", "int"), Col("item_id", "Eşya", "item"), Col("item_count", "Adet", "int"), Col("weight", "Ağırlık", "int")],
    },
    Sys {
        key: "lottery", title: "Lottery etkinliği", group: "Mağaza & Çarklar",
        note: "Katılmak için gereken eşyalar (req_) ve kazanılan 4 ödül.",
        table: "lottery_event_settings", pk: &["lnum"], serial: false, next_pk: false, group_col: "", reload: "", order: "lnum",
        cols: &[Col("lnum", "No", "int"), Col("req_item1", "Gereken 1", "item"), Col("req_item_count1", "Adet 1", "int"), Col("req_item2", "Gereken 2", "item"), Col("req_item_count2", "Adet 2", "int"),
            Col("req_item3", "Gereken 3", "item"), Col("req_item_count3", "Adet 3", "int"), Col("req_item4", "Gereken 4", "item"), Col("req_item_count4", "Adet 4", "int"),
            Col("req_item5", "Gereken 5", "item"), Col("req_item_count5", "Adet 5", "int"),
            Col("reward_item1", "Ödül 1", "item"), Col("reward_item2", "Ödül 2", "item"), Col("reward_item3", "Ödül 3", "item"), Col("reward_item4", "Ödül 4", "item"),
            Col("user_limit", "Kişi sınırı", "int"), Col("event_time", "Süre", "int")],
    },
];

fn internal<E: std::fmt::Display>(what: &'static str) -> impl Fn(E) -> ApiError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        ApiError::Internal
    }
}

fn find(key: &str) -> Result<&'static Sys, ApiError> {
    SYSTEMS.iter().find(|s| s.key == key).ok_or(ApiError::NotFound)
}

async fn systems(headers: HeaderMap, State(app): State<App>) -> R {
    auth(&headers, false)?;
    let mut list = Vec::new();
    for s in SYSTEMS {
        let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", s.table)).fetch_one(&app.pool).await.unwrap_or(-1);
        list.push(json!({
            "key": s.key, "title": s.title, "group": s.group, "note": s.note, "pk": s.pk, "serial": s.serial, "next_pk": s.next_pk,
            "group_col": s.group_col, "live": !s.reload.is_empty(), "count": n,
            "cols": s.cols.iter().map(|c| json!({"name": c.0, "label": c.1, "kind": c.2})).collect::<Vec<_>>(),
        }));
    }
    Ok(Json(json!({
        "systems": list,
        "events": {"1":"Castle Siege","2":"Napies Gorge","3":"Alseid's Prairie","4":"Nied's Triangle","5":"Nereid's Island","6":"Zipang","7":"Oreads","8":"Snow War",
                   "9":"Border Defence War","10":"Chaos Expansion","11":"Juraid Mountain","12":"Under the Castle","13":"Forgotten Temple","14":"Beef Event"}
    })))
}

async fn rows(headers: HeaderMap, State(app): State<App>, Path(key): Path<String>) -> R {
    auth(&headers, false)?;
    let s = find(&key)?;
    let sql = format!("SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY {}), '[]'::jsonb) FROM {} t", s.order, s.table);
    let v: Value = sqlx::query_scalar(&sql).fetch_one(&app.pool).await.map_err(internal("rewards rows"))?;
    Ok(Json(json!({"rows": v})))
}

#[derive(Deserialize)]
struct SaveIn {
    /// primary key of the row being edited (absent = new row)
    pk: Option<Map<String, Value>>,
    row: Map<String, Value>,
}

/// the request value as the column's JSON type: numbers / booleans / text only, strings that hold a number become numbers
fn clean(s: &Sys, row: &Map<String, Value>) -> Result<Map<String, Value>, ApiError> {
    let mut out = Map::new();
    for c in s.cols {
        let Some(v) = row.get(c.0) else { continue };
        let v = match (c.2, v) {
            ("text", Value::String(t)) => Value::String(t.chars().take(100).collect()),
            ("text", Value::Null) => Value::String(String::new()),
            ("bool", Value::Bool(b)) => Value::Bool(*b),
            ("bool", Value::Number(n)) => Value::Bool(n.as_i64().unwrap_or(0) != 0),
            ("text", _) => return Err(ApiError::BadRequest),
            (_, Value::Number(n)) if n.is_i64() => Value::Number(n.clone()),
            (_, Value::String(t)) => match t.trim().parse::<i64>() {
                Ok(n) => json!(n),
                Err(_) => return Err(ApiError::BadRequest),
            },
            (_, Value::Bool(b)) => json!(i64::from(*b)),
            _ => return Err(ApiError::BadRequest),
        };
        out.insert(c.0.to_string(), v);
    }
    Ok(out)
}

async fn save(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(key): Path<String>,
    Json(p): Json<SaveIn>,
) -> R {
    let (_, sess) = auth(&headers, true)?;
    let s = find(&key)?;
    let mut row = clean(s, &p.row)?;
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let result = if let Some(pk) = &p.pk {
        let pk = clean(s, pk)?;
        if s.pk.iter().any(|c| !pk.contains_key(*c)) {
            return Err(ApiError::BadRequest);
        }
        let set: Vec<String> = s
            .cols
            .iter()
            .filter(|c| !s.pk.contains(&c.0) && row.contains_key(c.0))
            .map(|c| format!("{0} = r.{0}", c.0))
            .collect();
        if set.is_empty() {
            return Err(ApiError::BadRequest);
        }
        let cond: Vec<String> = s.pk.iter().map(|c| format!("t.{0} = k.{0}", c)).collect();
        let sql = format!(
            "UPDATE {t} AS t SET {set} FROM jsonb_populate_record(NULL::{t}, $1::jsonb) r, jsonb_populate_record(NULL::{t}, $2::jsonb) k WHERE {cond}",
            t = s.table,
            set = set.join(", "),
            cond = cond.join(" AND ")
        );
        sqlx::query(&sql).bind(Value::Object(row.clone())).bind(Value::Object(pk)).execute(&mut *tx).await
    } else {
        if s.next_pk && !row.contains_key(s.pk[0]) {
            let next: i64 = sqlx::query_scalar(&format!("SELECT (COALESCE(MAX({}), 0) + 1)::bigint FROM {}", s.pk[0], s.table))
                .fetch_one(&mut *tx)
                .await
                .map_err(internal("next pk"))?;
            row.insert(s.pk[0].to_string(), json!(next));
        }
        let cols: Vec<&str> = s.cols.iter().map(|c| c.0).filter(|c| !(s.serial && s.pk[0] == *c) && row.contains_key(*c)).collect();
        if cols.is_empty() {
            return Err(ApiError::BadRequest);
        }
        let list = cols.join(", ");
        let sql = format!("INSERT INTO {t} ({list}) SELECT {list} FROM jsonb_populate_record(NULL::{t}, $1::jsonb)", t = s.table);
        sqlx::query(&sql).bind(Value::Object(row.clone())).execute(&mut *tx).await
    };
    let done = match result {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            tracing::warn!(table = s.table, error = %e, "reward save failed");
            let dup = e.as_database_error().map(|d| d.code().as_deref() == Some("23505")).unwrap_or(false);
            return Err(if dup { ApiError::Conflict } else { ApiError::BadRequest });
        }
    };
    if done == 0 {
        return Err(ApiError::NotFound);
    }
    audit(&mut tx, &sess.actor, "rewards.save", s.key, json!({"pk": p.pk, "row": row}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(reload(&app, s).await))
}

#[derive(Deserialize)]
struct DelIn {
    pk: Map<String, Value>,
}

async fn delete(
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(app): State<App>,
    Path(key): Path<String>,
    Json(p): Json<DelIn>,
) -> R {
    let (_, sess) = auth(&headers, true)?;
    let s = find(&key)?;
    let pk = clean(s, &p.pk)?;
    if s.pk.iter().any(|c| !pk.contains_key(*c)) {
        return Err(ApiError::BadRequest);
    }
    let cond: Vec<String> = s.pk.iter().map(|c| format!("t.{0} = k.{0}", c)).collect();
    let sql = format!("DELETE FROM {t} AS t USING jsonb_populate_record(NULL::{t}, $1::jsonb) k WHERE {cond}", t = s.table, cond = cond.join(" AND "));
    let mut tx = app.pool.begin().await.map_err(internal("tx"))?;
    let n = sqlx::query(&sql).bind(Value::Object(pk.clone())).execute(&mut *tx).await.map_err(internal("reward delete"))?.rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    audit(&mut tx, &sess.actor, "rewards.delete", s.key, json!({"pk": pk}), peer.ip()).await?;
    tx.commit().await.map_err(internal("commit"))?;
    Ok(Json(reload(&app, s).await))
}

/// queue the hot reload of the game server (tables read when needed need none)
async fn reload(app: &App, s: &Sys) -> Value {
    if s.reload.is_empty() {
        return json!({"ok": true, "live": true, "note": "Kaydedildi. Bu ödül oyunda kullanılırken veritabanından okunur, hemen geçerlidir."});
    }
    match queue_command(&app.pool, "reload", json!({"what": s.reload})).await {
        Ok(id) => json!({"ok": true, "live": true, "command": id, "note": "Kaydedildi, oyun sunucusuna yenileme komutu gönderildi."}),
        Err(_) => json!({"ok": true, "live": false, "note": "Kaydedildi ama yenileme komutu sıraya alınamadı."}),
    }
}
