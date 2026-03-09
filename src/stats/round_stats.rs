use crate::game::types::{KillEvent, RoundResult, Team};
use crate::player::types::Player;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-player stats for a single round
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerRoundStat {
    pub player_id: String,
    pub nickname: String,
    pub team: Team,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub damage_dealt: f32,
    pub headshots: u32,
    /// Whether this player planted the bomb
    pub planted_bomb: bool,
    /// Whether this player defused the bomb
    pub defused_bomb: bool,
}

impl PlayerRoundStat {
    pub fn headshot_rate(&self) -> f32 {
        if self.kills == 0 {
            0.0
        } else {
            self.headshots as f32 / self.kills as f32 * 100.0
        }
    }

    /// Contribution score (for MVP calculation)
    pub fn contribution_score(&self) -> f32 {
        let mut score = self.kills as f32 * 2.0
            + self.assists as f32 * 0.5
            + self.damage_dealt * 0.01;
        if self.planted_bomb {
            score += 3.0;
        }
        if self.defused_bomb {
            score += 5.0;
        }
        score
    }
}

/// Summary of a completed round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundStats {
    pub round_number: u32,
    pub result: RoundResult,
    pub player_stats: HashMap<String, PlayerRoundStat>,
    pub kill_log: Vec<KillEvent>,
    /// Duration of the round in seconds
    pub duration_secs: f32,
}

impl RoundStats {
    pub fn compile(
        round_number: u32,
        result: RoundResult,
        players: &HashMap<String, Player>,
        kill_log: Vec<KillEvent>,
        duration_secs: f32,
    ) -> Self {
        let player_stats: HashMap<String, PlayerRoundStat> = players
            .iter()
            .map(|(id, p)| {
                let headshots = kill_log
                    .iter()
                    .filter(|k| k.killer_id == *id && k.is_headshot)
                    .count() as u32;
                (
                    id.clone(),
                    PlayerRoundStat {
                        player_id: id.clone(),
                        nickname: p.profile.nickname.clone(),
                        team: p.team,
                        kills: p.round_kills,
                        deaths: p.round_deaths,
                        assists: p.round_assists,
                        damage_dealt: p.round_damage,
                        headshots,
                        planted_bomb: p.is_carrying_bomb,
                        defused_bomb: false,
                    },
                )
            })
            .collect();

        // Mark defuser
        if let RoundResult::CTWin(crate::game::types::CTWinReason::BombDefused) = result {
            // The defuser info would come from the bomb state - mark it via external call
        }

        RoundStats {
            round_number,
            result,
            player_stats,
            kill_log,
            duration_secs,
        }
    }

    /// Get the MVP of this round (highest contribution score)
    pub fn mvp(&self) -> Option<&PlayerRoundStat> {
        self.player_stats
            .values()
            .max_by(|a, b| a.contribution_score().partial_cmp(&b.contribution_score()).unwrap())
    }

    /// Get stats sorted by kills (descending)
    pub fn scoreboard(&self) -> Vec<&PlayerRoundStat> {
        let mut stats: Vec<&PlayerRoundStat> = self.player_stats.values().collect();
        stats.sort_by(|a, b| b.kills.cmp(&a.kills).then(b.damage_dealt.partial_cmp(&a.damage_dealt).unwrap()));
        stats
    }
}
