use crate::game::grenade::{GrenadeSim, GrenadeType};
use crate::game::types::{
    CTWinReason, KillEvent, MatchConfig, RoundPhase, RoundResult, Team, TWinReason, Vec3,
};
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
    pub time_remaining_secs: f32,
    pub result: RoundResult,
    pub kill_log: Vec<KillEvent>,
    pub bomb: BombState,
    pub grenades: GrenadeSim,
    pub mvp_id: Option<String>,
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
            grenades: GrenadeSim::new(),
            mvp_id: None,
        }
    }

    pub fn is_over(&self) -> bool {
        self.phase == RoundPhase::Ended
    }

    pub fn throw_grenade(&mut self, thrower_id: String, kind: GrenadeType, position: Vec3) -> u32 {
        self.grenades.throw(kind, thrower_id, position)
    }

    pub fn tick(
        &mut self,
        delta_secs: f32,
        players: &mut HashMap<String, Player>,
        config: &MatchConfig,
        _map: &MapDef,
    ) -> Option<RoundResult> {
        if self.phase == RoundPhase::Ended {
            return Some(self.result);
        }

        self.time_remaining_secs -= delta_secs;

        // Snapshot for grenade range checks (avoids borrow conflict)
        let snapshot: HashMap<String, Player> = players
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let nade_events = self.grenades.tick(delta_secs, &snapshot);
        for event in nade_events {
            for (pid, dmg) in &event.damage_map {
                if let Some(p) = players.get_mut(pid) { p.take_damage(*dmg); }
            }
            for (pid, blind_secs) in &event.blind_map {
                if let Some(p) = players.get_mut(pid) {
                    p.blind_timer = p.blind_timer.max(*blind_secs);
                }
            }
        }

        for p in players.values_mut() {
            if p.blind_timer > 0.0 { p.blind_timer = (p.blind_timer - delta_secs).max(0.0); }
            if p.on_fire_timer > 0.0 { p.on_fire_timer = (p.on_fire_timer - delta_secs).max(0.0); }
        }

        match self.phase {
            RoundPhase::FreezeTime => {
                if self.time_remaining_secs <= 0.0 {
                    self.phase = RoundPhase::Live;
                    self.time_remaining_secs = config.round_time_secs as f32;
                }
                None
            }
            RoundPhase::Live => {
                if let Some(result) = self.check_elimination(players) {
                    return self.end_round(result, players);
                }
                if self.time_remaining_secs <= 0.0 {
                    return self.end_round(RoundResult::CTWin(CTWinReason::TimeExpired), players);
                }
                None
            }
            RoundPhase::BombPlanted => {
                if let Some(result) = self.bomb.tick(delta_secs) {
                    return self.end_round(result, players);
                }
                None
            }
            RoundPhase::Ended => Some(self.result),
        }
    }

    pub fn on_bomb_planted(&mut self) {
        if self.phase == RoundPhase::Live {
            self.phase = RoundPhase::BombPlanted;
        }
    }

    fn check_elimination(&self, players: &HashMap<String, Player>) -> Option<RoundResult> {
        let ct = players.values().any(|p| p.team == Team::CT && p.is_alive());
        let t  = players.values().any(|p| p.team == Team::T  && p.is_alive());
        if !ct { return Some(RoundResult::TWin(TWinReason::CTsEliminated)); }
        if !t  { return Some(RoundResult::CTWin(CTWinReason::TerrorsEliminated)); }
        None
    }

    fn end_round(&mut self, result: RoundResult, players: &HashMap<String, Player>) -> Option<RoundResult> {
        self.result = result;
        self.phase = RoundPhase::Ended;
        self.mvp_id = self.compute_mvp(result, players);
        Some(result)
    }

    fn compute_mvp(&self, result: RoundResult, players: &HashMap<String, Player>) -> Option<String> {
        let winning_team = match result {
            RoundResult::CTWin(_) => Team::CT,
            RoundResult::TWin(_)  => Team::T,
            RoundResult::InProgress => return None,
        };
        match result {
            RoundResult::CTWin(CTWinReason::BombDefused) => {
                if let Some(id) = &self.bomb.defuser_id { return Some(id.clone()); }
            }
            RoundResult::TWin(TWinReason::BombExploded) => {
                if let Some(id) = &self.bomb.planter_id { return Some(id.clone()); }
            }
            _ => {}
        }
        players
            .values()
            .filter(|p| p.team == winning_team)
            .max_by(|a, b| {
                a.round_kills.cmp(&b.round_kills)
                    .then(a.round_damage.partial_cmp(&b.round_damage)
                        .unwrap_or(std::cmp::Ordering::Equal))
            })
            .map(|p| p.profile.id.clone())
    }

    pub fn record_kill(&mut self, event: KillEvent) {
        self.kill_log.push(event);
    }

    pub fn bomb_frac(&self) -> f32 {
        if !self.bomb.is_planted() || self.bomb.detonation_timer_secs <= 0.0 { return 0.0; }
        (self.bomb.timer_secs / self.bomb.detonation_timer_secs).clamp(0.0, 1.0)
    }

    pub fn defuse_frac(&self, total_defuse_secs: f32) -> f32 {
        use crate::map::bomb::BombPhase;
        if self.bomb.phase != BombPhase::Defusing || total_defuse_secs <= 0.0 { return 0.0; }
        (self.bomb.timer_secs / total_defuse_secs).clamp(0.0, 1.0)
    }
}
