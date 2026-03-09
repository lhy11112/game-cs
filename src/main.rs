use game_cs::game::r#match::Match;
use game_cs::game::types::{
    CTWinReason, GameMode, HitZone, MatchConfig, Rank, RoundResult, Team,
    TWinReason,
};
use game_cs::map::maps::MapRegistry;
use game_cs::network::matchmaking::{MatchmakingEntry, MatchmakingQueue};
use game_cs::network::room::{Room, RoomConfig};
use game_cs::player::types::{Player, PlayerProfile};
use game_cs::weapons::catalog::WeaponCatalog;

fn main() {
    env_logger::init();
    println!("=== game-cs: CS-Style Tactical Shooter Engine (Rust) ===\n");

    demo_weapon_catalog();
    demo_matchmaking();
    demo_room_system();
    demo_match_simulation();
    demo_economic_system();
}

// ─── Demo: Weapon Catalog ──────────────────────────────────────────────────

fn demo_weapon_catalog() {
    println!("--- Weapon Catalog ---");
    let catalog = WeaponCatalog::new();
    let mut weapons: Vec<_> = catalog.all().collect();
    weapons.sort_by(|a, b| a.price.cmp(&b.price).then(a.name.cmp(&b.name)));

    println!(
        "{:<20} {:<12} {:>8} {:>8} {:>6} {:>10}",
        "Name", "Kind", "Price", "Damage", "Mag", "Fire(RPM)"
    );
    println!("{}", "-".repeat(72));
    for w in &weapons {
        println!(
            "{:<20} {:<12} {:>8} {:>8.1} {:>6} {:>10.0}",
            w.name,
            format!("{:?}", w.kind),
            w.price,
            w.base_damage,
            w.magazine_size,
            w.fire_rate_rpm,
        );
    }
    println!("Total weapons in catalog: {}\n", weapons.len());
}

// ─── Demo: Matchmaking ─────────────────────────────────────────────────────

fn demo_matchmaking() {
    println!("--- Matchmaking System ---");
    let mut queue = MatchmakingQueue::new();

    // All players within a 3-rank spread: GoldNovaII..GoldNovaMaster
    let players = vec![
        ("p1", "Sniper_King", Rank::GoldNovaIII),
        ("p2", "HeadshotMaster", Rank::GoldNovaMaster),
        ("p3", "AWPer", Rank::GoldNovaMaster),
        ("p4", "RifleGod", Rank::GoldNovaII),
        ("p5", "FlashBanger", Rank::GoldNovaIII),
        ("p6", "SmokeArtist", Rank::GoldNovaII),
        ("p7", "BombPlanter", Rank::GoldNovaMaster),
        ("p8", "DefuseKit", Rank::GoldNovaMaster),
        ("p9", "Lurker", Rank::GoldNovaIII),
        ("p10", "EntryFragger", Rank::GoldNovaII),
    ];

    for (id, nick, rank) in &players {
        queue.enqueue(MatchmakingEntry::new(
            id.to_string(),
            nick.to_string(),
            *rank,
            GameMode::BombDefusal,
            20,
        ));
    }

    println!("Queue size: {}", queue.queue_size());
    let est = queue.estimated_wait_secs(Rank::GoldNovaIII, GameMode::BombDefusal);
    println!("Estimated wait for GoldNovaIII: {}s", est);

    let maps = vec!["de_dust2", "de_mirage", "de_inferno"];
    if let Some(result) = queue.try_form_match(&maps) {
        println!("Match formed!");
        println!("  Map: {}", result.map_name);
        println!("  Mode: {:?}", result.game_mode);
        println!("  Players: {:?}", result.player_ids);
    }
    println!("Queue after match: {}\n", queue.queue_size());
}

// ─── Demo: Room System ─────────────────────────────────────────────────────

fn demo_room_system() {
    println!("--- Room System ---");

    let owner = PlayerProfile::new("owner1".into(), "RoomOwner".into());
    let config = RoomConfig {
        map_name: "de_inferno".into(),
        game_mode: GameMode::BombDefusal,
        max_players: 10,
        password: Some("cs2024".into()),
    };

    let mut room = Room::new(&owner, config);
    println!("Room created: {} (owner: {})", room.id, room.owner_id);

    // Try joining without password
    let guest = PlayerProfile::new("g1".into(), "GuestPlayer".into());
    match room.join(&guest, None) {
        Err(e) => println!("Join without password (expected error): {}", e),
        Ok(_) => println!("Unexpectedly joined without password"),
    }

    // Join with correct password
    match room.join(&guest, Some("cs2024")) {
        Ok(team) => println!("GuestPlayer joined as {:?}", team),
        Err(e) => println!("Error: {}", e),
    }

    // Set ready states
    room.set_ready(&owner.id, true).unwrap();
    room.set_ready(&guest.id, true).unwrap();
    println!("All ready: {}", room.all_ready());

    // Owner starts game
    match room.start_game(&owner.id) {
        Ok(_) => println!("Game started!"),
        Err(e) => println!("Error: {}", e),
    }
    println!();
}

