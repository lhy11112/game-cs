use serde::{Deserialize, Serialize};

/// Category of weapon
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponKind {
    /// Rifles (AK-47, M4A1, etc.)
    Rifle,
    /// Submachine guns (MP5, UMP-45, etc.)
    SMG,
    /// Sniper rifles (AWP, SSG 08)
    Sniper,
    /// Shotguns
    Shotgun,
    /// Machine guns (Negev, M249)
    MachineGun,
    /// Pistols (Glock, Desert Eagle, USP-S, etc.)
    Pistol,
    /// Melee / knife
    Knife,
    /// Grenades and throwables
    Throwable,
}

/// Which inventory slot a weapon occupies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponSlot {
    Primary,
    Secondary,
    Melee,
    Throwable,
}

/// Static weapon parameters (loaded from catalog)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponStats {
    pub name: String,
    pub kind: WeaponKind,
    pub slot: WeaponSlot,
    /// Price in the buy menu
    pub price: u32,
    /// Damage per bullet at close range (0-100 HP scale)
    pub base_damage: f32,
    /// Armor penetration ratio (0.0-1.0)
    pub armor_penetration: f32,
    /// Rounds per minute
    pub fire_rate_rpm: f32,
    /// Magazine capacity
    pub magazine_size: u32,
    /// Total reserve ammo (not counting magazine)
    pub reserve_ammo: u32,
    /// Reload duration in seconds
    pub reload_time_secs: f32,
    /// Time between shots in seconds (derived from fire_rate)
    pub shot_interval_secs: f32,
    /// Recoil magnitude (higher = more kick)
    pub recoil: f32,
    /// Base inaccuracy in degrees (standing still, not shooting)
    pub base_inaccuracy: f32,
    /// Additional inaccuracy while moving
    pub move_inaccuracy: f32,
    /// Whether weapon is fully automatic
    pub is_automatic: bool,
    /// Reward on kill with this weapon
    pub kill_reward: u32,
}

impl WeaponStats {
    /// Compute shot interval from fire rate
    pub fn from_rpm(rpm: f32) -> f32 {
        60.0 / rpm
    }
}

/// Live ammunition state for a weapon instance carried by a player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoState {
    pub bullets_in_magazine: u32,
    pub reserve_bullets: u32,
    /// Time remaining in current reload (None = not reloading)
    pub reload_remaining_secs: Option<f32>,
    /// Time remaining until next shot is allowed
    pub cooldown_remaining_secs: f32,
}

impl AmmoState {
    pub fn new(stats: &WeaponStats) -> Self {
        AmmoState {
            bullets_in_magazine: stats.magazine_size,
            reserve_bullets: stats.reserve_ammo,
            reload_remaining_secs: None,
            cooldown_remaining_secs: 0.0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bullets_in_magazine == 0
    }

    pub fn is_reloading(&self) -> bool {
        self.reload_remaining_secs.is_some()
    }

    pub fn can_fire(&self) -> bool {
        !self.is_empty() && !self.is_reloading() && self.cooldown_remaining_secs <= 0.0
    }

    pub fn needs_reload(&self) -> bool {
        self.bullets_in_magazine == 0 && self.reserve_bullets > 0
    }

    /// Consume one bullet; panics if empty
    pub fn consume_bullet(&mut self) {
        assert!(self.bullets_in_magazine > 0, "attempted to fire empty magazine");
        self.bullets_in_magazine -= 1;
    }

    /// Start reloading; returns false if already reloading or full
    pub fn start_reload(&mut self, stats: &WeaponStats) -> bool {
        if self.is_reloading() {
            return false;
        }
        if self.reserve_bullets == 0 {
            return false;
        }
        if self.bullets_in_magazine == stats.magazine_size {
            return false;
        }
        self.reload_remaining_secs = Some(stats.reload_time_secs);
        true
    }

    /// Advance time. Returns true if a reload just completed.
    pub fn tick(&mut self, delta_secs: f32, stats: &WeaponStats) -> bool {
        if self.cooldown_remaining_secs > 0.0 {
            self.cooldown_remaining_secs = (self.cooldown_remaining_secs - delta_secs).max(0.0);
        }

        if let Some(remaining) = self.reload_remaining_secs {
            let new_remaining = remaining - delta_secs;
            if new_remaining <= 0.0 {
                // Complete reload
                let needed = stats.magazine_size - self.bullets_in_magazine;
                let transferred = needed.min(self.reserve_bullets);
                self.bullets_in_magazine += transferred;
                self.reserve_bullets -= transferred;
                self.reload_remaining_secs = None;
                return true;
            } else {
                self.reload_remaining_secs = Some(new_remaining);
            }
        }
        false
    }
}

/// A weapon instance held by a player (stats + live ammo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub stats: WeaponStats,
    pub ammo: AmmoState,
}

impl Weapon {
    pub fn new(stats: WeaponStats) -> Self {
        let ammo = AmmoState::new(&stats);
        Weapon { stats, ammo }
    }
}
