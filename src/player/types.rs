use crate::game::types::{Rank, Team, Vec3, ViewAngles};
use crate::player::inventory::Inventory;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Whether the player is alive or dead
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerState {
    Alive,
    Dead,
    Spectating,
}

/// Current movement mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementState {
    Standing,
    Walking,
    Running,
    Crouching,
    /// Silent walk (slow, no footstep sound)
    Sneaking,
    InAir,
}

impl MovementState {
    /// Speed multiplier relative to base run speed
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            MovementState::Running => 1.0,
            MovementState::Walking => 0.6,
            MovementState::Standing => 0.0,
            MovementState::Crouching => 0.45,
            MovementState::Sneaking => 0.3,
            MovementState::InAir => 0.85,
        }
    }

    /// Whether this state produces audible footsteps
    pub fn makes_noise(&self) -> bool {
        match self {
            MovementState::Sneaking | MovementState::Standing | MovementState::InAir => false,
            _ => true,
        }
    }

    /// Inaccuracy added to shooting due to movement
    pub fn inaccuracy_bonus(&self) -> f32 {
        match self {
            MovementState::Standing => 0.0,
            MovementState::Crouching => 0.1,
            MovementState::Sneaking => 0.15,
            MovementState::Walking => 0.3,
            MovementState::Running => 1.0,
            MovementState::InAir => 1.5,
        }
    }
}

/// Player armor state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArmorState {
    /// Kevlar vest (0-100)
    pub vest_hp: u32,
    pub has_helmet: bool,
    pub has_defuse_kit: bool,
}

impl ArmorState {
    pub fn has_vest(&self) -> bool {
        self.vest_hp > 0
    }

    /// Reduce armor from a hit, returns actual armor absorbed
    pub fn absorb_damage(&mut self, damage: f32) -> f32 {
        if self.vest_hp == 0 {
            return 0.0;
        }
        let absorbed = damage * 0.5; // armor absorbs 50% of protected hits
        let armor_damage = (absorbed as u32).min(self.vest_hp);
        self.vest_hp -= armor_damage;
        absorbed
    }
}

/// Persistent profile data for a player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: String,
    pub nickname: String,
    pub rank: Rank,
    pub experience: u32,
    pub total_kills: u64,
    pub total_deaths: u64,
    pub total_wins: u64,
    pub total_matches: u64,
    pub total_headshots: u64,
    pub created_at: DateTime<Utc>,
}

impl PlayerProfile {
    pub fn new(id: String, nickname: String) -> Self {
        PlayerProfile {
            id,
            nickname,
            rank: Rank::default(),
            experience: 0,
            total_kills: 0,
            total_deaths: 0,
            total_wins: 0,
            total_matches: 0,
            total_headshots: 0,
            created_at: Utc::now(),
        }
    }

    pub fn kd_ratio(&self) -> f64 {
        if self.total_deaths == 0 {
            return self.total_kills as f64;
        }
        self.total_kills as f64 / self.total_deaths as f64
    }

    pub fn win_rate(&self) -> f64 {
        if self.total_matches == 0 {
            return 0.0;
        }
        self.total_wins as f64 / self.total_matches as f64 * 100.0
    }

    pub fn headshot_rate(&self) -> f64 {
        if self.total_kills == 0 {
            return 0.0;
        }
        self.total_headshots as f64 / self.total_kills as f64 * 100.0
    }
}

/// Full in-game player, combining live state and profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub profile: PlayerProfile,
    pub team: Team,
    pub state: PlayerState,

    // Live combat state
    pub health: u32,
    pub armor: ArmorState,
    pub position: Vec3,
    pub view_angles: ViewAngles,
    pub movement: MovementState,
    pub inventory: Inventory,

    // Round-specific
    /// Money available for buying equipment
    pub money: u32,
    pub is_carrying_bomb: bool,
    /// Number of consecutive round losses (for loss bonus calculation)
    pub consecutive_losses: u32,

    // Per-round stats (reset each round)
    pub round_kills: u32,
    pub round_deaths: u32,
    pub round_assists: u32,
    pub round_damage: f32,
}

impl Player {
    pub const BASE_HEALTH: u32 = 100;
    /// Base run speed in units/second
    pub const BASE_SPEED: f32 = 250.0;

    pub fn new(profile: PlayerProfile, team: Team, start_money: u32) -> Self {
        Player {
            profile,
            team,
            state: PlayerState::Alive,
            health: Self::BASE_HEALTH,
            armor: ArmorState::default(),
            position: Vec3::zero(),
            view_angles: ViewAngles::default(),
            movement: MovementState::Standing,
            inventory: Inventory::default_loadout(),
            money: start_money,
            is_carrying_bomb: false,
            consecutive_losses: 0,
            round_kills: 0,
            round_deaths: 0,
            round_assists: 0,
            round_damage: 0.0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.state == PlayerState::Alive
    }

    pub fn current_speed(&self) -> f32 {
        Self::BASE_SPEED * self.movement.speed_multiplier()
    }

    /// Apply damage to player; returns whether player died
    pub fn take_damage(&mut self, damage: f32) -> bool {
        if !self.is_alive() {
            return false;
        }
        let dmg = damage.ceil() as u32;
        if dmg >= self.health {
            self.health = 0;
            self.state = PlayerState::Dead;
            self.round_deaths += 1;
            true
        } else {
            self.health -= dmg;
            self.round_damage += damage;
            false
        }
    }

    /// Respawn / reset for a new round
    pub fn reset_for_round(&mut self, spawn_position: Vec3) {
        self.health = Self::BASE_HEALTH;
        self.state = PlayerState::Alive;
        self.position = spawn_position;
        self.movement = MovementState::Standing;
        self.is_carrying_bomb = false;
        self.round_kills = 0;
        self.round_deaths = 0;
        self.round_assists = 0;
        self.round_damage = 0.0;
    }

    pub fn add_money(&mut self, amount: u32, max: u32) {
        self.money = (self.money + amount).min(max);
    }

    pub fn spend_money(&mut self, amount: u32) -> bool {
        if self.money >= amount {
            self.money -= amount;
            true
        } else {
            false
        }
    }
}
