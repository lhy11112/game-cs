use crate::game::types::Rank;
use crate::player::types::PlayerProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single entry on the leaderboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub player_id: String,
    pub nickname: String,
    pub rank: Rank,
    pub kd_ratio: f64,
    pub win_rate: f64,
    pub headshot_rate: f64,
    pub total_matches: u64,
    pub total_kills: u64,
}

impl LeaderboardEntry {
    pub fn from_profile(profile: &PlayerProfile) -> Self {
        LeaderboardEntry {
            player_id: profile.id.clone(),
            nickname: profile.nickname.clone(),
            rank: profile.rank,
            kd_ratio: profile.kd_ratio(),
            win_rate: profile.win_rate(),
            headshot_rate: profile.headshot_rate(),
            total_matches: profile.total_matches,
            total_kills: profile.total_kills,
        }
    }
}

/// Sort criteria for the leaderboard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardSort {
    ByRank,
    ByKD,
    ByWinRate,
    ByHeadshotRate,
    ByTotalKills,
}

/// In-memory leaderboard for fast retrieval
pub struct Leaderboard {
    entries: HashMap<String, LeaderboardEntry>,
}

impl Leaderboard {
    pub fn new() -> Self {
        Leaderboard {
            entries: HashMap::new(),
        }
    }

    pub fn upsert(&mut self, profile: &PlayerProfile) {
        self.entries.insert(
            profile.id.clone(),
            LeaderboardEntry::from_profile(profile),
        );
    }

    pub fn remove(&mut self, player_id: &str) {
        self.entries.remove(player_id);
    }

    /// Retrieve top `n` players sorted by `sort`
    pub fn top(&self, n: usize, sort: LeaderboardSort) -> Vec<&LeaderboardEntry> {
        let mut entries: Vec<&LeaderboardEntry> = self.entries.values().collect();

        // Only include players with enough matches to be on the leaderboard
        entries.retain(|e| e.total_matches >= 10);

        match sort {
            LeaderboardSort::ByRank => {
                entries.sort_by(|a, b| b.rank.cmp(&a.rank));
            }
            LeaderboardSort::ByKD => {
                entries.sort_by(|a, b| b.kd_ratio.partial_cmp(&a.kd_ratio).unwrap());
            }
            LeaderboardSort::ByWinRate => {
                entries.sort_by(|a, b| b.win_rate.partial_cmp(&a.win_rate).unwrap());
            }
            LeaderboardSort::ByHeadshotRate => {
                entries.sort_by(|a, b| b.headshot_rate.partial_cmp(&a.headshot_rate).unwrap());
            }
            LeaderboardSort::ByTotalKills => {
                entries.sort_by(|a, b| b.total_kills.cmp(&a.total_kills));
            }
        }

        entries.truncate(n);
        entries
    }

    /// Get rank of a specific player (1-based, None if not ranked)
    pub fn player_rank_position(&self, player_id: &str, sort: LeaderboardSort) -> Option<usize> {
        let sorted = self.top(usize::MAX, sort);
        sorted.iter().position(|e| e.player_id == player_id).map(|i| i + 1)
    }

    pub fn total_players(&self) -> usize {
        self.entries.len()
    }
}

impl Default for Leaderboard {
    fn default() -> Self {
        Self::new()
    }
}
