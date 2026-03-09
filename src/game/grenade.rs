/// Grenade system inspired by Counter-Strike 1.5 / FreeCS
///
/// Grenades are thrown projectiles with a fuse timer.  When the fuse expires
/// the grenade applies an area-of-effect: damage, blindness, smoke, or fire.
use crate::game::types::Vec3;
use crate::player::types::Player;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Grenade types ─────────────────────────────────────────────────────────────

/// CS 1.5 grenade categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrenadeType {
    /// High-Explosive: damages all players in radius
    HE,
    /// Flashbang: blinds players facing the detonation
    Flash,
    /// Smoke: creates an 18-second obscuring cloud
    Smoke,
    /// Molotov / Incendiary: burns area for 8 seconds
    Molotov,
}

impl GrenadeType {
    /// Fuse time in seconds (time from throw to detonation)
    pub fn fuse_secs(self) -> f32 {
        match self {
            GrenadeType::HE | GrenadeType::Molotov => 3.0,
            GrenadeType::Flash => 1.5,
            GrenadeType::Smoke => 2.0,
        }
    }

    /// Explosion / effect radius in world units
    pub fn radius(self) -> f32 {
        match self {
            GrenadeType::HE => 350.0,
            GrenadeType::Flash => 700.0,
            GrenadeType::Smoke => 200.0,
            GrenadeType::Molotov => 150.0,
        }
    }

    /// Maximum damage at point-blank (HE only)
    pub fn max_damage(self) -> f32 {
        match self {
            GrenadeType::HE => 98.0,
            GrenadeType::Molotov => 40.0, // per-tick burn
            _ => 0.0,
        }
    }

    /// Duration of the persistent effect in seconds (Smoke / Molotov)
    pub fn effect_duration(self) -> f32 {
        match self {
            GrenadeType::Smoke => 18.0,
            GrenadeType::Molotov => 8.0,
            GrenadeType::Flash => 3.0, // max blind duration
            GrenadeType::HE => 0.0,
        }
    }
}

// ── Thrown grenade ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrenadeState {
    Fusing,     // In flight / counting down
    Active,     // Persistent effect ongoing (smoke / fire)
    Expired,    // Done — safe to remove
}

/// A grenade that has been thrown and is live in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrownGrenade {
    pub id: u32,
    pub kind: GrenadeType,
    pub thrower_id: String,
    pub position: Vec3,
    pub state: GrenadeState,
    /// Fuse / effect timer remaining (seconds)
    pub timer_secs: f32,
    /// Whether the initial detonation has already been applied
    pub detonated: bool,
}

impl ThrownGrenade {
    pub fn new(id: u32, kind: GrenadeType, thrower_id: String, position: Vec3) -> Self {
        ThrownGrenade {
            id,
            kind,
            thrower_id,
            position,
            state: GrenadeState::Fusing,
            timer_secs: kind.fuse_secs(),
            detonated: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.state == GrenadeState::Expired
    }
}

// ── Grenade effect event ──────────────────────────────────────────────────────

/// The result of a grenade detonating or ticking its persistent effect
#[derive(Debug, Clone)]
pub struct GrenadeEffectEvent {
    pub grenade_id: u32,
    pub kind: GrenadeType,
    pub position: Vec3,
    /// Damage to apply to each affected player (player_id → damage)
    pub damage_map: HashMap<String, f32>,
    /// Blind duration for each affected player (player_id → seconds)
    pub blind_map: HashMap<String, f32>,
    /// Whether the grenade created / is sustaining a smoke cloud
    pub is_smoke: bool,
}

// ── Grenade simulator ─────────────────────────────────────────────────────────

/// Manages all live grenades for a single round
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrenadeSim {
    grenades: Vec<ThrownGrenade>,
    next_id: u32,
}

impl GrenadeSim {
    pub fn new() -> Self {
        GrenadeSim::default()
    }

    /// Register a newly thrown grenade
    pub fn throw(&mut self, kind: GrenadeType, thrower_id: String, position: Vec3) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.grenades.push(ThrownGrenade::new(id, kind, thrower_id, position));
        log::debug!("Grenade {:?} thrown by {} at {:?}", kind, id, position);
        id
    }

