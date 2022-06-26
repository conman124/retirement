use crate::assets::Account;

pub struct WithdrawalStrategy {
    // TODO this is kinda meh.  This doesn't enforce any real relation between the account and the withdrawal.  Hopefully,
    // the order and number of accounts stays the same, but there's nothing that enforces that...
    withdrawals_per_account: Vec<f64>,
    period: usize
}

impl<'a> WithdrawalStrategy {
    pub fn new(monthly_withdrawal: f64, accounts: &Vec<Account<'a>>, period: usize) -> WithdrawalStrategy {
        let total: f64 = accounts.iter().map(|a| a.balance()[period]).sum();
        let withdrawals_per_account = accounts.iter().map(|a| (a.balance()[period] / total) * monthly_withdrawal).collect();

        WithdrawalStrategy {
            withdrawals_per_account,
            period
        }
    }

    pub fn execute(self, accounts: &mut Vec<Account<'a>>) -> Result<(), f64> {
        // TODO ensure success in the constructor
        let mut shortfall = 0.0;
        for i in 0..accounts.len() {
            shortfall += accounts[i].attempt_withdrawal_with_shortfall(self.withdrawals_per_account[i], self.period);
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
    use super::*;
    use crate::assets::{AssetAllocation,AccountSettings};
    use crate::rates::Rate;

    #[test]
    pub fn withdrawalstrategy_constructor1account() {
        let dummy_allocation = AssetAllocation::new(vec![1.0]);
        let mut account = AccountSettings::new(1024.0, &dummy_allocation).create_account(1, vec![Rate::new(1.0, 1.0, 1.0)]);
        account.rebalance_and_invest_next_period(1);

        let strategy = WithdrawalStrategy::new(512.0, &vec![account], 1);
        assert_eq!(strategy.period, 1);
        assert_eq!(strategy.withdrawals_per_account, vec![512.0]);
    }

    #[test]
    pub fn withdrawalstrategy_constructor2accounts() {
        let dummy_allocation = AssetAllocation::new(vec![1.0]);
        let mut account1 = AccountSettings::new(1536.0, &dummy_allocation).create_account(1, vec![Rate::new(1.0, 1.0, 1.0)]);
        let mut account2 = AccountSettings::new(512.0, &dummy_allocation).create_account(1, vec![Rate::new(1.0, 1.0, 1.0)]);
        account1.rebalance_and_invest_next_period(1);
        account2.rebalance_and_invest_next_period(1);

        let strategy = WithdrawalStrategy::new(512.0, &vec![account1, account2], 1);
        assert_eq!(strategy.period, 1);
        assert_eq!(strategy.withdrawals_per_account, vec![384.0, 128.0]);
    }

    #[test]
    pub fn withdrawalstrategy_executesuccess() {
        let dummy_allocation = AssetAllocation::new(vec![1.0]);
        let mut account1 = AccountSettings::new(1536.0, &dummy_allocation).create_account(1, vec![Rate::new(1.0, 1.0, 1.0)]);
        let mut account2 = AccountSettings::new(512.0, &dummy_allocation).create_account(1, vec![Rate::new(1.0, 1.0, 1.0)]);
        account1.rebalance_and_invest_next_period(1);
        account2.rebalance_and_invest_next_period(1);

        let mut accounts = vec![account1, account2];

        let strategy = WithdrawalStrategy::new(512.0, &accounts, 1);
        strategy.execute(&mut accounts).expect("should have enough");
    }

    #[test]
    pub fn withdrawalstrategy_executefailure() {
        let dummy_allocation = AssetAllocation::new(vec![1.0]);
        let mut account1 = AccountSettings::new(1536.0, &dummy_allocation).create_account(1, vec![Rate::new(1.0, 1.0, 1.0)]);
        let mut account2 = AccountSettings::new(512.0, &dummy_allocation).create_account(1, vec![Rate::new(1.0, 1.0, 1.0)]);
        account1.rebalance_and_invest_next_period(1);
        account2.rebalance_and_invest_next_period(1);

        let mut accounts = vec![account1, account2];

        let strategy = WithdrawalStrategy::new(4096.0, &accounts, 1);
        assert_eq!(2048.0, strategy.execute(&mut accounts).expect_err("shouldn't have enough"));
    }
}