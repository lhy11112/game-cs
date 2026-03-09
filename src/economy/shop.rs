use crate::game::error::GameError;
use crate::game::types::RoundPhase;
use crate::player::types::Player;
use crate::weapons::catalog::WeaponCatalog;
use crate::weapons::types::Weapon;
use serde::{Deserialize, Serialize};

/// Category of item in the buy menu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuyItemKind {
    Weapon,
    Kevlar,
    KevlarHelmet,
    DefuseKit,
    Taser,
}

/// A single item available for purchase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyItem {
    pub id: String,
    pub kind: BuyItemKind,
    pub price: u32,
    pub description: String,
}

/// The buy menu system - handles purchases during freeze time
pub struct BuyMenu {
    catalog: WeaponCatalog,
}

impl BuyMenu {
    pub fn new() -> Self {
        BuyMenu {
            catalog: WeaponCatalog::new(),
        }
    }

    /// List all available weapon items
    pub fn available_weapons(&self) -> Vec<BuyItem> {
        self.catalog
            .all()
            .filter(|w| w.price > 0)
            .map(|w| BuyItem {
                id: w.name.clone(),
                kind: BuyItemKind::Weapon,
                price: w.price,
                description: format!(
                    "{} | DMG:{:.0} | MAG:{} | ${}",
                    w.name, w.base_damage, w.magazine_size, w.price
                ),
            })
            .collect()
    }

    /// Purchase a weapon for the player.
    /// Only allowed during freeze time.
    pub fn buy_weapon(
        &self,
        player: &mut Player,
        weapon_name: &str,
        round_phase: RoundPhase,
        _max_money: u32,
    ) -> Result<Option<Weapon>, GameError> {
        if round_phase != RoundPhase::FreezeTime {
            return Err(GameError::InvalidAction(
                "Can only buy weapons during freeze time".into(),
            ));
        }

        let stats = self
            .catalog
            .get(weapon_name)
            .ok_or_else(|| GameError::WeaponNotFound(weapon_name.into()))?
            .clone();

        let price = stats.price;
        if player.money < price {
            return Err(GameError::InsufficientFunds {
                needed: price,
                available: player.money,
            });
        }

        player.spend_money(price);
        let weapon = Weapon::new(stats);
        let displaced = player.inventory.equip(weapon);
        Ok(displaced)
    }

    /// Purchase Kevlar vest (without helmet)
    pub fn buy_kevlar(
        &self,
        player: &mut Player,
        round_phase: RoundPhase,
    ) -> Result<(), GameError> {
        if round_phase != RoundPhase::FreezeTime {
            return Err(GameError::InvalidAction(
                "Can only buy equipment during freeze time".into(),
            ));
        }
        const KEVLAR_PRICE: u32 = 650;
        if player.money < KEVLAR_PRICE {
            return Err(GameError::InsufficientFunds {
                needed: KEVLAR_PRICE,
                available: player.money,
            });
        }
        player.spend_money(KEVLAR_PRICE);
        player.armor.vest_hp = 100;
        Ok(())
    }

    /// Purchase full armor (Kevlar + Helmet)
    pub fn buy_kevlar_helmet(
        &self,
        player: &mut Player,
        round_phase: RoundPhase,
    ) -> Result<(), GameError> {
        if round_phase != RoundPhase::FreezeTime {
            return Err(GameError::InvalidAction(
                "Can only buy equipment during freeze time".into(),
            ));
        }
        let price: u32 = if player.armor.vest_hp > 0 { 350 } else { 1000 };
        if player.money < price {
            return Err(GameError::InsufficientFunds {
                needed: price,
                available: player.money,
            });
        }
        player.spend_money(price);
        player.armor.vest_hp = 100;
        player.armor.has_helmet = true;
        Ok(())
    }

    /// Purchase defuse kit (CT only)
    pub fn buy_defuse_kit(
        &self,
        player: &mut Player,
        round_phase: RoundPhase,
    ) -> Result<(), GameError> {
        if round_phase != RoundPhase::FreezeTime {
            return Err(GameError::InvalidAction(
                "Can only buy equipment during freeze time".into(),
            ));
        }
        use crate::game::types::Team;
        if player.team != Team::CT {
            return Err(GameError::InvalidAction(
                "Only CTs can buy defuse kits".into(),
            ));
        }
        const KIT_PRICE: u32 = 400;
        if player.money < KIT_PRICE {
            return Err(GameError::InsufficientFunds {
                needed: KIT_PRICE,
                available: player.money,
            });
        }
        player.spend_money(KIT_PRICE);
        player.armor.has_defuse_kit = true;
        Ok(())
    }
}

impl Default for BuyMenu {
    fn default() -> Self {
        Self::new()
    }
}
