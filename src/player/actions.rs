use crate::game::error::GameError;
use crate::game::types::HitZone;
use crate::player::types::Player;
use crate::weapons::fire::{attempt_fire, attempt_reload, compute_hit_damage, FireResult, ShotResult};

/// Attempt to fire the player's currently active weapon.
/// Returns `FireResult` describing what happened.
pub fn player_fire(player: &mut Player) -> Result<FireResult, GameError> {
    if !player.is_alive() {
        return Err(GameError::PlayerDead);
    }
    let is_moving = player.movement.makes_noise();
    let weapon = player
        .inventory
        .active_weapon_mut()
        .ok_or_else(|| GameError::InvalidAction("No weapon equipped".into()))?;

    Ok(attempt_fire(weapon, is_moving))
}

/// Attempt to reload the player's currently active weapon.
pub fn player_reload(player: &mut Player) -> Result<bool, GameError> {
    if !player.is_alive() {
        return Err(GameError::PlayerDead);
    }
    let weapon = player
        .inventory
        .active_weapon_mut()
        .ok_or_else(|| GameError::InvalidAction("No weapon equipped".into()))?;
    let stats = weapon.stats.clone();
    Ok(attempt_reload(&mut weapon.ammo, &stats))
}

/// Apply incoming damage to `player` from a shot.
///
/// Returns `ShotResult` with final damage dealt.
pub fn apply_shot_to_player(
    victim: &mut Player,
    base_damage: f32,
    armor_penetration: f32,
    hit_zone: HitZone,
) -> ShotResult {
    let result = compute_hit_damage(
        base_damage,
        armor_penetration,
        hit_zone,
        victim.armor.has_vest(),
        victim.armor.has_helmet,
    );
    victim.take_damage(result.final_damage);
    victim.round_damage += result.final_damage;
    result
}

/// Advance all weapon timers (reload, cooldown) for a player over `delta_secs`.
pub fn tick_player_weapons(player: &mut Player, delta_secs: f32) {
    macro_rules! tick_slot {
        ($slot:expr) => {
            if let Some(weapon) = $slot.as_mut() {
                let stats = weapon.stats.clone();
                weapon.ammo.tick(delta_secs, &stats);
            }
        };
    }
    tick_slot!(player.inventory.primary);
    tick_slot!(player.inventory.secondary);
    for w in &mut player.inventory.throwables {
        let stats = w.stats.clone();
        w.ammo.tick(delta_secs, &stats);
    }
}
