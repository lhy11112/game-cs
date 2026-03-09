use crate::weapons::types::{WeaponKind, WeaponSlot, WeaponStats};
use std::collections::HashMap;

/// Registry of all available weapons with their base stats
pub struct WeaponCatalog {
    weapons: HashMap<String, WeaponStats>,
}

impl WeaponCatalog {
    pub fn new() -> Self {
        let mut catalog = WeaponCatalog {
            weapons: HashMap::new(),
        };
        catalog.register_defaults();
        catalog
    }

    pub fn get(&self, name: &str) -> Option<&WeaponStats> {
        self.weapons.get(name)
    }

    pub fn all(&self) -> impl Iterator<Item = &WeaponStats> {
        self.weapons.values()
    }

    fn add(&mut self, stats: WeaponStats) {
        self.weapons.insert(stats.name.clone(), stats);
    }

    fn register_defaults(&mut self) {
        // ─── Pistols ───────────────────────────────────────────────────────────
        self.add(WeaponStats {
            name: "Glock-18".into(),
            kind: WeaponKind::Pistol,
            slot: WeaponSlot::Secondary,
            price: 200,
            base_damage: 28.0,
            armor_penetration: 0.47,
            fire_rate_rpm: 400.0,
            magazine_size: 20,
            reserve_ammo: 120,
            reload_time_secs: 2.2,
            shot_interval_secs: WeaponStats::from_rpm(400.0),
            recoil: 0.5,
            base_inaccuracy: 0.6,
            move_inaccuracy: 1.0,
            is_automatic: true,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "USP-S".into(),
            kind: WeaponKind::Pistol,
            slot: WeaponSlot::Secondary,
            price: 200,
            base_damage: 35.0,
            armor_penetration: 0.50,
            fire_rate_rpm: 352.0,
            magazine_size: 12,
            reserve_ammo: 24,
            reload_time_secs: 2.2,
            shot_interval_secs: WeaponStats::from_rpm(352.0),
            recoil: 0.4,
            base_inaccuracy: 0.5,
            move_inaccuracy: 0.9,
            is_automatic: false,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "Desert Eagle".into(),
            kind: WeaponKind::Pistol,
            slot: WeaponSlot::Secondary,
            price: 700,
            base_damage: 63.0,
            armor_penetration: 0.93,
            fire_rate_rpm: 267.0,
            magazine_size: 7,
            reserve_ammo: 35,
            reload_time_secs: 2.2,
            shot_interval_secs: WeaponStats::from_rpm(267.0),
            recoil: 1.5,
            base_inaccuracy: 0.9,
            move_inaccuracy: 2.0,
            is_automatic: false,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "P250".into(),
            kind: WeaponKind::Pistol,
            slot: WeaponSlot::Secondary,
            price: 300,
            base_damage: 38.0,
            armor_penetration: 0.94,
            fire_rate_rpm: 400.0,
            magazine_size: 13,
            reserve_ammo: 26,
            reload_time_secs: 2.2,
            shot_interval_secs: WeaponStats::from_rpm(400.0),
            recoil: 0.6,
            base_inaccuracy: 0.6,
            move_inaccuracy: 1.2,
            is_automatic: false,
            kill_reward: 300,
        });

        // ─── SMGs ───────────────────────────────────────────────────────────────
        self.add(WeaponStats {
            name: "MP5-SD".into(),
            kind: WeaponKind::SMG,
            slot: WeaponSlot::Primary,
            price: 1500,
            base_damage: 27.0,
            armor_penetration: 0.60,
            fire_rate_rpm: 857.0,
            magazine_size: 30,
            reserve_ammo: 120,
            reload_time_secs: 2.1,
            shot_interval_secs: WeaponStats::from_rpm(857.0),
            recoil: 0.7,
            base_inaccuracy: 0.5,
            move_inaccuracy: 0.7,
            is_automatic: true,
            kill_reward: 600,
        });

        self.add(WeaponStats {
            name: "UMP-45".into(),
            kind: WeaponKind::SMG,
            slot: WeaponSlot::Primary,
            price: 1200,
            base_damage: 35.0,
            armor_penetration: 0.65,
            fire_rate_rpm: 667.0,
            magazine_size: 25,
            reserve_ammo: 100,
            reload_time_secs: 3.5,
            shot_interval_secs: WeaponStats::from_rpm(667.0),
            recoil: 0.8,
            base_inaccuracy: 0.6,
            move_inaccuracy: 0.8,
            is_automatic: true,
            kill_reward: 600,
        });

        // ─── Rifles ─────────────────────────────────────────────────────────────
        self.add(WeaponStats {
            name: "AK-47".into(),
            kind: WeaponKind::Rifle,
            slot: WeaponSlot::Primary,
            price: 2700,
            base_damage: 36.0,
            armor_penetration: 0.775,
            fire_rate_rpm: 600.0,
            magazine_size: 30,
            reserve_ammo: 90,
            reload_time_secs: 2.4,
            shot_interval_secs: WeaponStats::from_rpm(600.0),
            recoil: 1.2,
            base_inaccuracy: 0.4,
            move_inaccuracy: 1.5,
            is_automatic: true,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "M4A1-S".into(),
            kind: WeaponKind::Rifle,
            slot: WeaponSlot::Primary,
            price: 2900,
            base_damage: 38.0,
            armor_penetration: 0.70,
            fire_rate_rpm: 600.0,
            magazine_size: 20,
            reserve_ammo: 40,
            reload_time_secs: 3.1,
            shot_interval_secs: WeaponStats::from_rpm(600.0),
            recoil: 0.9,
            base_inaccuracy: 0.3,
            move_inaccuracy: 1.2,
            is_automatic: true,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "M4A4".into(),
            kind: WeaponKind::Rifle,
            slot: WeaponSlot::Primary,
            price: 3100,
            base_damage: 33.0,
            armor_penetration: 0.70,
            fire_rate_rpm: 666.0,
            magazine_size: 30,
            reserve_ammo: 90,
            reload_time_secs: 3.1,
            shot_interval_secs: WeaponStats::from_rpm(666.0),
            recoil: 1.0,
            base_inaccuracy: 0.3,
            move_inaccuracy: 1.2,
            is_automatic: true,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "FAMAS".into(),
            kind: WeaponKind::Rifle,
            slot: WeaponSlot::Primary,
            price: 2050,
            base_damage: 30.0,
            armor_penetration: 0.70,
            fire_rate_rpm: 666.0,
            magazine_size: 25,
            reserve_ammo: 75,
            reload_time_secs: 3.3,
            shot_interval_secs: WeaponStats::from_rpm(666.0),
            recoil: 0.9,
            base_inaccuracy: 0.4,
            move_inaccuracy: 1.3,
            is_automatic: true,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "Galil AR".into(),
            kind: WeaponKind::Rifle,
            slot: WeaponSlot::Primary,
            price: 2000,
            base_damage: 30.0,
            armor_penetration: 0.775,
            fire_rate_rpm: 666.0,
            magazine_size: 35,
            reserve_ammo: 90,
            reload_time_secs: 3.3,
            shot_interval_secs: WeaponStats::from_rpm(666.0),
            recoil: 1.0,
            base_inaccuracy: 0.4,
            move_inaccuracy: 1.4,
            is_automatic: true,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "SG 553".into(),
            kind: WeaponKind::Rifle,
            slot: WeaponSlot::Primary,
            price: 3000,
            base_damage: 30.0,
            armor_penetration: 0.90,
            fire_rate_rpm: 666.0,
            magazine_size: 30,
            reserve_ammo: 90,
            reload_time_secs: 2.8,
            shot_interval_secs: WeaponStats::from_rpm(666.0),
            recoil: 1.0,
            base_inaccuracy: 0.4,
            move_inaccuracy: 1.2,
            is_automatic: true,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "AUG".into(),
            kind: WeaponKind::Rifle,
            slot: WeaponSlot::Primary,
            price: 3300,
            base_damage: 28.0,
            armor_penetration: 0.90,
            fire_rate_rpm: 666.0,
            magazine_size: 30,
            reserve_ammo: 90,
            reload_time_secs: 3.8,
            shot_interval_secs: WeaponStats::from_rpm(666.0),
            recoil: 0.8,
            base_inaccuracy: 0.3,
            move_inaccuracy: 1.1,
            is_automatic: true,
            kill_reward: 300,
        });

        // ─── Snipers ────────────────────────────────────────────────────────────
        self.add(WeaponStats {
            name: "AWP".into(),
            kind: WeaponKind::Sniper,
            slot: WeaponSlot::Primary,
            price: 4750,
            base_damage: 115.0,
            armor_penetration: 0.975,
            fire_rate_rpm: 41.0,
            magazine_size: 10,
            reserve_ammo: 30,
            reload_time_secs: 3.7,
            shot_interval_secs: WeaponStats::from_rpm(41.0),
            recoil: 3.0,
            base_inaccuracy: 0.0,
            move_inaccuracy: 5.0,
            is_automatic: false,
            kill_reward: 100,
        });

        self.add(WeaponStats {
            name: "SSG 08".into(),
            kind: WeaponKind::Sniper,
            slot: WeaponSlot::Primary,
            price: 1700,
            base_damage: 88.0,
            armor_penetration: 0.975,
            fire_rate_rpm: 48.0,
            magazine_size: 10,
            reserve_ammo: 90,
            reload_time_secs: 3.7,
            shot_interval_secs: WeaponStats::from_rpm(48.0),
            recoil: 2.5,
            base_inaccuracy: 0.0,
            move_inaccuracy: 5.0,
            is_automatic: false,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "SCAR-20".into(),
            kind: WeaponKind::Sniper,
            slot: WeaponSlot::Primary,
            price: 5000,
            base_damage: 80.0,
            armor_penetration: 0.975,
            fire_rate_rpm: 240.0,
            magazine_size: 20,
            reserve_ammo: 90,
            reload_time_secs: 3.7,
            shot_interval_secs: WeaponStats::from_rpm(240.0),
            recoil: 2.0,
            base_inaccuracy: 0.1,
            move_inaccuracy: 5.0,
            is_automatic: true,
            kill_reward: 300,
        });

        // ─── Shotguns ───────────────────────────────────────────────────────────
        self.add(WeaponStats {
            name: "Nova".into(),
            kind: WeaponKind::Shotgun,
            slot: WeaponSlot::Primary,
            price: 1050,
            base_damage: 26.0,
            armor_penetration: 0.50,
            fire_rate_rpm: 68.0,
            magazine_size: 8,
            reserve_ammo: 32,
            reload_time_secs: 0.5,
            shot_interval_secs: WeaponStats::from_rpm(68.0),
            recoil: 2.0,
            base_inaccuracy: 1.0,
            move_inaccuracy: 2.0,
            is_automatic: false,
            kill_reward: 900,
        });

        self.add(WeaponStats {
            name: "XM1014".into(),
            kind: WeaponKind::Shotgun,
            slot: WeaponSlot::Primary,
            price: 2000,
            base_damage: 20.0,
            armor_penetration: 0.50,
            fire_rate_rpm: 171.0,
            magazine_size: 7,
            reserve_ammo: 32,
            reload_time_secs: 0.5,
            shot_interval_secs: WeaponStats::from_rpm(171.0),
            recoil: 1.8,
            base_inaccuracy: 1.0,
            move_inaccuracy: 1.8,
            is_automatic: true,
            kill_reward: 900,
        });

        // ─── Machine Guns ───────────────────────────────────────────────────────
        self.add(WeaponStats {
            name: "M249".into(),
            kind: WeaponKind::MachineGun,
            slot: WeaponSlot::Primary,
            price: 5200,
            base_damage: 32.0,
            armor_penetration: 0.80,
            fire_rate_rpm: 857.0,
            magazine_size: 100,
            reserve_ammo: 200,
            reload_time_secs: 4.7,
            shot_interval_secs: WeaponStats::from_rpm(857.0),
            recoil: 1.5,
            base_inaccuracy: 0.5,
            move_inaccuracy: 2.0,
            is_automatic: true,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "Negev".into(),
            kind: WeaponKind::MachineGun,
            slot: WeaponSlot::Primary,
            price: 1700,
            base_damage: 35.0,
            armor_penetration: 0.80,
            fire_rate_rpm: 1000.0,
            magazine_size: 150,
            reserve_ammo: 300,
            reload_time_secs: 5.7,
            shot_interval_secs: WeaponStats::from_rpm(1000.0),
            recoil: 2.0,
            base_inaccuracy: 0.7,
            move_inaccuracy: 2.5,
            is_automatic: true,
            kill_reward: 300,
        });

        // ─── Grenades ───────────────────────────────────────────────────────────
        self.add(WeaponStats {
            name: "HE Grenade".into(),
            kind: WeaponKind::Throwable,
            slot: WeaponSlot::Throwable,
            price: 300,
            base_damage: 98.0,
            armor_penetration: 0.50,
            fire_rate_rpm: 60.0,
            magazine_size: 1,
            reserve_ammo: 0,
            reload_time_secs: 0.0,
            shot_interval_secs: 1.0,
            recoil: 0.0,
            base_inaccuracy: 0.0,
            move_inaccuracy: 0.0,
            is_automatic: false,
            kill_reward: 300,
        });

        self.add(WeaponStats {
            name: "Flashbang".into(),
            kind: WeaponKind::Throwable,
            slot: WeaponSlot::Throwable,
            price: 200,
            base_damage: 0.0,
            armor_penetration: 0.0,
            fire_rate_rpm: 60.0,
            magazine_size: 2,
            reserve_ammo: 0,
            reload_time_secs: 0.0,
            shot_interval_secs: 1.0,
            recoil: 0.0,
            base_inaccuracy: 0.0,
            move_inaccuracy: 0.0,
            is_automatic: false,
            kill_reward: 0,
        });

        self.add(WeaponStats {
            name: "Smoke Grenade".into(),
            kind: WeaponKind::Throwable,
            slot: WeaponSlot::Throwable,
            price: 300,
            base_damage: 0.0,
            armor_penetration: 0.0,
            fire_rate_rpm: 60.0,
            magazine_size: 1,
            reserve_ammo: 0,
            reload_time_secs: 0.0,
            shot_interval_secs: 1.0,
            recoil: 0.0,
            base_inaccuracy: 0.0,
            move_inaccuracy: 0.0,
            is_automatic: false,
            kill_reward: 0,
        });

        self.add(WeaponStats {
            name: "Molotov".into(),
            kind: WeaponKind::Throwable,
            slot: WeaponSlot::Throwable,
            price: 400,
            base_damage: 40.0,
            armor_penetration: 0.0,
            fire_rate_rpm: 60.0,
            magazine_size: 1,
            reserve_ammo: 0,
            reload_time_secs: 0.0,
            shot_interval_secs: 1.0,
            recoil: 0.0,
            base_inaccuracy: 0.0,
            move_inaccuracy: 0.0,
            is_automatic: false,
            kill_reward: 0,
        });

        self.add(WeaponStats {
            name: "Incendiary Grenade".into(),
            kind: WeaponKind::Throwable,
            slot: WeaponSlot::Throwable,
            price: 600,
            base_damage: 40.0,
            armor_penetration: 0.0,
            fire_rate_rpm: 60.0,
            magazine_size: 1,
            reserve_ammo: 0,
            reload_time_secs: 0.0,
            shot_interval_secs: 1.0,
            recoil: 0.0,
            base_inaccuracy: 0.0,
            move_inaccuracy: 0.0,
            is_automatic: false,
            kill_reward: 0,
        });

        // ─── Knife ──────────────────────────────────────────────────────────────
        self.add(WeaponStats {
            name: "Knife".into(),
            kind: WeaponKind::Knife,
            slot: WeaponSlot::Melee,
            price: 0,
            base_damage: 65.0,
            armor_penetration: 1.0,
            fire_rate_rpm: 120.0,
            magazine_size: 1,
            reserve_ammo: 0,
            reload_time_secs: 0.0,
            shot_interval_secs: WeaponStats::from_rpm(120.0),
            recoil: 0.0,
            base_inaccuracy: 0.0,
            move_inaccuracy: 0.0,
            is_automatic: false,
            kill_reward: 1500,
        });
    }
}

impl Default for WeaponCatalog {
    fn default() -> Self {
        Self::new()
    }
}
