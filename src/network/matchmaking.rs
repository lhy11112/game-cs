use crate::game::types::{GameMode, Rank};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A player entry in the matchmaking queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakingEntry {
    pub player_id: String,
    pub nickname: String,
    pub rank: Rank,
    pub game_mode: GameMode,
    pub queued_at: DateTime<Utc>,
    /// Estimated ping in ms (used for region matching)
    pub estimated_ping_ms: u32,
}

impl MatchmakingEntry {
    pub fn new(player_id: String, nickname: String, rank: Rank, game_mode: GameMode, ping_ms: u32) -> Self {
        MatchmakingEntry {
            player_id,
            nickname,
            rank,
            game_mode,
            queued_at: Utc::now(),
            estimated_ping_ms: ping_ms,
        }
    }

    /// How long the player has been waiting, in seconds
    pub fn wait_secs(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.queued_at)
            .num_seconds()
    }
}

/// The result of a successful matchmaking session
#[derive(Debug, Serialize, Deserialize)]
pub struct MatchmakingResult {
    pub map_name: String,
    pub game_mode: GameMode,
    pub player_ids: Vec<String>,
}

/// Matchmaking timeout in seconds before the match is cancelled
const MATCHMAKING_TIMEOUT_SECS: i64 = 60;

/// Players per match (5v5)
const PLAYERS_PER_MATCH: usize = 10;

/// Maximum rank difference for a balanced match
const MAX_RANK_DIFF: i32 = 3;

/// The global matchmaking queue
pub struct MatchmakingQueue {
    entries: HashMap<String, MatchmakingEntry>,
}

impl MatchmakingQueue {
    pub fn new() -> Self {
        MatchmakingQueue {
            entries: HashMap::new(),
        }
    }

    /// Add a player to the queue. Replacing an existing entry if present.
    pub fn enqueue(&mut self, entry: MatchmakingEntry) {
        log::debug!("Player {} joined matchmaking queue ({:?})", entry.player_id, entry.game_mode);
        self.entries.insert(entry.player_id.clone(), entry);
    }

    /// Remove a player from the queue
    pub fn dequeue(&mut self, player_id: &str) -> bool {
        self.entries.remove(player_id).is_some()
    }

    /// Check if a player is in the queue
    pub fn is_queued(&self, player_id: &str) -> bool {
        self.entries.contains_key(player_id)
    }

    /// Attempt to form a match from queued players.
    ///
    /// Priority order: rank proximity → ping → wait time.
    /// Returns a `MatchmakingResult` if enough players were found, otherwise None.
    pub fn try_form_match(&mut self, preferred_maps: &[&str]) -> Option<MatchmakingResult> {
        // Remove timed-out entries
        self.prune_timed_out();

        // Group by game mode
        for mode in &[GameMode::BombDefusal, GameMode::TeamDeathmatch, GameMode::Deathmatch] {
            let mut candidates: Vec<&MatchmakingEntry> = self
                .entries
                .values()
                .filter(|e| &e.game_mode == mode)
                .collect();

            if candidates.len() < PLAYERS_PER_MATCH {
                continue;
            }

            // Sort by rank, then by wait time (longest waiting first for same rank)
            candidates.sort_by(|a, b| {
                let ra = a.rank as i32;
                let rb = b.rank as i32;
                ra.cmp(&rb).then_with(|| b.queued_at.cmp(&a.queued_at))
            });

            // Greedy window search: find a window of PLAYERS_PER_MATCH with rank spread ≤ MAX_RANK_DIFF
            for window_start in 0..=(candidates.len() - PLAYERS_PER_MATCH) {
                let window = &candidates[window_start..window_start + PLAYERS_PER_MATCH];
                let min_rank = window[0].rank as i32;
                let max_rank = window[PLAYERS_PER_MATCH - 1].rank as i32;

                if max_rank - min_rank <= MAX_RANK_DIFF {
                    let player_ids: Vec<String> =
                        window.iter().map(|e| e.player_id.clone()).collect();

                    // Remove matched players from queue
                    for pid in &player_ids {
                        self.entries.remove(pid);
                    }

                    let map = preferred_maps
                        .first()
                        .copied()
                        .unwrap_or("de_dust2");

                    log::info!(
                        "Match formed: {} players on {} ({:?})",
                        player_ids.len(), map, mode
                    );

                    return Some(MatchmakingResult {
                        map_name: map.into(),
                        game_mode: *mode,
                        player_ids,
                    });
                }
            }

            // Expand rank tolerance for long-waiting players (>30s)
            let long_waiters: Vec<&MatchmakingEntry> = candidates
                .iter()
                .filter(|e| e.wait_secs() > 30)
                .copied()
                .collect();

            if long_waiters.len() >= PLAYERS_PER_MATCH {
                let player_ids: Vec<String> = long_waiters[..PLAYERS_PER_MATCH]
                    .iter()
                    .map(|e| e.player_id.clone())
                    .collect();

                for pid in &player_ids {
                    self.entries.remove(pid);
                }

                let map = preferred_maps.first().copied().unwrap_or("de_dust2");
                return Some(MatchmakingResult {
                    map_name: map.into(),
                    game_mode: *mode,
                    player_ids,
                });
            }
        }

        None
    }

    /// Remove entries that have exceeded the matchmaking timeout
    fn prune_timed_out(&mut self) {
        let timed_out: Vec<String> = self
            .entries
            .values()
            .filter(|e| e.wait_secs() > MATCHMAKING_TIMEOUT_SECS)
            .map(|e| e.player_id.clone())
            .collect();

        for pid in timed_out {
            log::debug!("Player {} timed out of matchmaking queue", pid);
            self.entries.remove(&pid);
        }
    }

    pub fn queue_size(&self) -> usize {
        self.entries.len()
    }

    pub fn estimated_wait_secs(&self, rank: Rank, mode: GameMode) -> u32 {
        let similar_count = self
            .entries
            .values()
            .filter(|e| e.game_mode == mode && (e.rank as i32 - rank as i32).abs() <= MAX_RANK_DIFF)
            .count();

        // Rough estimate based on queue population
        if similar_count >= PLAYERS_PER_MATCH - 1 {
            5
        } else if similar_count >= PLAYERS_PER_MATCH / 2 {
            20
        } else {
            45
        }
    }
}

impl Default for MatchmakingQueue {
    fn default() -> Self {
        Self::new()
    }
}
