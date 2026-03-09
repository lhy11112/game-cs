use serde::{Deserialize, Serialize};

/// Team designation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Team {
    /// Counter-Terrorists (defenders)
    CT,
    /// Terrorists (attackers)
    T,
    /// Spectator / not assigned
    #[default]
    Spectator,
}

/// Game mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    /// Classic bomb defusal (CS standard)
    BombDefusal,
    /// Team deathmatch
    TeamDeathmatch,
    /// Free-for-all deathmatch
    Deathmatch,
}

/// Round outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundResult {
    /// Counter-Terrorists win
    CTWin(CTWinReason),
    /// Terrorists win
    TWin(TWinReason),
    /// Round is still in progress
    InProgress,
}

/// Reasons CT can win a round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CTWinReason {
    /// All terrorists eliminated
    TerrorsEliminated,
    /// Bomb defused
    BombDefused,
    /// Time expired without bomb plant
    TimeExpired,
}

/// Reasons T can win a round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TWinReason {
    /// All counter-terrorists eliminated
    CTsEliminated,
    /// Bomb exploded
    BombExploded,
}

/// Round phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundPhase {
    /// Freeze time - players cannot move
    FreezeTime,
    /// Live phase
    Live,
    /// Bomb has been planted
    BombPlanted,
    /// Round ended
    Ended,
}

/// Player rank / segment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rank {
    SilverI,
    SilverII,
    SilverIII,
    SilverIV,
    SilverElite,
    SilverEliteMaster,
    GoldNovaI,
    GoldNovaII,
    GoldNovaIII,
    GoldNovaMaster,
    MasterGuardianI,
    MasterGuardianII,
    MasterGuardianElite,
    DistinguishedMasterGuardian,
    LegendaryEagle,
    LegendaryEagleMaster,
    SupremeMasterFirstClass,
    GlobalElite,
}

impl Default for Rank {
    fn default() -> Self {
        Rank::SilverI
    }
}

impl std::fmt::Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Rank::SilverI => "Silver I",
            Rank::SilverII => "Silver II",
            Rank::SilverIII => "Silver III",
            Rank::SilverIV => "Silver IV",
            Rank::SilverElite => "Silver Elite",
            Rank::SilverEliteMaster => "Silver Elite Master",
            Rank::GoldNovaI => "Gold Nova I",
            Rank::GoldNovaII => "Gold Nova II",
            Rank::GoldNovaIII => "Gold Nova III",
            Rank::GoldNovaMaster => "Gold Nova Master",
            Rank::MasterGuardianI => "Master Guardian I",
            Rank::MasterGuardianII => "Master Guardian II",
            Rank::MasterGuardianElite => "Master Guardian Elite",
            Rank::DistinguishedMasterGuardian => "Distinguished Master Guardian",
            Rank::LegendaryEagle => "Legendary Eagle",
            Rank::LegendaryEagleMaster => "Legendary Eagle Master",
            Rank::SupremeMasterFirstClass => "Supreme Master First Class",
            Rank::GlobalElite => "Global Elite",
        };
        write!(f, "{}", name)
    }
}

/// 3D position in the game world
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn distance_to(&self, other: &Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn zero() -> Self {
        Vec3::new(0.0, 0.0, 0.0)
    }
}

/// Player view angles (pitch, yaw)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ViewAngles {
    /// Up/down angle in degrees (-90 to +90)
    pub pitch: f32,
    /// Left/right angle in degrees (0-360)
    pub yaw: f32,
}

impl ViewAngles {
    pub fn new(pitch: f32, yaw: f32) -> Self {
        ViewAngles {
            pitch: pitch.clamp(-90.0, 90.0),
            yaw: yaw % 360.0,
        }
    }
}

/// Hit box zone on a player body
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitZone {
    Head,
    Neck,
    Chest,
    Stomach,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

impl HitZone {
    /// Damage multiplier for this hit zone
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            HitZone::Head => 4.0,
            HitZone::Neck => 2.0,
            HitZone::Chest => 1.0,
            HitZone::Stomach => 1.0,
            HitZone::LeftArm | HitZone::RightArm => 0.7,
            HitZone::LeftLeg | HitZone::RightLeg => 0.7,
        }
    }
}

/// Kill event recorded during a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillEvent {
    pub killer_id: String,
    pub victim_id: String,
    pub weapon_name: String,
    pub hit_zone: HitZone,
    pub is_headshot: bool,
    pub timestamp_ms: u64,
}

/// Match configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    pub game_mode: GameMode,
    pub max_rounds: u32,
    /// Rounds to win (e.g. 16 in standard 30-round match)
    pub rounds_to_win: u32,
    /// Freeze time in seconds
    pub freeze_time_secs: u32,
    /// Round time in seconds (before bomb)
    pub round_time_secs: u32,
    /// Bomb timer in seconds after planting
    pub bomb_timer_secs: u32,
    /// Time to plant bomb in seconds
    pub plant_time_secs: f32,
    /// Time to defuse bomb in seconds (without kit)
    pub defuse_time_secs: f32,
    /// Time to defuse bomb with defuse kit in seconds
    pub defuse_kit_time_secs: f32,
    /// Starting money per player
    pub start_money: u32,
    /// Max money a player can hold
    pub max_money: u32,
}

impl Default for MatchConfig {
    fn default() -> Self {
        MatchConfig {
            game_mode: GameMode::BombDefusal,
            max_rounds: 30,
            rounds_to_win: 16,
            freeze_time_secs: 15,
            round_time_secs: 115,
            bomb_timer_secs: 40,
            plant_time_secs: 3.0,
            defuse_time_secs: 10.0,
            defuse_kit_time_secs: 5.0,
            start_money: 800,
            max_money: 16000,
        }
    }
}

/// Summary of a completed round (for round-history display and MVP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundSummary {
    pub round_number: u32,
    pub result: RoundResult,
    /// Player ID of the round MVP
    pub mvp_id: Option<String>,
    pub mvp_nickname: Option<String>,
    pub ct_score: u32,
    pub t_score: u32,
}

/// Reward money given at the end of a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundMoneyRewards {
    pub ct_win: u32,
    pub t_win: u32,
    pub ct_loss: u32,
    pub t_loss_base: u32,
    /// Extra added per consecutive loss (loss bonus)
    pub loss_bonus_increment: u32,
    /// Maximum consecutive loss bonus rounds
    pub max_loss_bonus: u32,
    pub kill_reward: u32,
}

impl Default for RoundMoneyRewards {
    fn default() -> Self {
        RoundMoneyRewards {
            ct_win: 3250,
            t_win: 3250,
            ct_loss: 1400,
            t_loss_base: 1400,
            loss_bonus_increment: 500,
            max_loss_bonus: 4,
            kill_reward: 300,
        }
    }
}
