use crate::game::types::{RoundMoneyRewards, RoundResult, Team};
use serde::{Deserialize, Serialize};

/// Money reward summary for a team after a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRewardSummary {
    pub team: Team,
    pub base_reward: u32,
    pub loss_bonus: u32,
    pub total_reward: u32,
}

/// Calculator for end-of-round money distribution
pub struct RoundRewardCalculator {
    pub rewards: RoundMoneyRewards,
}

impl RoundRewardCalculator {
    pub fn new(rewards: RoundMoneyRewards) -> Self {
        RoundRewardCalculator { rewards }
    }

    /// Compute end-of-round money rewards for each team.
    ///
    /// `ct_consecutive_losses` / `t_consecutive_losses`: how many consecutive
    /// rounds that team has lost going into this round.
    pub fn compute(
        &self,
        result: RoundResult,
        ct_consecutive_losses: u32,
        t_consecutive_losses: u32,
    ) -> (TeamRewardSummary, TeamRewardSummary) {
        match result {
            RoundResult::CTWin(_) => {
                let ct = TeamRewardSummary {
                    team: Team::CT,
                    base_reward: self.rewards.ct_win,
                    loss_bonus: 0,
                    total_reward: self.rewards.ct_win,
                };
                let t_bonus = self.loss_bonus(t_consecutive_losses, self.rewards.t_loss_base);
                let t = TeamRewardSummary {
                    team: Team::T,
                    base_reward: self.rewards.t_loss_base,
                    loss_bonus: t_bonus,
                    total_reward: self.rewards.t_loss_base + t_bonus,
                };
                (ct, t)
            }
            RoundResult::TWin(_) => {
                let ct_bonus =
                    self.loss_bonus(ct_consecutive_losses, self.rewards.ct_loss);
                let ct = TeamRewardSummary {
                    team: Team::CT,
                    base_reward: self.rewards.ct_loss,
                    loss_bonus: ct_bonus,
                    total_reward: self.rewards.ct_loss + ct_bonus,
                };
                let t = TeamRewardSummary {
                    team: Team::T,
                    base_reward: self.rewards.t_win,
                    loss_bonus: 0,
                    total_reward: self.rewards.t_win,
                };
                (ct, t)
            }
            RoundResult::InProgress => {
                // No reward mid-round
                (
                    TeamRewardSummary { team: Team::CT, base_reward: 0, loss_bonus: 0, total_reward: 0 },
                    TeamRewardSummary { team: Team::T, base_reward: 0, loss_bonus: 0, total_reward: 0 },
                )
            }
        }
    }

    /// Calculate the loss bonus for `consecutive_losses` consecutive losses
    fn loss_bonus(&self, consecutive_losses: u32, _base_loss_award: u32) -> u32 {
        if consecutive_losses == 0 {
            return 0;
        }
        let bonus_tiers = consecutive_losses.min(self.rewards.max_loss_bonus);
        self.rewards.loss_bonus_increment * bonus_tiers
    }
}

impl Default for RoundRewardCalculator {
    fn default() -> Self {
        RoundRewardCalculator::new(RoundMoneyRewards::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::types::{CTWinReason, TWinReason};

    #[test]
    fn test_ct_win_rewards() {
        let calc = RoundRewardCalculator::default();
        let (ct, t) = calc.compute(RoundResult::CTWin(CTWinReason::TerrorsEliminated), 0, 3);
        assert_eq!(ct.total_reward, 3250);
        // T team had 3 consecutive losses: base 1400 + 3*500 = 2900
        assert_eq!(t.total_reward, 2900);
    }

    #[test]
    fn test_t_win_rewards() {
        let calc = RoundRewardCalculator::default();
        let (ct, t) = calc.compute(RoundResult::TWin(TWinReason::BombExploded), 2, 0);
        // CT had 2 consecutive losses: base 1400 + 2*500 = 2400
        assert_eq!(ct.total_reward, 2400);
        assert_eq!(t.total_reward, 3250);
    }

    #[test]
    fn test_loss_bonus_cap() {
        let calc = RoundRewardCalculator::default();
        // 10 consecutive losses should be capped at max_loss_bonus (4)
        let (ct, _) = calc.compute(RoundResult::TWin(TWinReason::CTsEliminated), 10, 0);
        assert_eq!(ct.loss_bonus, 4 * 500);
    }
}
