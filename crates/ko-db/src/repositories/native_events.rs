//! Server-authoritative persistence for the native v2615 event panels.

use crate::models::native_events::{
    NativeCoinState, NativeJigsawState, NativeMarbleState, NativeMarbleTile, NativeRewardGrant,
    NativeRouletteHistory, NativeRoulettePending, NativeRouletteReward,
};
use crate::DbPool;

pub struct NativeEventsRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> NativeEventsRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn is_active(&self, key: &str) -> Result<bool, sqlx::Error> {
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT active FROM native_event_config WHERE event_key = $1")
                .bind(key)
                .fetch_optional(self.pool)
                .await?;
        Ok(row.map(|v| v.0).unwrap_or(false))
    }

    pub async fn set_active(&self, key: &str, active: bool) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE native_event_config SET active=$2, updated_at=NOW() WHERE event_key=$1",
        )
        .bind(key)
        .bind(active)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Ensure every native v2615 event is available after a server restart.
    /// Runtime GM close commands can still disable individual rows until the
    /// next restart.
    pub async fn activate_all(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("UPDATE native_event_config SET active=TRUE, updated_at=NOW()")
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Reserve one of the three claims allowed in the current ISO week.
    /// The advisory transaction lock makes simultaneous reply packets for the
    /// same character serialize before the count is checked.
    pub async fn reserve_board_claim(&self, character: &str) -> Result<Option<i64>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1)::BIGINT)")
            .bind(character)
            .execute(&mut *tx)
            .await?;
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM native_board_claim \
             WHERE character_name=$1 \
               AND claimed_at >= date_trunc('week', CURRENT_TIMESTAMP)",
        )
        .bind(character)
        .fetch_one(&mut *tx)
        .await?;
        if count.0 >= 3 {
            tx.rollback().await?;
            return Ok(None);
        }
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO native_board_claim(character_name,item_id) \
             VALUES($1,811084000) RETURNING id",
        )
        .bind(character)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(row.0))
    }

    pub async fn cancel_board_claim(&self, id: i64, character: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM native_board_claim WHERE id=$1 AND character_name=$2")
            .bind(id)
            .bind(character)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn board_claim_history(
        &self,
        character: &str,
    ) -> Result<Vec<(String, i32, i32)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT character_name,item_id,EXTRACT(EPOCH FROM claimed_at)::INTEGER \
             FROM native_board_claim WHERE character_name=$1 \
             ORDER BY claimed_at DESC LIMIT 20",
        )
        .bind(character)
        .fetch_all(self.pool)
        .await
    }

    /// Active Akara Altar rows in the exact order used by the v2615 panel.
    pub async fn akara_auctions(
        &self,
    ) -> Result<Vec<(i16, i32, i16, i64, i64, i32, i32)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT slot,item_id,item_ext,current_bid,min_increment, \
                    EXTRACT(EPOCH FROM ends_at)::INTEGER,bid_count \
             FROM native_akara_auction \
             WHERE enabled=TRUE AND ends_at>CURRENT_TIMESTAMP \
             ORDER BY slot LIMIT 16",
        )
        .fetch_all(self.pool)
        .await
    }

    /// Atomically accept a higher bid and return the refreshed row.
    pub async fn place_akara_bid(
        &self,
        slot: i16,
        item_id: i32,
        character: &str,
        offered: i64,
    ) -> Result<Option<(i64, i64, i32, i32)>, sqlx::Error> {
        sqlx::query_as(
            "UPDATE native_akara_auction SET \
                current_bid=$4,current_bidder=$3,bid_count=bid_count+1,updated_at=NOW() \
             WHERE slot=$1 AND item_id=$2 AND enabled=TRUE \
               AND ends_at>CURRENT_TIMESTAMP \
               AND $4>=current_bid+min_increment \
             RETURNING current_bid,min_increment, \
                       EXTRACT(EPOCH FROM ends_at)::INTEGER,bid_count",
        )
        .bind(slot)
        .bind(item_id)
        .bind(character)
        .bind(offered)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn roulette_rewards(
        &self,
        roulette_type: i16,
    ) -> Result<Vec<NativeRouletteReward>, sqlx::Error> {
        sqlx::query_as(
            "SELECT roulette_type,slot,item_id,item_count,weight FROM native_roulette_reward \
             WHERE roulette_type=$1 ORDER BY slot",
        )
        .bind(roulette_type)
        .fetch_all(self.pool)
        .await
    }

    pub async fn reserve_roulette_result(
        &self,
        character: &str,
        reward: &NativeRouletteReward,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO native_roulette_user \
             (character_name,pending_type,pending_slot,pending_item_id,pending_item_count,pending_created_at,last_spin_at,last_free_spin_date) \
             VALUES ($1,$2,$3,$4,$5,NOW(),NOW(),CASE WHEN $2::SMALLINT=7 THEN CURRENT_DATE ELSE NULL END) \
             ON CONFLICT (character_name) DO UPDATE SET \
               pending_type=EXCLUDED.pending_type,pending_slot=EXCLUDED.pending_slot, \
               pending_item_id=EXCLUDED.pending_item_id,pending_item_count=EXCLUDED.pending_item_count, \
               pending_created_at=NOW(),last_spin_at=NOW(), \
               last_free_spin_date=CASE WHEN $2::SMALLINT=7 THEN CURRENT_DATE ELSE native_roulette_user.last_free_spin_date END, \
               updated_at=NOW() \
             WHERE (native_roulette_user.pending_item_id IS NULL \
                OR native_roulette_user.pending_created_at < NOW() - INTERVAL '2 minutes') \
               AND ($2::SMALLINT<>7 OR native_roulette_user.last_free_spin_date IS DISTINCT FROM CURRENT_DATE)",
        )
        .bind(character)
        .bind(reward.roulette_type)
        .bind(reward.slot)
        .bind(reward.item_id)
        .bind(reward.item_count)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn roulette_pending(
        &self,
        character: &str,
    ) -> Result<Option<NativeRoulettePending>, sqlx::Error> {
        sqlx::query_as(
            "SELECT pending_type,pending_slot,pending_item_id,pending_item_count \
             FROM native_roulette_user WHERE character_name=$1",
        )
        .bind(character)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn complete_roulette(
        &self,
        character: &str,
    ) -> Result<Option<NativeRoulettePending>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let pending: Option<NativeRoulettePending> = sqlx::query_as(
            "SELECT pending_type,pending_slot,pending_item_id,pending_item_count \
             FROM native_roulette_user WHERE character_name=$1 FOR UPDATE",
        )
        .bind(character)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(ref p) = pending {
            if let (Some(kind), Some(item_id), Some(item_count)) =
                (p.pending_type, p.pending_item_id, p.pending_item_count)
            {
                sqlx::query(
                    "INSERT INTO native_roulette_history \
                     (character_name,roulette_type,item_id,item_count) VALUES ($1,$2,$3,$4)",
                )
                .bind(character)
                .bind(kind)
                .bind(item_id)
                .bind(item_count)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "UPDATE native_roulette_user SET pending_type=NULL,pending_slot=NULL, \
                     pending_item_id=NULL,pending_item_count=NULL,pending_created_at=NULL,updated_at=NOW() \
                     WHERE character_name=$1",
                )
                .bind(character)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(pending)
    }

    /// Seconds until the daily free spin is available again (0 = available now).
    pub async fn roulette_free_wait_seconds(&self, character: &str) -> Result<i32, sqlx::Error> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT GREATEST(1, EXTRACT(EPOCH FROM (CURRENT_DATE + INTERVAL '1 day' - NOW()))::int)              FROM native_roulette_user WHERE character_name=$1 AND last_free_spin_date = CURRENT_DATE",
        )
        .bind(character)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(|r| r.0).unwrap_or(0))
    }

    pub async fn roulette_history(
        &self,
        character: &str,
    ) -> Result<Vec<NativeRouletteHistory>, sqlx::Error> {
        sqlx::query_as(
            "SELECT roulette_type,item_id,item_count FROM native_roulette_history \
             WHERE character_name=$1 ORDER BY won_at DESC LIMIT 20",
        )
        .bind(character)
        .fetch_all(self.pool)
        .await
    }

    pub async fn jigsaw_state(&self, character: &str) -> Result<NativeJigsawState, sqlx::Error> {
        sqlx::query(
            "INSERT INTO native_jigsaw_user(character_name) VALUES($1) ON CONFLICT DO NOTHING",
        )
        .bind(character)
        .execute(self.pool)
        .await?;
        sqlx::query_as(
            "SELECT piece_counts,reward_claimed FROM native_jigsaw_user WHERE character_name=$1",
        )
        .bind(character)
        .fetch_one(self.pool)
        .await
    }

    /// Chance cards left today (8 a day; every piece placed uses one).
    pub async fn jigsaw_cards_left(&self, character: &str) -> Result<i32, sqlx::Error> {
        self.jigsaw_state(character).await?;
        sqlx::query(
            "UPDATE native_jigsaw_user SET pieces_today=0,reset_date=CURRENT_DATE              WHERE character_name=$1 AND reset_date<>CURRENT_DATE",
        )
        .bind(character)
        .execute(self.pool)
        .await?;
        let used: i16 = sqlx::query_scalar(
            "SELECT pieces_today FROM native_jigsaw_user WHERE character_name=$1",
        )
        .bind(character)
        .fetch_one(self.pool)
        .await?;
        Ok((8 - i32::from(used)).max(0))
    }

    /// The nine reward boxes of the puzzle: (pieces needed, item, count) in box order.
    pub async fn jigsaw_rewards(&self) -> Result<Vec<(i16, i32, i16)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT required_total,item_id,item_count FROM native_jigsaw_reward ORDER BY reward_index",
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn add_jigsaw_piece(
        &self,
        character: &str,
        piece: i16,
    ) -> Result<Option<NativeJigsawState>, sqlx::Error> {
        if !(0..8).contains(&piece) {
            return Ok(None);
        }
        self.jigsaw_state(character).await?;
        sqlx::query(
            "UPDATE native_jigsaw_user SET pieces_today=0,reset_date=CURRENT_DATE \
             WHERE character_name=$1 AND reset_date<>CURRENT_DATE",
        )
        .bind(character)
        .execute(self.pool)
        .await?;
        sqlx::query_as(
            "UPDATE native_jigsaw_user SET \
               piece_counts[$2 + 1]=LEAST(piece_counts[$2 + 1] + 1,255), \
               pieces_today=pieces_today+1,last_piece_at=NOW(),updated_at=NOW() \
             WHERE character_name=$1 AND pieces_today<8 \
               AND (last_piece_at IS NULL OR last_piece_at < NOW()-INTERVAL '900 milliseconds') \
             RETURNING piece_counts,reward_claimed",
        )
        .bind(character)
        .bind(piece as i32)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn claim_jigsaw(
        &self,
        character: &str,
        reward_index: i16,
    ) -> Result<Option<NativeRewardGrant>, sqlx::Error> {
        if !(0..9).contains(&reward_index) {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO native_jigsaw_user(character_name) VALUES($1) ON CONFLICT DO NOTHING",
        )
        .bind(character)
        .execute(&mut *tx)
        .await?;
        let state: NativeJigsawState = sqlx::query_as(
            "SELECT piece_counts,reward_claimed FROM native_jigsaw_user WHERE character_name=$1 FOR UPDATE",
        ).bind(character).fetch_one(&mut *tx).await?;
        let reward: Option<(i16,i32,i16)> = sqlx::query_as(
            "SELECT required_total,item_id,item_count FROM native_jigsaw_reward WHERE reward_index=$1",
        ).bind(reward_index).fetch_optional(&mut *tx).await?;
        let Some((required, item_id, item_count)) = reward else {
            tx.rollback().await?;
            return Ok(None);
        };
        let claimed = state
            .reward_claimed
            .get(reward_index as usize)
            .copied()
            .unwrap_or(true);
        let total: i32 = state.piece_counts.iter().map(|v| *v as i32).sum();
        if claimed || total < required as i32 {
            tx.rollback().await?;
            return Ok(None);
        }
        sqlx::query(
            "UPDATE native_jigsaw_user SET reward_claimed[$2+1]=TRUE,updated_at=NOW() WHERE character_name=$1",
        ).bind(character).bind(reward_index as i32).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some(NativeRewardGrant {
            item_id,
            item_count,
        }))
    }

    pub async fn coin_state(&self, character: &str) -> Result<NativeCoinState, sqlx::Error> {
        sqlx::query(
            "INSERT INTO native_coin_user(character_name) VALUES($1) ON CONFLICT DO NOTHING",
        )
        .bind(character)
        .execute(self.pool)
        .await?;
        sqlx::query_as("SELECT points,reward_claimed FROM native_coin_user WHERE character_name=$1")
            .bind(character)
            .fetch_one(self.pool)
            .await
    }

    pub async fn add_coin_point(
        &self,
        character: &str,
    ) -> Result<Option<NativeCoinState>, sqlx::Error> {
        self.coin_state(character).await?;
        sqlx::query_as(
            "UPDATE native_coin_user SET points=points+1,last_point_at=NOW(),updated_at=NOW() \
             WHERE character_name=$1 AND (last_point_at IS NULL OR last_point_at<NOW()-INTERVAL '60 seconds') \
             RETURNING points,reward_claimed",
        ).bind(character).fetch_optional(self.pool).await
    }

    pub async fn claim_coin(
        &self,
        character: &str,
        reward_index: i16,
    ) -> Result<Option<NativeRewardGrant>, sqlx::Error> {
        if !(0..10).contains(&reward_index) {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO native_coin_user(character_name) VALUES($1) ON CONFLICT DO NOTHING",
        )
        .bind(character)
        .execute(&mut *tx)
        .await?;
        let state: NativeCoinState = sqlx::query_as(
            "SELECT points,reward_claimed FROM native_coin_user WHERE character_name=$1 FOR UPDATE",
        )
        .bind(character)
        .fetch_one(&mut *tx)
        .await?;
        let reward: Option<(i32,i32,i16)> = sqlx::query_as(
            "SELECT required_points,item_id,item_count FROM native_coin_reward WHERE reward_index=$1",
        ).bind(reward_index).fetch_optional(&mut *tx).await?;
        let Some((required, item_id, item_count)) = reward else {
            tx.rollback().await?;
            return Ok(None);
        };
        let claimed = state
            .reward_claimed
            .get(reward_index as usize)
            .copied()
            .unwrap_or(true);
        if claimed || state.points < required {
            tx.rollback().await?;
            return Ok(None);
        }
        sqlx::query("UPDATE native_coin_user SET reward_claimed[$2+1]=TRUE,updated_at=NOW() WHERE character_name=$1")
            .bind(character).bind(reward_index as i32).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some(NativeRewardGrant {
            item_id,
            item_count,
        }))
    }

    pub async fn marble_state(&self, character: &str) -> Result<NativeMarbleState, sqlx::Error> {
        sqlx::query(
            "INSERT INTO native_marble_user(character_name) VALUES($1) ON CONFLICT DO NOTHING",
        )
        .bind(character)
        .execute(self.pool)
        .await?;
        sqlx::query(
            "UPDATE native_marble_user SET rolls_today=0,reset_date=CURRENT_DATE \
             WHERE character_name=$1 AND reset_date<>CURRENT_DATE",
        )
        .bind(character)
        .execute(self.pool)
        .await?;
        sqlx::query_as(
            "SELECT position,laps,rolls_today,treasure_claimed FROM native_marble_user WHERE character_name=$1",
        ).bind(character).fetch_one(self.pool).await
    }

    pub async fn roll_marble(
        &self,
        character: &str,
        die: i16,
    ) -> Result<Option<(NativeMarbleState, NativeMarbleTile)>, sqlx::Error> {
        if !(1..=6).contains(&die) {
            return Ok(None);
        }
        self.marble_state(character).await?;
        let mut tx = self.pool.begin().await?;
        let old: NativeMarbleState = sqlx::query_as(
            "SELECT position,laps,rolls_today,treasure_claimed FROM native_marble_user \
             WHERE character_name=$1 FOR UPDATE",
        )
        .bind(character)
        .fetch_one(&mut *tx)
        .await?;
        if old.rolls_today >= 12 {
            tx.rollback().await?;
            return Ok(None);
        }
        let raw = old.position as i32 + die as i32;
        let position = (raw % 24) as i16;
        let laps = old.laps + if raw >= 24 { 1 } else { 0 };
        let state: NativeMarbleState = sqlx::query_as(
            "UPDATE native_marble_user SET position=$2,laps=$3,rolls_today=rolls_today+1, \
             last_roll_at=NOW(),updated_at=NOW() WHERE character_name=$1 \
             RETURNING position,laps,rolls_today,treasure_claimed",
        )
        .bind(character)
        .bind(position)
        .bind(laps)
        .fetch_one(&mut *tx)
        .await?;
        let tile: NativeMarbleTile = sqlx::query_as(
            "SELECT board_index,item_id,item_count,tile_type FROM native_marble_board WHERE board_index=$1",
        ).bind(position).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some((state, tile)))
    }
}
