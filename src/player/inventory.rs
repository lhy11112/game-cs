use crate::weapons::types::{Weapon, WeaponSlot};
use serde::{Deserialize, Serialize};

/// A player's weapon inventory (primary, secondary, melee, throwables)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub primary: Option<Weapon>,
    pub secondary: Option<Weapon>,
    pub melee: Option<Weapon>,
    pub throwables: Vec<Weapon>,
    /// Index into throwables of the active throwable
    pub active_throwable: Option<usize>,
    /// Which slot is currently active
    pub active_slot: ActiveSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ActiveSlot {
    #[default]
    Primary,
    Secondary,
    Melee,
    Throwable,
}

impl Inventory {
    /// Starting loadout with only a knife
    pub fn default_loadout() -> Self {
        use crate::weapons::catalog::WeaponCatalog;
        let catalog = WeaponCatalog::new();
        let knife = catalog
            .get("Knife")
            .cloned()
            .map(Weapon::new)
            .expect("Knife must be in catalog");

        Inventory {
            primary: None,
            secondary: None,
            melee: Some(knife),
            throwables: Vec::new(),
            active_throwable: None,
            active_slot: ActiveSlot::Melee,
        }
    }

    /// Add a weapon to the appropriate slot.
    /// Returns the old weapon that was displaced (if any).
    pub fn equip(&mut self, weapon: Weapon) -> Option<Weapon> {
        match weapon.stats.slot {
            WeaponSlot::Primary => {
                let old = self.primary.take();
                self.primary = Some(weapon);
                self.active_slot = ActiveSlot::Primary;
                old
            }
            WeaponSlot::Secondary => {
                let old = self.secondary.take();
                self.secondary = Some(weapon);
                self.active_slot = ActiveSlot::Secondary;
                old
            }
            WeaponSlot::Melee => {
                let old = self.melee.take();
                self.melee = Some(weapon);
                self.active_slot = ActiveSlot::Melee;
                old
            }
            WeaponSlot::Throwable => {
                self.throwables.push(weapon);
                self.active_throwable = Some(self.throwables.len() - 1);
                self.active_slot = ActiveSlot::Throwable;
                None
            }
        }
    }

    /// Drop a weapon from a slot; returns it if present
    pub fn drop_primary(&mut self) -> Option<Weapon> {
        self.primary.take()
    }

    pub fn drop_secondary(&mut self) -> Option<Weapon> {
        self.secondary.take()
    }

    /// Get a reference to the currently active weapon
    pub fn active_weapon(&self) -> Option<&Weapon> {
        match self.active_slot {
            ActiveSlot::Primary => self.primary.as_ref(),
            ActiveSlot::Secondary => self.secondary.as_ref(),
            ActiveSlot::Melee => self.melee.as_ref(),
            ActiveSlot::Throwable => {
                self.active_throwable
                    .and_then(|i| self.throwables.get(i))
            }
        }
    }

    /// Get a mutable reference to the currently active weapon
    pub fn active_weapon_mut(&mut self) -> Option<&mut Weapon> {
        match self.active_slot {
            ActiveSlot::Primary => self.primary.as_mut(),
            ActiveSlot::Secondary => self.secondary.as_mut(),
            ActiveSlot::Melee => self.melee.as_mut(),
            ActiveSlot::Throwable => {
                self.active_throwable
                    .and_then(|i| self.throwables.get_mut(i))
            }
        }
    }

    /// Switch to a different slot
    pub fn switch_to(&mut self, slot: ActiveSlot) -> bool {
        let has_weapon = match slot {
            ActiveSlot::Primary => self.primary.is_some(),
            ActiveSlot::Secondary => self.secondary.is_some(),
            ActiveSlot::Melee => self.melee.is_some(),
            ActiveSlot::Throwable => !self.throwables.is_empty(),
        };
        if has_weapon {
            self.active_slot = slot;
        }
        has_weapon
    }

    /// Switch to next throwable
    pub fn cycle_throwable(&mut self) {
        if self.throwables.is_empty() {
            return;
        }
        let next = match self.active_throwable {
            None => 0,
            Some(i) => (i + 1) % self.throwables.len(),
        };
        self.active_throwable = Some(next);
        self.active_slot = ActiveSlot::Throwable;
    }

    /// Remove and return the currently active throwable after throwing
    pub fn use_throwable(&mut self) -> Option<Weapon> {
        if let Some(idx) = self.active_throwable {
            if idx < self.throwables.len() {
                let thrown = self.throwables.remove(idx);
                self.active_throwable = if self.throwables.is_empty() {
                    None
                } else {
                    Some(idx.min(self.throwables.len() - 1))
                };
                return Some(thrown);
            }
        }
        None
    }

    /// Clear everything (on death / round end)
    pub fn clear(&mut self) {
        self.primary = None;
        self.secondary = None;
        self.throwables.clear();
        self.active_throwable = None;
        // Keep knife
        self.active_slot = ActiveSlot::Melee;
    }
}
