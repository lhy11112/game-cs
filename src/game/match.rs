use crate::economy::rewards::RoundRewardCalculator;
use crate::game::error::GameError;
use crate::game::round::Round;
use crate::game::types::{KillEvent, MatchConfig, RoundMoneyRewards, RoundResult, Team};
use crate::map::bomb::{BombAction, BombPhase};
use crate::map::types::MapDef;
use crate::player::types::Player;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Match-level score tracker
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Score {
    pub ct: u32,
    pub t: u32,
}

/// Overall match state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchState {
    WaitingForPlayers,
    Warmup,
    Live,
    /// Halftime break
    Halftime,
    Ended,
}

/// Full match, containing all rounds and players
pub struct Match {
    pub id: String,
    pub config: MatchConfig,
    pub map: MapDef,
    pub players: HashMap<String, Player>,
    pub score: Score,
    pub state: MatchState,
    pub current_round: Option<Round>,
    pub round_number: u32,
    pub match_time_secs: f64,
    /// Consecutive losses per team (for loss bonus)
    ct_consecutive_losses: u32,
    t_consecutive_losses: u32,
    reward_calculator: RoundRewardCalculator,
}

impl Match {
    pub fn new(config: MatchConfig, map: MapDef) -> Self {
        Match {
            id: Uuid::new_v4().to_string(),
            reward_calculator: RoundRewardCalculator::new(RoundMoneyRewards::default()),
            config,
            map,
            players: HashMap::new(),
            score: Score::default(),
            state: MatchState::WaitingForPlayers,
            current_round: None,
            round_number: 0,
            match_time_secs: 0.0,
            ct_consecutive_losses: 0,
            t_consecutive_losses: 0,
        }
    }

    pub fn add_player(&mut self, player: Player) -> Result<(), GameError> {
        if self.state != MatchState::WaitingForPlayers && self.state != MatchState::Warmup {
            return Err(GameError::InvalidAction("Match already started".into()));
        }
        self.players.insert(player.profile.id.clone(), player);
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), GameError> {
        if self.state != MatchState::WaitingForPlayers {
            return Err(GameError::InvalidAction("Match already started".into()));
        }
        self.state = MatchState::Live;
        self.begin_round();
        Ok(())
    }

    fn begin_round(&mut self) {
        self.round_number += 1;
        let round = Round::new(self.round_number, &self.config);

        // Reset player state for the new round
        for player in self.players.values_mut() {
            let spawn = self.map.spawn_for(player.team, self.round_number);
            player.reset_for_round(spawn);
        }

        self.current_round = Some(round);
        log::info!("Round {} started", self.round_number);
    }

    /// Advance match time by `delta_secs`
    pub fn tick(&mut self, delta_secs: f32) -> Option<RoundResult> {
        if self.state != MatchState::Live {
            return None;
        }
        self.match_time_secs += delta_secs as f64;

        let round_ended = {
            if let Some(round) = &mut self.current_round {
                round.tick(delta_secs, &self.players, &self.config, &self.map)
            } else {
                None
            }
        };

        if let Some(result) = round_ended {
            self.handle_round_end(result);
            return Some(result);
        }
        None
    }

