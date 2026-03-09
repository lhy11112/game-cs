use crate::game::types::{Team, Vec3};
use crate::map::types::{AABB, BombSite, MapDef, SpawnZone};
use std::collections::HashMap;

/// Registry holding all available map definitions
pub struct MapRegistry {
    maps: HashMap<String, MapDef>,
}

impl MapRegistry {
    pub fn new() -> Self {
        let mut r = MapRegistry {
            maps: HashMap::new(),
        };
        r.register_all();
        r
    }

    pub fn get(&self, name: &str) -> Option<&MapDef> {
        self.maps.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.maps.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    fn add(&mut self, map: MapDef) {
        self.maps.insert(map.name.clone(), map);
    }

    fn register_all(&mut self) {
        self.add(Self::dust2());
        self.add(Self::inferno());
        self.add(Self::mirage());
        self.add(Self::nuke());
        self.add(Self::overpass());
    }

    // ─── de_dust2 ────────────────────────────────────────────────────────────
    fn dust2() -> MapDef {
        MapDef {
            name: "de_dust2".into(),
            display_name: "Dust II".into(),
            description: "Classic desert map with symmetrical mid control. \
                          Long A, Catwalk, and B tunnels define the tactical meta."
                .into(),
            ct_spawns: SpawnZone {
                team: Team::CT,
                spawns: vec![
                    Vec3::new(258.0, 2748.0, -127.0),
                    Vec3::new(390.0, 2748.0, -127.0),
                    Vec3::new(310.0, 2820.0, -127.0),
                    Vec3::new(430.0, 2820.0, -127.0),
                    Vec3::new(200.0, 2820.0, -127.0),
                ],
            },
            t_spawns: SpawnZone {
                team: Team::T,
                spawns: vec![
                    Vec3::new(-1580.0, 580.0, -127.0),
                    Vec3::new(-1460.0, 580.0, -127.0),
                    Vec3::new(-1520.0, 500.0, -127.0),
                    Vec3::new(-1400.0, 500.0, -127.0),
                    Vec3::new(-1640.0, 500.0, -127.0),
                ],
            },
            bomb_sites: vec![
                BombSite {
                    label: 'A',
                    bounds: AABB::new(
                        Vec3::new(-480.0, 2680.0, -200.0),
                        Vec3::new(100.0, 3080.0, 200.0),
                    ),
                },
                BombSite {
                    label: 'B',
                    bounds: AABB::new(
                        Vec3::new(-2150.0, 1600.0, -200.0),
                        Vec3::new(-1540.0, 2400.0, 200.0),
                    ),
                },
            ],
        }
    }

    // ─── de_inferno ──────────────────────────────────────────────────────────
    fn inferno() -> MapDef {
        MapDef {
            name: "de_inferno".into(),
            display_name: "Inferno".into(),
            description: "Italian village with tight Apartments, Banana, and mid. \
                          Heavily favors CTs at B short."
                .into(),
            ct_spawns: SpawnZone {
                team: Team::CT,
                spawns: vec![
                    Vec3::new(1100.0, 700.0, -64.0),
                    Vec3::new(1200.0, 700.0, -64.0),
                    Vec3::new(1150.0, 780.0, -64.0),
                    Vec3::new(1050.0, 780.0, -64.0),
                    Vec3::new(1250.0, 780.0, -64.0),
                ],
            },
            t_spawns: SpawnZone {
                team: Team::T,
                spawns: vec![
                    Vec3::new(-1780.0, 500.0, -64.0),
                    Vec3::new(-1680.0, 500.0, -64.0),
                    Vec3::new(-1730.0, 420.0, -64.0),
                    Vec3::new(-1630.0, 420.0, -64.0),
                    Vec3::new(-1830.0, 420.0, -64.0),
                ],
            },
            bomb_sites: vec![
                BombSite {
                    label: 'A',
                    bounds: AABB::new(
                        Vec3::new(600.0, 500.0, -200.0),
                        Vec3::new(1200.0, 900.0, 200.0),
                    ),
                },
                BombSite {
                    label: 'B',
                    bounds: AABB::new(
                        Vec3::new(-500.0, -300.0, -200.0),
                        Vec3::new(200.0, 400.0, 200.0),
                    ),
                },
            ],
        }
    }

    // ─── de_mirage ───────────────────────────────────────────────────────────
    fn mirage() -> MapDef {
        MapDef {
            name: "de_mirage".into(),
            display_name: "Mirage".into(),
            description: "Middle-eastern market. Famous for mid window control, \
                          A ramp, and short-A rushes."
                .into(),
            ct_spawns: SpawnZone {
                team: Team::CT,
                spawns: vec![
                    Vec3::new(-730.0, 500.0, -64.0),
                    Vec3::new(-630.0, 500.0, -64.0),
                    Vec3::new(-680.0, 580.0, -64.0),
                    Vec3::new(-780.0, 580.0, -64.0),
                    Vec3::new(-580.0, 580.0, -64.0),
                ],
            },
            t_spawns: SpawnZone {
                team: Team::T,
                spawns: vec![
                    Vec3::new(-2870.0, 900.0, -64.0),
                    Vec3::new(-2770.0, 900.0, -64.0),
                    Vec3::new(-2820.0, 980.0, -64.0),
                    Vec3::new(-2720.0, 980.0, -64.0),
                    Vec3::new(-2920.0, 980.0, -64.0),
                ],
            },
            bomb_sites: vec![
                BombSite {
                    label: 'A',
                    bounds: AABB::new(
                        Vec3::new(-1200.0, 200.0, -200.0),
                        Vec3::new(-600.0, 800.0, 200.0),
                    ),
                },
                BombSite {
                    label: 'B',
                    bounds: AABB::new(
                        Vec3::new(-2400.0, 200.0, -200.0),
                        Vec3::new(-1800.0, 800.0, 200.0),
                    ),
                },
            ],
        }
    }

    // ─── de_nuke ─────────────────────────────────────────────────────────────
    fn nuke() -> MapDef {
        MapDef {
            name: "de_nuke".into(),
            display_name: "Nuke".into(),
            description: "Nuclear facility with stacked A-site (upper) and B-site (lower). \
                          Vents and ramp create unique vertical gameplay."
                .into(),
            ct_spawns: SpawnZone {
                team: Team::CT,
                spawns: vec![
                    Vec3::new(700.0, 1000.0, -64.0),
                    Vec3::new(800.0, 1000.0, -64.0),
                    Vec3::new(750.0, 1080.0, -64.0),
                    Vec3::new(650.0, 1080.0, -64.0),
                    Vec3::new(850.0, 1080.0, -64.0),
                ],
            },
            t_spawns: SpawnZone {
                team: Team::T,
                spawns: vec![
                    Vec3::new(-300.0, 1000.0, -64.0),
                    Vec3::new(-200.0, 1000.0, -64.0),
                    Vec3::new(-250.0, 1080.0, -64.0),
                    Vec3::new(-150.0, 1080.0, -64.0),
                    Vec3::new(-350.0, 1080.0, -64.0),
                ],
            },
            bomb_sites: vec![
                BombSite {
                    label: 'A',
                    bounds: AABB::new(
                        Vec3::new(300.0, 800.0, -200.0),
                        Vec3::new(900.0, 1400.0, 200.0),
                    ),
                },
                BombSite {
                    label: 'B',
                    bounds: AABB::new(
                        Vec3::new(300.0, 800.0, -600.0),
                        Vec3::new(900.0, 1400.0, -200.0),
                    ),
                },
            ],
        }
    }

    // ─── de_overpass ─────────────────────────────────────────────────────────
    fn overpass() -> MapDef {
        MapDef {
            name: "de_overpass".into(),
            display_name: "Overpass".into(),
            description: "German park with water canal, connector, and long B site. \
                          Known for diverse utility usage."
                .into(),
            ct_spawns: SpawnZone {
                team: Team::CT,
                spawns: vec![
                    Vec3::new(0.0, 1000.0, -64.0),
                    Vec3::new(100.0, 1000.0, -64.0),
                    Vec3::new(50.0, 1080.0, -64.0),
                    Vec3::new(-50.0, 1080.0, -64.0),
                    Vec3::new(150.0, 1080.0, -64.0),
                ],
            },
            t_spawns: SpawnZone {
                team: Team::T,
                spawns: vec![
                    Vec3::new(-1500.0, 1000.0, -64.0),
                    Vec3::new(-1400.0, 1000.0, -64.0),
                    Vec3::new(-1450.0, 1080.0, -64.0),
                    Vec3::new(-1350.0, 1080.0, -64.0),
                    Vec3::new(-1550.0, 1080.0, -64.0),
                ],
            },
            bomb_sites: vec![
                BombSite {
                    label: 'A',
                    bounds: AABB::new(
                        Vec3::new(-600.0, 600.0, -200.0),
                        Vec3::new(100.0, 1200.0, 200.0),
                    ),
                },
                BombSite {
                    label: 'B',
                    bounds: AABB::new(
                        Vec3::new(-2100.0, 600.0, -200.0),
                        Vec3::new(-1400.0, 1200.0, 200.0),
                    ),
                },
            ],
        }
    }
}

impl Default for MapRegistry {
    fn default() -> Self {
        Self::new()
    }
}
