use crate::game::error::GameError;
use crate::game::types::{GameMode, Rank, Team};
use crate::player::types::PlayerProfile;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for a custom room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub map_name: String,
    pub game_mode: GameMode,
    pub max_players: u8,
    /// Optional password; None = public room
    pub password: Option<String>,
}

impl Default for RoomConfig {
    fn default() -> Self {
        RoomConfig {
            map_name: "de_dust2".into(),
            game_mode: GameMode::BombDefusal,
            max_players: 10,
            password: None,
        }
    }
}

/// Phase of the room lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomState {
    Lobby,
    Starting,
    InGame,
    PostGame,
}

/// A player slot in the room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSlot {
    pub player_id: String,
    pub nickname: String,
    pub team: Team,
    pub is_ready: bool,
    pub rank: Rank,
}

/// A game room / lobby
#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub owner_id: String,
    pub config: RoomConfig,
    pub state: RoomState,
    pub slots: HashMap<String, RoomSlot>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl Room {
    pub fn new(owner: &PlayerProfile, config: RoomConfig) -> Self {
        let room_id = Uuid::new_v4().to_string();
        let mut slots = HashMap::new();
        slots.insert(
            owner.id.clone(),
            RoomSlot {
                player_id: owner.id.clone(),
                nickname: owner.nickname.clone(),
                team: Team::CT,
                is_ready: false,
                rank: owner.rank,
            },
        );

        Room {
            id: room_id,
            owner_id: owner.id.clone(),
            config,
            state: RoomState::Lobby,
            slots,
            created_at: Utc::now(),
            last_activity: Utc::now(),
        }
    }

    pub fn player_count(&self) -> usize {
        self.slots.len()
    }

    pub fn is_full(&self) -> bool {
        self.slots.len() >= self.config.max_players as usize
    }

    pub fn is_idle_for(&self, secs: i64) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_activity)
            .num_seconds();
        elapsed >= secs
    }

    /// Join the room. Returns the assigned team.
    pub fn join(
        &mut self,
        profile: &PlayerProfile,
        password: Option<&str>,
    ) -> Result<Team, GameError> {
        if self.state != RoomState::Lobby {
            return Err(GameError::InvalidAction("Room is not in lobby state".into()));
        }
        if self.is_full() {
            return Err(GameError::RoomFull);
        }
        if self.slots.contains_key(&profile.id) {
            return Err(GameError::PlayerAlreadyInRoom);
        }
        // Check password
        match (&self.config.password, password) {
            (Some(_required), None) => return Err(GameError::RoomPasswordRequired),
            (Some(required), Some(given)) if required != given => {
                return Err(GameError::WrongRoomPassword)
            }
            _ => {}
        }

        // Auto-assign to smaller team
        let ct_count = self.slots.values().filter(|s| s.team == Team::CT).count();
        let t_count = self.slots.values().filter(|s| s.team == Team::T).count();
        let team = if t_count < ct_count { Team::T } else { Team::CT };

        self.slots.insert(
            profile.id.clone(),
            RoomSlot {
                player_id: profile.id.clone(),
                nickname: profile.nickname.clone(),
                team,
                is_ready: false,
                rank: profile.rank,
            },
        );
        self.touch();
        Ok(team)
    }

    /// Leave the room
    pub fn leave(&mut self, player_id: &str) -> Result<(), GameError> {
        self.slots
            .remove(player_id)
            .ok_or_else(|| GameError::PlayerNotFound(player_id.into()))?;
        // Transfer ownership if owner leaves
        if self.owner_id == player_id {
            if let Some(new_owner_id) = self.slots.keys().next().cloned() {
                self.owner_id = new_owner_id;
            }
        }
        self.touch();
        Ok(())
    }

    /// Toggle ready state for a player
    pub fn set_ready(&mut self, player_id: &str, ready: bool) -> Result<(), GameError> {
        let slot = self
            .slots
            .get_mut(player_id)
            .ok_or_else(|| GameError::PlayerNotFound(player_id.into()))?;
        slot.is_ready = ready;
        self.touch();
        Ok(())
    }

    /// Kick a player (owner only)
    pub fn kick(
        &mut self,
        requester_id: &str,
        target_id: &str,
    ) -> Result<(), GameError> {
        if requester_id != self.owner_id {
            return Err(GameError::NotRoomOwner);
        }
        self.slots
            .remove(target_id)
            .ok_or_else(|| GameError::PlayerNotFound(target_id.into()))?;
        self.touch();
        Ok(())
    }

    /// Change the map (owner only)
    pub fn change_map(
        &mut self,
        requester_id: &str,
        map_name: String,
    ) -> Result<(), GameError> {
        if requester_id != self.owner_id {
            return Err(GameError::NotRoomOwner);
        }
        self.config.map_name = map_name;
        // Reset ready states on map change
        for slot in self.slots.values_mut() {
            slot.is_ready = false;
        }
        self.touch();
        Ok(())
    }

    /// Check if all players are ready and owner can start
    pub fn all_ready(&self) -> bool {
        self.slots.len() >= 2 && self.slots.values().all(|s| s.is_ready)
    }

    /// Start the game (owner only, all must be ready)
    pub fn start_game(&mut self, requester_id: &str) -> Result<(), GameError> {
        if requester_id != self.owner_id {
            return Err(GameError::NotRoomOwner);
        }
        if !self.all_ready() {
            return Err(GameError::InvalidAction(
                "Not all players are ready".into(),
            ));
        }
        self.state = RoomState::Starting;
        self.touch();
        Ok(())
    }

    fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
}