    fn handle_round_end(&mut self, result: RoundResult) {
        // Update score
        match result {
            RoundResult::CTWin(_) => {
                self.score.ct += 1;
                self.ct_consecutive_losses = 0;
                self.t_consecutive_losses += 1;
            }
            RoundResult::TWin(_) => {
                self.score.t += 1;
                self.t_consecutive_losses = 0;
                self.ct_consecutive_losses += 1;
            }
            RoundResult::InProgress => {}
        }

        // Distribute end-of-round money
        let (ct_reward, t_reward) = self.reward_calculator.compute(
            result,
            self.ct_consecutive_losses,
            self.t_consecutive_losses,
        );

        let max_money = self.config.max_money;
        for player in self.players.values_mut() {
            let reward = match player.team {
                Team::CT => ct_reward.total_reward,
                Team::T => t_reward.total_reward,
                Team::Spectator => 0,
            };
            player.add_money(reward, max_money);
        }

        log::info!(
            "Round {} ended: {:?} | Score CT:{} T:{}",
            self.round_number, result, self.score.ct, self.score.t
        );

        // Check match win condition
        if self.score.ct >= self.config.rounds_to_win
            || self.score.t >= self.config.rounds_to_win
            || self.round_number >= self.config.max_rounds
        {
            self.state = MatchState::Ended;
            log::info!("Match ended. Final: CT:{} T:{}", self.score.ct, self.score.t);
            return;
        }

        // Halftime
        let half = self.config.max_rounds / 2;
        if self.round_number == half {
            self.state = MatchState::Halftime;
            self.swap_teams();
            log::info!("Halftime! Teams have been swapped.");
            return;
        }

        // Start next round
        self.begin_round();
    }

    /// Swap CT and T sides (halftime)
    fn swap_teams(&mut self) {
        for player in self.players.values_mut() {
            player.team = match player.team {
                Team::CT => Team::T,
                Team::T => Team::CT,
                t => t,
            };
        }
    }

    /// Resume from halftime
    pub fn resume_from_halftime(&mut self) -> Result<(), GameError> {
        if self.state != MatchState::Halftime {
            return Err(GameError::InvalidAction("Not at halftime".into()));
        }
        self.state = MatchState::Live;
        self.begin_round();
        Ok(())
    }

    /// Record a kill event in the current round
    pub fn record_kill(
        &mut self,
        killer_id: &str,
        victim_id: &str,
        weapon: &str,
        hit_zone: crate::game::types::HitZone,
        is_headshot: bool,
        timestamp_ms: u64,
    ) -> Result<(), GameError> {
        // Award kill money to the killer
        let kill_reward = {
            let killer = self
                .players
                .get(killer_id)
                .ok_or_else(|| GameError::PlayerNotFound(killer_id.into()))?;
            // Look up weapon kill reward
            // Default to standard kill reward if weapon not found
            let _ = killer;
            self.config_kill_reward(weapon)
        };

        let max_money = self.config.max_money;
        if let Some(killer) = self.players.get_mut(killer_id) {
            killer.add_money(kill_reward, max_money);
            killer.round_kills += 1;
        }
        if let Some(victim) = self.players.get_mut(victim_id) {
            victim.round_deaths += 1;
        }

        let event = KillEvent {
            killer_id: killer_id.into(),
            victim_id: victim_id.into(),
            weapon_name: weapon.into(),
            hit_zone,
            is_headshot,
            timestamp_ms,
        };

        if let Some(round) = &mut self.current_round {
            round.record_kill(event);
        }

        Ok(())
    }

    fn config_kill_reward(&self, weapon_name: &str) -> u32 {
        use crate::weapons::catalog::WeaponCatalog;
        let cat = WeaponCatalog::new();
        cat.get(weapon_name)
            .map(|w| w.kill_reward)
            .unwrap_or(300)
    }

    /// Apply a bomb action (plant/defuse/cancel) to the current round
    pub fn apply_bomb_action(
        &mut self,
        action: BombAction,
    ) -> Result<(), GameError> {
        let round = self
            .current_round
            .as_mut()
            .ok_or_else(|| GameError::InvalidAction("No active round".into()))?;

        let map = &self.map;
        round.bomb.apply(action, map)?;

        // Transition to BombPlanted phase if bomb just finished planting
        if round.bomb.phase == BombPhase::Planted {
            round.on_bomb_planted();
        }

        Ok(())
    }

    pub fn is_ended(&self) -> bool {
        self.state == MatchState::Ended
    }

    pub fn winner(&self) -> Option<Team> {
        if self.state != MatchState::Ended {
            return None;
        }
        if self.score.ct > self.score.t {
            Some(Team::CT)
        } else if self.score.t > self.score.ct {
            Some(Team::T)
        } else {
            None // Tie
        }
    }
}
