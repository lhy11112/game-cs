use crate::economy::rewards::RoundRewardCalculator;
use crate::game::drops::DropZone;
use crate::game::error::GameError;
use crate::game::round::Round;
use crate::game::types::{
    KillEvent, MatchConfig, RoundMoneyRewards, RoundResult, RoundSummary, Team,
};
use crate::map::bomb::{BombAction, BombPhase};
use crate::map::types::MapDef;
use crate::player::types::Player;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// History of completed rounds (for scoreboard / HUD ring)
    pub round_history: Vec<RoundSummary>,
    /// Weapons currently on the ground
    pub drops: DropZone,

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
            round_history: Vec::new(),
            drops: DropZone::new(),
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

        // Clear dropped weapons from previous round
        self.drops.clear();

        // Reset player state for the new round
        for player in self.players.values_mut() {
            let spawn = self.map.spawn_for(player.team, self.round_number);
            player.reset_for_round(spawn);
        }

        // Assign bomb carrier: random T-side player (CS 1.5 mechanic)
        self.assign_bomb_carrier();

        self.current_round = Some(round);
        log::info!("Round {} started", self.round_number);
    }

    /// Randomly select one T-side player to carry the bomb
    fn assign_bomb_carrier(&mut self) {
        let mut rng = rand::thread_rng();
        let t_ids: Vec<String> = self
            .players
            .values()
            .filter(|p| p.team == Team::T)
            .map(|p| p.profile.id.clone())
            .collect();

        if let Some(carrier_id) = t_ids.choose(&mut rng) {
            if let Some(p) = self.players.get_mut(carrier_id) {
                p.is_carrying_bomb = true;
                log::info!("Bomb carrier: {}", p.profile.nickname);
            }
        }
    }

    /// Advance match time by `delta_secs`
    pub fn tick(&mut self, delta_secs: f32) -> Option<RoundResult> {
        if self.state != MatchState::Live {
            return None;
        }
        self.match_time_secs += delta_secs as f64;

        // Round tick needs mutable players for grenade effects
        let round_ended = if let Some(round) = &mut self.current_round {
            round.tick(delta_secs, &mut self.players, &self.config, &self.map)
        } else {
            None
        };

        if let Some(result) = round_ended {
            self.handle_round_end(result);
            return Some(result);
        }
        None
    }

    fn handle_round_end(&mut self, result: RoundResult) {
        // Collect MVP info before mutating
        let mvp_id = self.current_round.as_ref().and_then(|r| r.mvp_id.clone());
        let mvp_nickname = mvp_id.as_ref().and_then(|id| {
            self.players.get(id).map(|p| p.profile.nickname.clone())
        });

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

        // Record round summary
        self.round_history.push(RoundSummary {
            round_number: self.round_number,
            result,
            mvp_id,
            mvp_nickname,
            ct_score: self.score.ct,
            t_score: self.score.t,
        });

        // Drop weapons from dead players
        for player in self.players.values() {
            if !player.is_alive() {
                if let Some(primary) = &player.inventory.primary {
                    self.drops.add(
                        player.profile.id.clone(),
                        primary.clone(),
                        player.position,
                    );
                }
            }
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
                Team::T  => t_reward.total_reward,
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
            log::info!("Halftime! Teams swapped. Money reset to ${}.", self.config.start_money);
            return;
        }

        // Start next round
        self.begin_round();
    }

    /// Swap CT and T sides (halftime) and reset money / inventory to pistol-round values
    fn swap_teams(&mut self) {
        let start = self.config.start_money;
        for player in self.players.values_mut() {
            player.team = match player.team {
                Team::CT => Team::T,
                Team::T  => Team::CT,
                t => t,
            };
            // CS halftime rule: money resets to starting amount
            player.money = start;
            // Inventory resets to knife only (fresh half)
            player.inventory.clear();
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
        let kill_reward = self.config_kill_reward(weapon);
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
        cat.get(weapon_name).map(|w| w.kill_reward).unwrap_or(300)
    }

    /// Apply a bomb action (plant/defuse/cancel) to the current round
    pub fn apply_bomb_action(&mut self, action: BombAction) -> Result<(), GameError> {
        let round = self
            .current_round
            .as_mut()
            .ok_or_else(|| GameError::InvalidAction("No active round".into()))?;
        let map = &self.map;
        round.bomb.apply(action, map)?;
        if round.bomb.phase == BombPhase::Planted {
            round.on_bomb_planted();
        }
        Ok(())
    }

    pub fn is_ended(&self) -> bool {
        self.state == MatchState::Ended
    }

    pub fn winner(&self) -> Option<Team> {
        if self.state != MatchState::Ended { return None; }
        if self.score.ct > self.score.t { Some(Team::CT) }
        else if self.score.t > self.score.ct { Some(Team::T) }
        else { None }
    }

    /// Latest round MVP summary (if any)
    pub fn last_mvp(&self) -> Option<&RoundSummary> {
        self.round_history.last()
    }
}
