use crate::game::error::GameError;
use crate::game::types::{RoundResult, TWinReason, Vec3};
use crate::map::types::MapDef;
use serde::{Deserialize, Serialize};

/// Phase of the bomb in a round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BombPhase {
    /// Bomb has not been planted
    NotPlanted,
    /// A player is in the process of planting
    Planting,
    /// Bomb is planted and counting down
    Planted,
    /// A player is in the process of defusing
    Defusing,
    /// Bomb was defused successfully
    Defused,
    /// Bomb exploded
    Exploded,
}

/// Live bomb state for the current round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BombState {
    pub phase: BombPhase,
    pub position: Option<Vec3>,
    /// Planter player ID
    pub planter_id: Option<String>,
    /// Defuser player ID
    pub defuser_id: Option<String>,
    /// Time remaining in current action (planting / defusing / detonation)
    pub timer_secs: f32,
    /// Full bomb timer (set on plant)
    pub detonation_timer_secs: f32,
    pub plant_time_secs: f32,
    pub defuse_time_secs: f32,
    pub defuse_kit_time_secs: f32,
}

/// An action that can be requested of the bomb state machine
pub enum BombAction {
    StartPlant { player_id: String, position: Vec3 },
    CancelPlant,
    StartDefuse { player_id: String, has_kit: bool },
    CancelDefuse,
}

impl BombState {
    pub fn new(detonation_timer_secs: f32, plant_time_secs: f32, defuse_time_secs: f32, defuse_kit_time_secs: f32) -> Self {
        BombState {
            phase: BombPhase::NotPlanted,
            position: None,
            planter_id: None,
            defuser_id: None,
            timer_secs: 0.0,
            detonation_timer_secs,
            plant_time_secs,
            defuse_time_secs,
            defuse_kit_time_secs,
        }
    }

    pub fn is_planted(&self) -> bool {
        self.phase == BombPhase::Planted || self.phase == BombPhase::Defusing
    }

    pub fn is_done(&self) -> bool {
        matches!(self.phase, BombPhase::Defused | BombPhase::Exploded)
    }

    /// Apply an action to the bomb state machine
    pub fn apply(
        &mut self,
        action: BombAction,
        map: &MapDef,
    ) -> Result<(), GameError> {
        match action {
            BombAction::StartPlant { player_id, position } => {
                if self.phase != BombPhase::NotPlanted {
                    return Err(GameError::BombAlreadyPlanted);
                }
                if map.bomb_site_at(&position).is_none() {
                    return Err(GameError::NotAtBombSite);
                }
                self.phase = BombPhase::Planting;
                self.planter_id = Some(player_id);
                self.position = Some(position);
                self.timer_secs = self.plant_time_secs;
                Ok(())
            }
            BombAction::CancelPlant => {
                if self.phase == BombPhase::Planting {
                    self.phase = BombPhase::NotPlanted;
                    self.planter_id = None;
                    self.position = None;
                    self.timer_secs = 0.0;
                }
                Ok(())
            }
            BombAction::StartDefuse { player_id, has_kit } => {
                if self.phase != BombPhase::Planted {
                    return Err(GameError::BombNotPlanted);
                }
                let defuse_time = if has_kit {
                    self.defuse_kit_time_secs
                } else {
                    self.defuse_time_secs
                };
                self.phase = BombPhase::Defusing;
                self.defuser_id = Some(player_id);
                self.timer_secs = defuse_time;
                Ok(())
            }
            BombAction::CancelDefuse => {
                if self.phase == BombPhase::Defusing {
                    self.phase = BombPhase::Planted;
                    self.defuser_id = None;
                    // Timer keeps the detonation countdown, not defuse timer
                    // Nothing needed here - detonation timer was never paused
                }
                Ok(())
            }
        }
    }

    /// Advance the bomb timer by `delta_secs`.
    /// Returns `Some(RoundResult)` if the bomb caused the round to end.
    pub fn tick(&mut self, delta_secs: f32) -> Option<RoundResult> {
        match self.phase {
            BombPhase::Planting => {
                self.timer_secs -= delta_secs;
                if self.timer_secs <= 0.0 {
                    self.phase = BombPhase::Planted;
                    self.timer_secs = self.detonation_timer_secs;
                    log::info!(
                        "Bomb planted at {:?} by {:?}",
                        self.position,
                        self.planter_id
                    );
                }
                None
            }
            BombPhase::Planted => {
                self.timer_secs -= delta_secs;
                if self.timer_secs <= 0.0 {
                    self.phase = BombPhase::Exploded;
                    log::info!("Bomb exploded!");
                    return Some(RoundResult::TWin(TWinReason::BombExploded));
                }
                None
            }
            BombPhase::Defusing => {
                self.timer_secs -= delta_secs;
                if self.timer_secs <= 0.0 {
                    self.phase = BombPhase::Defused;
                    log::info!("Bomb defused by {:?}", self.defuser_id);
                    return Some(RoundResult::CTWin(
                        crate::game::types::CTWinReason::BombDefused,
                    ));
                }
                // Detonation timer keeps ticking while defusing
                None
            }
            _ => None,
        }
    }

    /// Check whether player is close enough to the bomb to interact with it
    pub fn player_in_range(player_pos: &Vec3, bomb_pos: &Vec3, range: f32) -> bool {
        player_pos.distance_to(bomb_pos) <= range
    }
}