    /// Advance all grenade timers and return any effect events that fired
    pub fn tick(&mut self, delta_secs: f32, players: &HashMap<String, Player>) -> Vec<GrenadeEffectEvent> {
        let mut events = Vec::new();

        for nade in &mut self.grenades {
            if nade.state == GrenadeState::Expired {
                continue;
            }

            nade.timer_secs -= delta_secs;

            match nade.state {
                GrenadeState::Fusing => {
                    if nade.timer_secs <= 0.0 {
                        // Detonate
                        let event = detonate(nade, players);
                        nade.detonated = true;

                        match nade.kind {
                            GrenadeType::Smoke | GrenadeType::Molotov => {
                                // Persistent effect
                                nade.state = GrenadeState::Active;
                                nade.timer_secs = nade.kind.effect_duration();
                            }
                            _ => {
                                nade.state = GrenadeState::Expired;
                            }
                        }
                        events.push(event);
                    }
                }
                GrenadeState::Active => {
                    // Molotov tick-burns every 0.5s
                    if nade.kind == GrenadeType::Molotov {
                        // Burn tick approximate: emit burn event every ~0.5s
                        // We simplify: always emit, caller checks delta
                        let event = burn_tick(nade, players, delta_secs);
                        events.push(event);
                    }
                    if nade.timer_secs <= 0.0 {
                        nade.state = GrenadeState::Expired;
                    }
                }
                GrenadeState::Expired => {}
            }
        }

        // Remove expired grenades
        self.grenades.retain(|g| g.state != GrenadeState::Expired);

        events
    }

    pub fn active_smokes(&self) -> Vec<Vec3> {
        self.grenades
            .iter()
            .filter(|g| g.kind == GrenadeType::Smoke && g.state == GrenadeState::Active)
            .filter_map(|g| Some(g.position))
            .collect()
    }

    pub fn active_fires(&self) -> Vec<Vec3> {
        self.grenades
            .iter()
            .filter(|g| g.kind == GrenadeType::Molotov && g.state == GrenadeState::Active)
            .filter_map(|g| Some(g.position))
            .collect()
    }

    pub fn live_count(&self) -> usize {
        self.grenades.len()
    }
}

// ── Effect helpers ─────────────────────────────────────────────────────────────

fn detonate(nade: &ThrownGrenade, players: &HashMap<String, Player>) -> GrenadeEffectEvent {
    let radius = nade.kind.radius();
    let max_dmg = nade.kind.max_damage();
    let mut damage_map = HashMap::new();
    let mut blind_map = HashMap::new();

    for (pid, player) in players {
        if !player.is_alive() {
            continue;
        }
        let dist = player.position.distance_to(&nade.position);
        if dist > radius {
            continue;
        }

        match nade.kind {
            GrenadeType::HE => {
                // Damage falls off linearly with distance
                let falloff = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let dmg = max_dmg * falloff;
                if dmg > 1.0 {
                    damage_map.insert(pid.clone(), dmg);
                }
            }
            GrenadeType::Flash => {
                // Blind duration falls off with distance; faces away = less blind
                // Simplified: all in range get blinded proportional to proximity
                let falloff = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let blind = nade.kind.effect_duration() * falloff;
                if blind > 0.2 {
                    blind_map.insert(pid.clone(), blind);
                }
            }
            GrenadeType::Molotov => {
                // Initial burst damage
                if dist < nade.kind.radius() * 0.5 {
                    damage_map.insert(pid.clone(), 15.0);
                }
            }
            GrenadeType::Smoke => {} // no damage
        }
    }

    GrenadeEffectEvent {
        grenade_id: nade.id,
        kind: nade.kind,
        position: nade.position,
        damage_map,
        blind_map,
        is_smoke: nade.kind == GrenadeType::Smoke,
    }
}

fn burn_tick(nade: &ThrownGrenade, players: &HashMap<String, Player>, delta_secs: f32) -> GrenadeEffectEvent {
    let radius = nade.kind.radius();
    let dps = nade.kind.max_damage(); // damage-per-second
    let mut damage_map = HashMap::new();

    for (pid, player) in players {
        if !player.is_alive() {
            continue;
        }
        let dist = player.position.distance_to(&nade.position);
        if dist <= radius {
            damage_map.insert(pid.clone(), dps * delta_secs);
        }
    }

    GrenadeEffectEvent {
        grenade_id: nade.id,
        kind: nade.kind,
        position: nade.position,
        damage_map,
        blind_map: HashMap::new(),
        is_smoke: false,
    }
}
