/// [RJJ-SUPPLY] Asset supply calculation service
/// 
/// This module handles the calculation of total_supply for assets based on UNSPENT UTXOs.
/// 
/// ## Supply Calculation Rules:
/// 1. total_supply = SUM(amount) WHERE spent = false
/// 2. When new charm is created: supply += amount (after confirmation)
/// 3. When charm is marked as spent: supply -= amount
/// 4. In transfers: supply remains constant (spends N, creates N)
/// 
/// ## Burn Detection [RJJ-BURN]:
/// A burn occurs when tokens are permanently removed from circulation:
/// - UTXO is spent but no new UTXO is created with that amount
/// - Output to OP_RETURN address (unspendable)
/// - Output to known burn address
/// 
/// When burn is detected: supply -= burned_amount

use crate::domain::errors::CharmError;
use crate::infrastructure::persistence::repositories::CharmRepository;

/// Handles supply calculations for assets
pub struct AssetSupplyCalculator<'a> {
    charm_repository: &'a CharmRepository,
}

impl<'a> AssetSupplyCalculator<'a> {
    pub fn new(charm_repository: &'a CharmRepository) -> Self {
        Self { charm_repository }
    }

    /// Calculate total supply for an asset based on UNSPENT charms
    /// [RJJ-SUPPLY] Only counts charms where spent = false
    pub async fn calculate_supply_for_asset(&self, app_id: &str) -> Result<i64, CharmError> {
        // Get all charms for this asset
        let charms = self.charm_repository
            .find_by_app_id(app_id)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to get charms: {}", e)))?;
        
        // Sum amounts of UNSPENT charms only
        let total_supply: i64 = charms
            .iter()
            .filter(|charm| !charm.spent) // [RJJ-SUPPLY] Only unspent
            .map(|charm| charm.amount)
            .sum();
        
        Ok(total_supply)
    }

    /// Calculate supply change when a charm is spent
    /// [RJJ-SUPPLY] Returns negative value to subtract from supply
    pub fn calculate_supply_change_on_spend(&self, amount: i64) -> i64 {
        -amount // Subtract from supply
    }

    /// Calculate supply change when a new charm is created
    /// [RJJ-SUPPLY] Returns positive value to add to supply
    pub fn calculate_supply_change_on_create(&self, amount: i64) -> i64 {
        amount // Add to supply
    }

    /// Detect if a transaction is a burn operation
    /// [RJJ-BURN] TODO: Implement burn detection logic
    /// 
    /// A burn is detected when:
    /// 1. Input charm is spent (amount X)
    /// 2. No output charm is created with amount X
    /// 3. Or output is to OP_RETURN / burn address
    /// 
    /// Returns: (is_burn, burned_amount)
    pub fn detect_burn(
        &self,
        _spent_charms: &[i64],  // amounts of spent charms
        _created_charms: &[i64], // amounts of created charms
    ) -> (bool, i64) {
        // TODO: [RJJ-BURN] Implement burn detection
        // Compare spent vs created amounts
        // If spent > created, difference is burned
        (false, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supply_change_calculations() {
        let calculator = AssetSupplyCalculator {
            charm_repository: unsafe { std::mem::zeroed() }, // Mock for test
        };
        
        // Test spend (should be negative)
        assert_eq!(calculator.calculate_supply_change_on_spend(1000), -1000);
        
        // Test create (should be positive)
        assert_eq!(calculator.calculate_supply_change_on_create(1000), 1000);
    }
}
