pub const ERB_DECIMALS: u128 = 1_0000_0000; // example 10^8 smallest units
pub const INITIAL_BLOCK_REWARD: u128 = 15 * ERB_DECIMALS; // 15 ERB
pub const HALVING_INTERVAL_BLOCKS: u64 = 1_051_200; // ~4 years @30s

pub struct RewardCalculator {
    annual_emission: u64, // legacy
    blocks_per_year: u64, // legacy
}

impl RewardCalculator {
    pub fn new(annual_emission: u64, blocks_per_year: u64) -> Self {
        Self {
            annual_emission,
            blocks_per_year,
        }
    }

    /// Legacy: Calculate annual reward for a validator based on stake
    pub fn calculate_annual_reward(&self, validator_stake: u64, total_stake: u64) -> u64 {
        if total_stake == 0 {
            return 0;
        }
        (validator_stake * self.annual_emission) / total_stake
    }

    /// Legacy: Calculate block reward for validator
    pub fn calculate_block_reward(&self, validator_stake: u64, total_stake: u64) -> u64 {
        if total_stake == 0 {
            return 0;
        }
        let emission_per_block = self.annual_emission / self.blocks_per_year;
        (validator_stake * emission_per_block) / total_stake
    }

    /// Legacy: delegator rewards split
    pub fn calculate_delegator_rewards(
        &self,
        block_reward: u64,
        validator_stake: u64,
        delegator_stake: u64,
        commission_rate: u8,
    ) -> (u64, u64) {
        if validator_stake == 0 {
            return (0, 0);
        }
        let delegator_share = (delegator_stake * block_reward) / validator_stake;
        let commission = (delegator_share * commission_rate as u64) / 100;
        let delegator_reward = delegator_share - commission;
        let validator_reward = block_reward - delegator_share + commission;
        (validator_reward, delegator_reward)
    }
}

/// New halving-based emission policy
pub struct RewardPolicy {
    pub initial_block_reward: u128,
    pub halving_interval_blocks: u64,
}

impl Default for RewardPolicy {
    fn default() -> Self {
        Self {
            initial_block_reward: INITIAL_BLOCK_REWARD,
            halving_interval_blocks: HALVING_INTERVAL_BLOCKS,
        }
    }
}

impl RewardPolicy {
    pub fn reward_for_height(&self, height: u64) -> u128 {
        let halvings = height / self.halving_interval_blocks;
        // Shift-right per halving, saturating at zero after many halvings
        self.initial_block_reward
            .checked_shr(halvings as u32)
            .unwrap_or(0)
    }
}

/// Economics tracking for burn/emission
#[derive(Default, Debug, Clone)]
pub struct Economics {
    pub total_supply: u128,
    pub cumulative_issued: u128,
    pub cumulative_burned: u128,
    pub base_gas_price: u128,
    pub target_gas_per_block: u64,
    pub max_change_ratio_bps: u32, // basis points (1/100 of a percent)
}

impl Economics {
    pub fn apply_block_economics(
        &mut self,
        block_height: u64,
        tx_fees_burned: u128,
        reward_policy: &RewardPolicy,
    ) {
        let reward = reward_policy.reward_for_height(block_height);
        self.cumulative_issued = self.cumulative_issued.saturating_add(reward);
        self.total_supply = self.total_supply.saturating_add(reward);
        self.cumulative_burned = self.cumulative_burned.saturating_add(tx_fees_burned);
        self.total_supply = self.total_supply.saturating_sub(tx_fees_burned);
    }

    pub fn adjust_base_gas_price(&mut self, block_gas_used: u64) {
        if self.target_gas_per_block == 0 || self.base_gas_price == 0 {
            return;
        }
        let target = self.target_gas_per_block as i128;
        let used = block_gas_used as i128;
        let diff = used - target;
        if diff == 0 {
            return;
        }
        let abs_ratio_bps = ((diff.unsigned_abs()) * 10_000u128 / (target.unsigned_abs())) as u32;
        let delta_bps = abs_ratio_bps.min(self.max_change_ratio_bps);
        let delta = (self.base_gas_price * delta_bps as u128) / 10_000u128;
        if diff > 0 {
            self.base_gas_price = self.base_gas_price.saturating_add(delta);
        } else {
            self.base_gas_price = self.base_gas_price.saturating_sub(delta);
        }
    }
}
