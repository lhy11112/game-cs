use crate::game::types::{Rank, RoundResult, Team};
use crate::stats::round_stats::RoundStats;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MVP designation for the entire match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVP {
    pub player_id: String,
    pub nickname: String,
    pub score: f32,
}

/// Aggregated per-player stats across all rounds
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerMatchStat {
    pub player_id: String,
    pub nickname: String,
    pub team: Team,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub headshots: u32,
    pub damage_dealt: f32,
    pub bomb_plants: u32,
    pub bomb_defuses: u32,
    pub mvp_count: u32,
    pub rounds_played: u32,
}

impl PlayerMatchStat {
    pub fn kd_ratio(&self) -> f64 {
        if self.deaths == 0 {
            return self.kills as f64;
        }
        self.kills as f64 / self.deaths as f64
    }

    pub fn headshot_rate(&self) -> f64 {
        if self.kills == 0 {
            return 0.0;
        }
        self.headshots as f64 / self.kills as f64 * 100.0
    }

    pub fn average_damage(&self) -> f64 {
        if self.rounds_played == 0 {
            return 0.0;
        }
        self.damage_dealt as f64 / self.rounds_played as f64
    }

    /// Contribution score for full-match MVP
    pub fn contribution_score(&self) -> f64 {
        self.kills as f64 * 2.0
            + self.assists as f64 * 0.5
            + self.damage_dealt as f64 * 0.005
            + self.bomb_plants as f64 * 3.0
            + self.bomb_defuses as f64 * 5.0
            + self.mvp_count as f64 * 2.0
    }
}

/// Full post-match statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchStats {
    pub match_id: String,
    pub map_name: String,
    pub ct_score: u32,
    pub t_score: u32,
    pub winner: Option<Team>,
    pub total_rounds: u32,
    pub player_stats: HashMap<String, PlayerMatchStat>,
    pub round_history: Vec<RoundResult>,
    /// Wall-clock duration in seconds
    pub duration_secs: f64,
    pub mvp: Option<MVP>,
}

impl MatchStats {
    pub fn compile(
        match_id: String,
        map_name: String,
        ct_score: u32,
        t_score: u32,
        winner: Option<Team>,
        round_stats_list: Vec<RoundStats>,
        duration_secs: f64,
    ) -> Self {
        let mut player_stats: HashMap<String, PlayerMatchStat> = HashMap::new();
        let mut round_history = Vec::new();

        for (_round_idx, round) in round_stats_list.iter().enumerate() {
            round_history.push(round.result);

            // Determine round MVP
            let round_mvp_id = round.mvp().map(|s| s.player_id.clone());

            for (pid, prs) in &round.player_stats {
                let entry = player_stats.entry(pid.clone()).or_insert_with(|| PlayerMatchStat {
                    player_id: pid.clone(),
                    nickname: prs.nickname.clone(),
                    team: prs.team,
                    ..Default::default()
                });

                entry.kills += prs.kills;
                entry.deaths += prs.deaths;
                entry.assists += prs.assists;
                entry.headshots += prs.headshots;
                entry.damage_dealt += prs.damage_dealt;
                if prs.planted_bomb {
                    entry.bomb_plants += 1;
                }
                if prs.defused_bomb {
                    entry.bomb_defuses += 1;
                }
                entry.rounds_played += 1;

                if round_mvp_id.as_deref() == Some(pid.as_str()) {
                    entry.mvp_count += 1;
                }
            }
        }

        // Compute full-match MVP
        let mvp = player_stats
            .values()
            .max_by(|a, b| {
                a.contribution_score()
                    .partial_cmp(&b.contribution_score())
                    .unwrap()
            })
            .map(|s| MVP {
                player_id: s.player_id.clone(),
                nickname: s.nickname.clone(),
                score: s.contribution_score() as f32,
            });

        MatchStats {
            match_id,
            map_name,
            ct_score,
            t_score,
            winner,
            total_rounds: round_stats_list.len() as u32,
            player_stats,
            round_history,
            duration_secs,
            mvp,
        }
    }

    /// Return sorted scoreboard (kills desc, then damage desc)
    pub fn scoreboard(&self) -> Vec<&PlayerMatchStat> {
        let mut stats: Vec<&PlayerMatchStat> = self.player_stats.values().collect();
        stats.sort_by(|a, b| {
            b.kills
                .cmp(&a.kills)
                .then(b.damage_dealt.partial_cmp(&a.damage_dealt).unwrap())
        });
        stats
    }

    /// Compute rank change for a player based on win/loss and performance
    pub fn rank_change(&self, player_id: &str, _current_rank: Rank) -> i32 {
        let stats = match self.player_stats.get(player_id) {
            None => return 0,
            Some(s) => s,
        };

        let won = match (self.winner, stats.team) {
            (Some(winning_team), team) if winning_team == team => true,
            _ => false,
        };

        let base: i32 = if won { 1 } else { -1 };

        // Performance bonus/penalty
        let kd = stats.kd_ratio();
        let performance_bonus: i32 = if kd > 1.5 {
            1
        } else if kd < 0.5 {
            -1
        } else {
            0
        };

        base + performance_bonus
    }
}
