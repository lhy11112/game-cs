use crate::game::types::{CTWinReason, KillEvent, MatchConfig, RoundPhase, RoundResult, Team, TWinReason};
use crate::map::bomb::BombState;
use crate::map::types::MapDef;
use crate::player::types::Player;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// All state for a single round
#[derive(Debug, Serialize, Deserialize)]
pub struct Round {
    pub number: u32,
    pub phase: RoundPhase,
    /// Time remaining in current phase (seconds)
    pub time_remaining_secs: f32,
    pub result: RoundResult,
    pub kill_log: Vec<KillEvent>,
    pub bomb: BombState,
}

impl Round {
    pub fn new(number: u32, config: &MatchConfig) -> Self {
        Round {
            number,
            phase: RoundPhase::FreezeTime,
            time_remaining_secs: config.freeze_time_secs as f32,
            result: RoundResult::InProgress,
            kill_log: Vec::new(),
            bomb: BombState::new(
                config.bomb_timer_secs as f32,
                config.plant_time_secs,
                config.defuse_time_secs,
                config.defuse_kit_time_secs,
            ),
        }
    }

    pub fn is_over(&self) -> bool {
        self.phase == RoundPhase::Ended
    }

    /// Advance the round clock by `delta_secs`.
    ///
    /// Returns the round result if the round just ended.
    pub fn tick(
        &mut self,
        delta_secs: f32,
        players: &HashMap<String, Player>,
        config: &MatchConfig,
        _map: &MapDef,
    ) -> Option<RoundResult> {
        if self.phase == RoundPhase::Ended {
            return Some(self.result);
        }

        self.time_remaining_secs -= delta_secs;

        match self.phase {
            RoundPhase::FreezeTime => {
                if self.time_remaining_secs <= 0.0 {
                    self.phase = RoundPhase::Live;
                    self.time_remaining_secs = config.round_time_secs as f32;
                }
                None
            }

            RoundPhase::Live => {
                // Check elimination win conditions
                if let Some(result) = self.check_elimination(players) {
                    return self.end_round(result);
                }

                // Check time expiry (no plant)
                if self.time_remaining_secs <= 0.0 {
                    return self.end_round(RoundResult::CTWin(CTWinReason::TimeExpired));
                }

                None
            }

            RoundPhase::BombPlanted => {
                // Advance bomb timer
                if let Some(result) = self.bomb.tick(delta_secs) {
                    return self.end_round(result);
                }

                // Check if all Ts are dead (bomb still ticking - CT can still defuse)
                // Check if all CTs are dead
                let ct_alive = players.values().any(|p| p.team == Team::CT && p.is_alive());
                if !ct_alive {
                    // CTs eliminated but bomb still ticking - let it tick
                }

                // Check elimination win (T's all dead after bomb plant - CT win on defuse or T win on explosion)
                let t_alive = players.values().any(|p| p.team == Team::T && p.is_alive());
                let _ = t_alive; // bomb continues regardless of T deaths after plant

                None
            }

            RoundPhase::Ended => Some(self.result),
        }
    }

    /// Record a bomb plant transition
    pub fn on_bomb_planted(&mut self) {
        if self.phase == RoundPhase::Live {
            self.phase = RoundPhase::BombPlanted;
        }
    }

    /// Check if one team has been fully eliminated
    fn check_elimination(&self, players: &HashMap<String, Player>) -> Option<RoundResult> {
        let ct_alive = players.values().any(|p| p.team == Team::CT && p.is_alive());
        let t_alive = players.values().any(|p| p.team == Team::T && p.is_alive());

        if !ct_alive {
            return Some(RoundResult::TWin(TWinReason::CTsEliminated));
        }
        if !t_alive {
            return Some(RoundResult::CTWin(CTWinReason::TerrorsEliminated));
        }
        None
    }

    fn end_round(&mut self, result: RoundResult) -> Option<RoundResult> {
        self.result = result;
        self.phase = RoundPhase::Ended;
        Some(result)
    }

    pub fn record_kill(&mut self, event: KillEvent) {
        self.kill_log.push(event);
    }
}
