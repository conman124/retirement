use crate::rates::Rate;

pub struct AssetAllocation {
    stocks_glide: Vec<f64>,
}

impl AssetAllocation {
    pub fn new(stocks_glide: Vec<f64>) -> AssetAllocation {
        assert!(stocks_glide.len() >= 1);
        assert!(stocks_glide.iter().min_by(|x,y| x.partial_cmp(y).unwrap()).unwrap() >= &0.0);
        assert!(stocks_glide.iter().max_by(|x,y| x.partial_cmp(y).unwrap()).unwrap() <= &1.0);

        AssetAllocation{ stocks_glide }
    }

    pub fn new_linear_glide(periods_before: usize, start_stocks: f64, periods_glide: usize, end_stocks: f64) -> AssetAllocation {
        assert!(periods_before >= 1);
        assert!(periods_glide >= 1);
        assert!(start_stocks >= 0.0 && start_stocks <= 1.0);
        assert!(end_stocks >= 0.0 && end_stocks <= 1.0);

        let mut stocks_glide = vec![start_stocks; periods_before + periods_glide];
        
        for i in periods_before..periods_before+periods_glide {
            let frac = (i - periods_before + 1) as f64 / periods_glide as f64;
            stocks_glide[i] = frac * (end_stocks - start_stocks) + start_stocks;
        }

        AssetAllocation { stocks_glide }
    }

    pub fn stocks(&self, period: usize) -> f64 {
        if period < self.stocks_glide.len() {
            self.stocks_glide[period]
        } else {
            *self.stocks_glide.last().unwrap()
        }
    }

    pub fn bonds(&self, period: usize) -> f64 {
        1.0 - self.stocks(period)
    }
}

pub struct AccountSettings<'a> {
    starting_balance: f64,
    allocation: &'a AssetAllocation
}

pub struct Account<'a> {
    balance: Vec<f64>,
    allocation: &'a AssetAllocation,
    rates: Vec<Rate>
}

impl<'a> AccountSettings<'a> {
    pub fn new(starting_balance: f64, allocation: &'a AssetAllocation) -> AccountSettings<'a> {
        AccountSettings { starting_balance, allocation }
    }

    pub fn create_account(&self, periods: usize, rates: Vec<Rate>) -> Account<'a> {
        assert_eq!(rates.len(), periods);
        let mut balance = vec![0.0; periods+1];

        balance[0] = self.starting_balance;

        Account::<'a> {
            balance,
            allocation: self.allocation,
            rates: rates
        }
    }
}

impl<'a> Account<'a> {
    pub fn rebalance_and_invest_next_period(&mut self, period: usize) {
        assert!(period > 0);
        assert!(period < self.balance.len());

        let balance = self.balance[period-1];
        let stocks_new = balance * self.allocation.stocks(period-1) * self.rates[period-1].stocks();
        let bonds_new = balance * self.allocation.bonds(period-1) * self.rates[period-1].bonds();
        self.balance[period] = stocks_new + bonds_new;
    }
    
    pub fn withdraw_from_period(&mut self, amount: f64, period: usize) {
        assert!(period > 0);
        assert!(period < self.balance.len());
        assert!(amount <= self.balance[period]);
    
        self.balance[period] -= amount;
    }

    pub fn attempt_withdrawal_with_shortfall(&mut self, amount: f64, period: usize) -> f64 {
        let shortfall = amount - f64::min(amount, self.balance[period]);

        self.withdraw_from_period(f64::min(amount, self.balance[period]), period);

        shortfall
    }

