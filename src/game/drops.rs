/// Weapon drop / pickup system — CS 1.5 / FreeCS mechanic
///
/// When a player is killed their weapons fall to the ground.  Any living player
/// who walks over the weapon can pick it up.
use crate::game::types::Vec3;
use crate::weapons::types::Weapon;
use serde::{Deserialize, Serialize};

/// A weapon lying on the ground
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroppedWeapon {
    pub id: u32,
    /// Original owner (for display / logging)
    pub original_owner_id: String,
    pub weapon: Weapon,
    pub position: Vec3,
}

/// How close a player must be to pick up a dropped weapon (world units)
pub const PICKUP_RANGE: f32 = 80.0;

/// Manages all weapons currently on the ground for a round
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DropZone {
    drops: Vec<DroppedWeapon>,
    next_id: u32,
}

impl DropZone {
    pub fn new() -> Self {
        DropZone::default()
    }

    /// Add a dropped weapon at the given position
    pub fn add(&mut self, owner_id: String, weapon: Weapon, position: Vec3) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        log::debug!(
            "Weapon '{}' dropped by {} at {:?}",
            weapon.stats.name, owner_id, position
        );
        self.drops.push(DroppedWeapon { id, original_owner_id: owner_id, weapon, position });
        id
    }

    /// Find the closest dropped weapon within pickup range of `player_pos`.
    /// Returns its ID if found.
    pub fn nearest_in_range(&self, player_pos: &Vec3) -> Option<u32> {
        let mut best_dist = PICKUP_RANGE;
        let mut best_id = None;
        for drop in &self.drops {
            let dist = drop.position.distance_to(player_pos);
            if dist <= best_dist {
                best_dist = dist;
                best_id = Some(drop.id);
            }
        }
        best_id
    }

    /// Remove and return the dropped weapon with the given ID (pickup action)
    pub fn take(&mut self, id: u32) -> Option<DroppedWeapon> {
        if let Some(idx) = self.drops.iter().position(|d| d.id == id) {
            Some(self.drops.remove(idx))
        } else {
            None
        }
    }

    /// Clear all drops (start of new round)
    pub fn clear(&mut self) {
        self.drops.clear();
    }

    pub fn all(&self) -> &[DroppedWeapon] {
        &self.drops
    }

    pub fn count(&self) -> usize {
        self.drops.len()
    }
}
