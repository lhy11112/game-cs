use crate::game::types::{Team, Vec3};
use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        AABB { min, max }
    }

    pub fn contains(&self, point: &Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    pub fn center(&self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
            (self.min.z + self.max.z) / 2.0,
        )
    }
}

/// A bomb planting site (A or B)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BombSite {
    pub label: char,
    pub bounds: AABB,
}

impl BombSite {
    pub fn is_inside(&self, pos: &Vec3) -> bool {
        self.bounds.contains(pos)
    }
}

/// Spawn zone for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnZone {
    pub team: Team,
    pub spawns: Vec<Vec3>,
}

/// Map definition: all static data for one game map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapDef {
    pub name: String,
    pub display_name: String,
    pub ct_spawns: SpawnZone,
    pub t_spawns: SpawnZone,
    pub bomb_sites: Vec<BombSite>,
    /// A brief description / layout note
    pub description: String,
}

impl MapDef {
    /// Return the bomb site that contains `position`, if any
    pub fn bomb_site_at(&self, position: &Vec3) -> Option<&BombSite> {
        self.bomb_sites.iter().find(|s| s.is_inside(position))
    }

    /// Get a spawn point for the given team (round-robin via round number)
    pub fn spawn_for(&self, team: Team, round: u32) -> Vec3 {
        let zone = match team {
            Team::CT => &self.ct_spawns,
            Team::T => &self.t_spawns,
            Team::Spectator => return Vec3::zero(),
        };
        if zone.spawns.is_empty() {
            return Vec3::zero();
        }
        zone.spawns[(round as usize) % zone.spawns.len()]
    }
}