    pub fn balance(&self) -> &Vec<f64> {
        &self.balance
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assetallocation_vec() {
        let assets = AssetAllocation::new(vec![1.0, 1.0, 1.0, 1.0, 0.5, 0.75]);

        assert_eq!(assets.stocks(0), 1.0);
        assert_eq!(assets.stocks(1), 1.0);
        assert_eq!(assets.stocks(2), 1.0);
        assert_eq!(assets.stocks(3), 1.0);
        assert_eq!(assets.stocks(4), 0.5);
        assert_eq!(assets.stocks(5), 0.75);
        assert_eq!(assets.stocks(6), 0.75);
        assert_eq!(assets.stocks(100), 0.75);

        assert_eq!(assets.bonds(0), 0.0);
        assert_eq!(assets.bonds(1), 0.0);
        assert_eq!(assets.bonds(2), 0.0);
        assert_eq!(assets.bonds(3), 0.0);
        assert_eq!(assets.bonds(4), 0.5);
        assert_eq!(assets.bonds(5), 0.25);
        assert_eq!(assets.bonds(6), 0.25);
        assert_eq!(assets.bonds(100), 0.25);
    }

    #[test]
    fn assetallocation_linearglide() {
        let assets = AssetAllocation::new_linear_glide(4, 1.0, 4, 0.5);

        assert_eq!(assets.stocks(0), 1.0);
        assert_eq!(assets.stocks(1), 1.0);
        assert_eq!(assets.stocks(2), 1.0);
        assert_eq!(assets.stocks(3), 1.0);
        assert_eq!(assets.stocks(4), 0.875);
        assert_eq!(assets.stocks(5), 0.75);
        assert_eq!(assets.stocks(6), 0.625);
        assert_eq!(assets.stocks(7), 0.5);
        assert_eq!(assets.stocks(8), 0.5);
        assert_eq!(assets.stocks(100), 0.5);

        assert_eq!(assets.bonds(0), 0.0);
        assert_eq!(assets.bonds(1), 0.0);
        assert_eq!(assets.bonds(2), 0.0);
        assert_eq!(assets.bonds(3), 0.0);
        assert_eq!(assets.bonds(4), 0.125);
        assert_eq!(assets.bonds(5), 0.25);
        assert_eq!(assets.bonds(6), 0.375);
        assert_eq!(assets.bonds(7), 0.5);
        assert_eq!(assets.bonds(8), 0.5);
        assert_eq!(assets.bonds(100), 0.5);
    }

    #[test]
    fn account_rebalanceandinvest() {
        // Use powers of two to make the floating point math work out roundly
        let allocation = AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25);
        let mut account = Account{ balance: vec![1024.0, 0.0], allocation: &allocation, rates: vec![Rate::new(2.0, 0.5, 1.0)] };
        
        account.rebalance_and_invest_next_period(1);
        assert_eq!(account.balance, vec![1024.0, 1664.0]);
    }

    #[test]
    fn account_withdrawall() {
        let allocation = AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25);
        let mut account = Account{ balance: vec![1024.0; 2], allocation: &allocation, rates: vec![] };

        account.withdraw_from_period(1024.0, 1);
        assert_eq!(account.balance, vec![1024.0, 0.0]);
    }


    #[test]
    fn account_withdrawsome() {
        let allocation = AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25);
        let mut account = Account{ balance: vec![1024.0; 2], allocation: &allocation, rates: vec![] };

        account.withdraw_from_period(512.0, 1);
        assert_eq!(account.balance, vec![1024.0, 512.0]);
    }


    #[test]
    #[should_panic]
    fn account_withdrawmore() {
        let allocation = AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25);
        let mut account = Account{ balance: vec![1024.0; 2], allocation: &allocation, rates: vec![] };

        account.withdraw_from_period(2048.0, 1);
    }

    #[test]
    fn account_attemptwithdrawall() {
        let allocation = AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25);
        let mut account = Account{ balance: vec![1024.0; 2], allocation: &allocation, rates: vec![] };

        let shortfall = account.attempt_withdrawal_with_shortfall(1024.0, 1);
        assert_eq!(account.balance, vec![1024.0, 0.0]);
        assert_eq!(shortfall, 0.0);
    }


    #[test]
    fn account_attemptwithdrawsome() {
        let allocation = AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25);
        let mut account = Account{ balance: vec![1024.0; 2], allocation: &allocation, rates: vec![] };

        let shortfall = account.attempt_withdrawal_with_shortfall(512.0, 1);
        assert_eq!(account.balance, vec![1024.0, 512.0]);
        assert_eq!(shortfall, 0.0);
    }


    #[test]
    fn account_attemptwithdrawmore() {
        let allocation = AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25);
        let mut account = Account{ balance: vec![1024.0; 2], allocation: &allocation, rates: vec![] };

        let shortfall = account.attempt_withdrawal_with_shortfall(2048.0, 1);
        assert_eq!(account.balance, vec![1024.0, 0.0]);
        assert_eq!(shortfall, 1024.0);
    }
}