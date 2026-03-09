use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Player not found: {0}")]
    PlayerNotFound(String),

    #[error("Room not found: {0}")]
    RoomNotFound(String),

    #[error("Match not found: {0}")]
    MatchNotFound(String),

    #[error("Weapon not found: {0}")]
    WeaponNotFound(String),

    #[error("Insufficient funds: need {needed}, have {available}")]
    InsufficientFunds { needed: u32, available: u32 },

    #[error("Invalid action: {0}")]
    InvalidAction(String),

    #[error("Round already ended")]
    RoundEnded,

    #[error("Match already ended")]
    MatchEnded,

    #[error("Player is dead")]
    PlayerDead,

    #[error("Player already in room")]
    PlayerAlreadyInRoom,

    #[error("Room is full")]
    RoomFull,

    #[error("Room is password protected")]
    RoomPasswordRequired,

    #[error("Wrong room password")]
    WrongRoomPassword,

    #[error("Not room owner")]
    NotRoomOwner,

    #[error("Bomb already planted")]
    BombAlreadyPlanted,

    #[error("Not at bomb site")]
    NotAtBombSite,

    #[error("No bomb in inventory")]
    NoBomb,

    #[error("Bomb not planted")]
    BombNotPlanted,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}
