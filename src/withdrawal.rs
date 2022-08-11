use crate::assets::Account;
use crate::montecarlo::Period;

pub trait WithdrawalStrategy {
    fn execute(&self, withdrawal: f64, accounts: &mut Vec<Account>, period: Period) -> Result<(), f64>;
}

pub struct WithdrawalStrategyOrig {

}

impl WithdrawalStrategyOrig {
    pub fn new() -> WithdrawalStrategyOrig {
        WithdrawalStrategyOrig { }
    }
}

impl WithdrawalStrategy for WithdrawalStrategyOrig {
    fn execute(&self, withdrawal: f64, accounts: &mut Vec<Account>, period: Period) -> Result<(), f64> {
        let total: f64 = accounts.iter().map(|a| a.balance()[period.get()]).sum();
        let withdrawals_per_account: Vec<f64> = accounts.iter().map(|a| (a.balance()[period.get()] / total) * withdrawal).collect();
        
        let mut shortfall = 0.0;
        for i in 0..accounts.len() {
            shortfall += accounts[i].attempt_withdrawal_with_shortfall(withdrawals_per_account[i], period);
        }

        if shortfall != 0.0 {
            Err(shortfall)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::assets::{AssetAllocation,AccountSettings};
    use crate::montecarlo::Lifespan;
    use crate::rates::Rate;

    #[test]
    pub fn withdrawalstrategyorig_executesuccess() {
        let dummy_allocation = Rc::new(AssetAllocation::new(vec![1.0]));
        let mut account1 = AccountSettings::new(1536.0, Rc::clone(&dummy_allocation)).create_account(Lifespan::new(1), Rc::new(vec![Rate::new(1.0, 1.0, 1.0)]));
        let mut account2 = AccountSettings::new(512.0, dummy_allocation).create_account(Lifespan::new(1), Rc::new(vec![Rate::new(1.0, 1.0, 1.0)]));
        account1.rebalance_and_invest_next_period(Period::new(0));
        account2.rebalance_and_invest_next_period(Period::new(0));

        let mut accounts = vec![account1, account2];

        let strategy = WithdrawalStrategyOrig::new();
        strategy.execute(512.0, &mut accounts, Period::new(0)).expect("should have enough");
    }

    #[test]
    pub fn withdrawalstrategyorig_executefailure() {
        let dummy_allocation = Rc::new(AssetAllocation::new(vec![1.0]));
        let mut account1 = AccountSettings::new(1536.0, Rc::clone(&dummy_allocation)).create_account(Lifespan::new(1), Rc::new(vec![Rate::new(1.0, 1.0, 1.0)]));
        let mut account2 = AccountSettings::new(512.0, dummy_allocation).create_account(Lifespan::new(1), Rc::new(vec![Rate::new(1.0, 1.0, 1.0)]));
        account1.rebalance_and_invest_next_period(Period::new(0));
        account2.rebalance_and_invest_next_period(Period::new(0));

        let mut accounts = vec![account1, account2];

        let strategy = WithdrawalStrategyOrig::new();
        assert_eq!(2048.0, strategy.execute(4096.0, &mut accounts, Period::new(0)).expect_err("shouldn't have enough"));
    }
}