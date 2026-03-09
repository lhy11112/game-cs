use crate::game::types::HitZone;
use crate::weapons::types::{AmmoState, Weapon, WeaponStats};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Result of a fire attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FireResult {
    /// Shot was fired, hit detection pending
    Fired { spread_x: f32, spread_y: f32 },
    /// Gun is empty - needs reload
    EmptyMagazine,
    /// Gun is reloading
    IsReloading,
    /// Cooldown not yet elapsed
    OnCooldown { remaining_secs: f32 },
    /// Weapon doesn't use ammo (e.g. knife)
    MeleeStrike,
}

/// Outcome of a single shot against a target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotResult {
    pub hit: bool,
    pub hit_zone: Option<HitZone>,
    pub raw_damage: f32,
    /// Damage after armor reduction
    pub final_damage: f32,
    pub is_headshot: bool,
    pub penetrated_armor: bool,
}

impl ShotResult {
    pub fn missed() -> Self {
        ShotResult {
            hit: false,
            hit_zone: None,
            raw_damage: 0.0,
            final_damage: 0.0,
            is_headshot: false,
            penetrated_armor: false,
        }
    }
}

/// Attempt to fire a weapon. Returns the fire result and updates ammo state.
pub fn attempt_fire(weapon: &mut Weapon, is_moving: bool) -> FireResult {
    let stats = &weapon.stats;

    // Knife / throwable - special handling
    if stats.magazine_size == 1 && stats.reserve_ammo == 0 && stats.reload_time_secs == 0.0 {
        return FireResult::MeleeStrike;
    }

    let ammo = &mut weapon.ammo;

    if ammo.is_reloading() {
        return FireResult::IsReloading;
    }
    if ammo.cooldown_remaining_secs > 0.0 {
        return FireResult::OnCooldown {
            remaining_secs: ammo.cooldown_remaining_secs,
        };
    }
    if ammo.is_empty() {
        return FireResult::EmptyMagazine;
    }

    // Consume bullet and set cooldown
    ammo.consume_bullet();
    ammo.cooldown_remaining_secs = stats.shot_interval_secs;

    // Calculate spread based on movement
    let spread = calculate_spread(stats, is_moving);

    FireResult::Fired {
        spread_x: spread.0,
        spread_y: spread.1,
    }
}

/// Calculate bullet spread (inaccuracy cone) in degrees
fn calculate_spread(stats: &WeaponStats, is_moving: bool) -> (f32, f32) {
    let mut rng = rand::thread_rng();
    let inaccuracy = if is_moving {
        stats.base_inaccuracy + stats.move_inaccuracy
    } else {
        stats.base_inaccuracy
    };

    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let radius = rng.gen_range(0.0..inaccuracy);
    (radius * angle.cos(), radius * angle.sin())
}

/// Compute hit result when a bullet reaches a target
///
/// - `base_damage`: weapon's base damage value
/// - `armor_penetration`: weapon armor penetration ratio (0-1)
/// - `hit_zone`: where the bullet hit
/// - `target_has_armor`: whether the target is wearing body armor
/// - `target_has_helmet`: whether the target has a helmet
pub fn compute_hit_damage(
    base_damage: f32,
    armor_penetration: f32,
    hit_zone: HitZone,
    target_has_armor: bool,
    target_has_helmet: bool,
) -> ShotResult {
    let zone_multiplier = hit_zone.damage_multiplier();
    let raw_damage = base_damage * zone_multiplier;

    let is_headshot = matches!(hit_zone, HitZone::Head);
    let protected = match hit_zone {
        HitZone::Head => target_has_helmet,
        HitZone::Neck | HitZone::Chest | HitZone::Stomach => target_has_armor,
        _ => false,
    };

    let final_damage = if protected {
        // Armor reduces damage proportional to penetration
        // Formula: damage * (armor_pen * 0.5 + 0.5)
        // High penetration weapons deal close to full damage through armor
        raw_damage * (armor_penetration * 0.5 + 0.5)
    } else {
        raw_damage
    };

    ShotResult {
        hit: true,
        hit_zone: Some(hit_zone),
        raw_damage,
        final_damage,
        is_headshot,
        penetrated_armor: protected,
    }
}

/// Attempt to start reloading a weapon
pub fn attempt_reload(ammo: &mut AmmoState, stats: &WeaponStats) -> bool {
    ammo.start_reload(stats)
}
