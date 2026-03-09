pub mod types;
pub mod catalog;
pub mod fire;

pub use types::{Weapon, WeaponKind, WeaponSlot, WeaponStats, AmmoState};
pub use catalog::WeaponCatalog;
pub use fire::{FireResult, ShotResult};