// ─── Demo: Match Simulation ────────────────────────────────────────────────

fn demo_match_simulation() {
    println!("--- Match Simulation ---");

    let registry = MapRegistry::new();
    let map = registry.get("de_dust2").expect("de_dust2 must exist").clone();
    let config = MatchConfig::default();

    let mut game_match = Match::new(config, map);

    // Create 5v5 roster
    for i in 1..=5 {
        let p = Player::new(
            PlayerProfile::new(format!("ct{}", i), format!("CT_Player{}", i)),
            Team::CT,
            800,
        );
        game_match.add_player(p).unwrap();
    }
    for i in 1..=5 {
        let p = Player::new(
            PlayerProfile::new(format!("t{}", i), format!("T_Player{}", i)),
            Team::T,
            800,
        );
        game_match.add_player(p).unwrap();
    }

    game_match.start().unwrap();
    println!("Match started: {}", game_match.id);
    println!("Players: {}", game_match.players.len());

    // Simulate freeze time passing (15s)
    println!("Simulating freeze time (15s)...");
    let mut result = None;
    let mut t = 0.0f32;
    while t < 15.5 && result.is_none() {
        result = game_match.tick(0.5);
        t += 0.5;
    }
    println!("Freeze time ended. Round is live.");

    // Simulate T team eliminating all CTs
    let t_secs = 20.0f32;
    let mut t2 = 0.0f32;
    while t2 < t_secs && result.is_none() {
        // Kill CT1 at t=5s
        if t2 >= 5.0 && t2 < 5.5 {
            if let Some(victim) = game_match.players.get_mut("ct1") {
                victim.take_damage(200.0);
            }
            game_match
                .record_kill("t1", "ct1", "AK-47", HitZone::Head, true, 5000)
                .unwrap();
        }
        // Kill remaining CTs at t=10s
        if t2 >= 10.0 && t2 < 10.5 {
            for ci in 2..=5 {
                if let Some(victim) = game_match.players.get_mut(&format!("ct{}", ci)) {
                    victim.take_damage(200.0);
                }
                game_match
                    .record_kill(
                        "t2",
                        &format!("ct{}", ci),
                        "AK-47",
                        HitZone::Chest,
                        false,
                        10000,
                    )
                    .ok();
            }
        }
        result = game_match.tick(0.5);
        t2 += 0.5;
    }

    match result {
        Some(RoundResult::TWin(reason)) => {
            println!("Round 1 ended: T Win ({:?})", reason);
        }
        Some(RoundResult::CTWin(reason)) => {
            println!("Round 1 ended: CT Win ({:?})", reason);
        }
        _ => println!("Round still in progress"),
    }

    println!(
        "Score — CT: {} | T: {}\n",
        game_match.score.ct, game_match.score.t
    );
}

// ─── Demo: Economic System ─────────────────────────────────────────────────

fn demo_economic_system() {
    use game_cs::economy::rewards::RoundRewardCalculator;

    println!("--- Economic System ---");
    let calc = RoundRewardCalculator::default();

    println!("Scenario 1: CT wins (T had 3 consecutive losses)");
    let (ct, t) = calc.compute(
        RoundResult::CTWin(CTWinReason::TerrorsEliminated),
        0,
        3,
    );
    println!(
        "  CT: ${} (base ${} + bonus ${})",
        ct.total_reward, ct.base_reward, ct.loss_bonus
    );
    println!(
        "  T:  ${} (base ${} + bonus ${})",
        t.total_reward, t.base_reward, t.loss_bonus
    );

    println!("Scenario 2: T wins by bomb explosion (CT had 2 consecutive losses)");
    let (ct2, t2) = calc.compute(RoundResult::TWin(TWinReason::BombExploded), 2, 0);
    println!(
        "  CT: ${} (base ${} + bonus ${})",
        ct2.total_reward, ct2.base_reward, ct2.loss_bonus
    );
    println!(
        "  T:  ${} (base ${} + bonus ${})",
        t2.total_reward, t2.base_reward, t2.loss_bonus
    );

    println!("Scenario 3: T wins, CT on 5-loss streak (bonus capped)");
    let (ct3, _) = calc.compute(RoundResult::TWin(TWinReason::CTsEliminated), 5, 0);
    println!(
        "  CT: ${} (base ${} + bonus ${})\n",
        ct3.total_reward, ct3.base_reward, ct3.loss_bonus
    );
}
