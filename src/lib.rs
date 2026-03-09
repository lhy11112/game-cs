/// game-cs: A CS-style tactical shooter game engine implemented in Rust
///
/// This library provides the core game systems for a Counter-Strike style
/// first-person shooter, including:
/// - Weapon system with realistic parameters
/// - Player movement, health, and actions
/// - Economic system (buying weapons/equipment)
/// - Match/round management with bomb defusal mode
/// - Matchmaking and room management
/// - Statistics and settlement

pub mod weapons;
pub mod player;
pub mod economy;
pub mod game;
pub mod map;
pub mod network;
pub mod stats;

pub use game::error::GameError;
pub type Result<T> = std::result::Result<T, GameError>;
