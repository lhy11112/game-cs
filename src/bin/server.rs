use game_cs::game::types::{GameMode, MatchConfig};
use game_cs::map::maps::MapRegistry;
use game_cs::network::matchmaking::MatchmakingQueue;

fn main() {
    env_logger::init();
    println!("=== game-cs Server ===");
    println!("Initializing game server components...\n");

    let map_registry = MapRegistry::new();
    println!("Maps loaded:");
    for m in map_registry.list() {
        println!("  - {}", m);
    }
    println!();

    // Initialize matchmaking queue
    let _queue = MatchmakingQueue::new();

    println!("Server ready. Accepting connections...");
    println!("(This is a demonstration server - no network layer yet)");
    println!("\nAvailable game modes:");
    for mode in &[GameMode::BombDefusal, GameMode::TeamDeathmatch, GameMode::Deathmatch] {
        println!("  - {:?}", mode);
    }

    println!("\nMatch configuration (default):");
    let cfg = MatchConfig::default();
    println!("  Max rounds:     {}", cfg.max_rounds);
    println!("  Rounds to win:  {}", cfg.rounds_to_win);
    println!("  Freeze time:    {}s", cfg.freeze_time_secs);
    println!("  Round time:     {}s", cfg.round_time_secs);
    println!("  Bomb timer:     {}s", cfg.bomb_timer_secs);
    println!("  Plant time:     {}s", cfg.plant_time_secs);
    println!("  Defuse time:    {}s ({}s with kit)", cfg.defuse_time_secs, cfg.defuse_kit_time_secs);
    println!("  Start money:    ${}", cfg.start_money);
    println!("  Max money:      ${}", cfg.max_money);
}
