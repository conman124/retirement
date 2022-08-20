use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::{rates::Rate, montecarlo::{Period, Lifespan}};

#[derive(Debug)]
#[wasm_bindgen]
pub struct AssetAllocation {
    stocks_glide: Vec<f64>,
}

#[wasm_bindgen]
impl AssetAllocation {
    #[wasm_bindgen(constructor)]
    pub fn new(stocks_glide: Vec<f64>) -> AssetAllocation {
        assert!(stocks_glide.len() >= 1);
        assert!(stocks_glide.iter().min_by(|x,y| x.partial_cmp(y).unwrap()).unwrap() >= &0.0);
        assert!(stocks_glide.iter().max_by(|x,y| x.partial_cmp(y).unwrap()).unwrap() <= &1.0);

        AssetAllocation{ stocks_glide }
    }

    #[wasm_bindgen]
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

    #[wasm_bindgen]
    pub fn stocks(&self, period: Period) -> f64 {
        if period.get() < self.stocks_glide.len() {
            self.stocks_glide[period.get()]
        } else {
            *self.stocks_glide.last().unwrap()
        }
    }

    #[wasm_bindgen]
    pub fn bonds(&self, period: Period) -> f64 {
        1.0 - self.stocks(period)
    }
}

#[wasm_bindgen]
pub struct AccountSettings {
    starting_balance: f64,
    allocation: Rc<AssetAllocation>
}

#[derive(Debug)]
pub struct Account {
    starting_balance: f64,
    balance: Vec<f64>,
    allocation: Rc<AssetAllocation>,
    rates: Rc<Vec<Rate>>
}

#[wasm_bindgen]
impl AccountSettings {
    #[wasm_bindgen(constructor)]
    pub fn new_from_js(starting_balance: f64, allocation: AssetAllocation) -> AccountSettings {
        Self::new(starting_balance, Rc::new(allocation))
    }
}

impl AccountSettings {
    pub fn new(starting_balance: f64, allocation: Rc<AssetAllocation>) -> AccountSettings {
        AccountSettings { starting_balance, allocation }
    }

    pub fn create_account(&self, lifespan: Lifespan, rates: Rc<Vec<Rate>>) -> Account {
        assert_eq!(rates.len(), lifespan.periods());
        let balance = vec![0.0; lifespan.periods()];

        Account {
            starting_balance: self.starting_balance,
            balance,
            allocation: Rc::clone(&self.allocation),
            rates: rates
        }
    }
}

impl Account {
    pub fn rebalance_and_invest_next_period(&mut self, period: Period) {
        assert!(period.get() < self.balance.len());
        assert_eq!(self.balance[period.get()], 0.0);

        let balance = if period.get() > 0 { self.balance[(period-1).get()] } else { self.starting_balance };
        let stocks_new = balance * self.allocation.stocks(period) * self.rates[(period).get()].stocks();
        let bonds_new = balance * self.allocation.bonds(period) * self.rates[(period).get()].bonds();
        self.balance[period.get()] = stocks_new + bonds_new;
    }
    
    pub fn withdraw_from_period(&mut self, amount: f64, period: Period) {
        assert!(period.get() < self.balance.len());
        assert!(amount <= self.balance[period.get()]);
    
        self.balance[period.get()] -= amount;
    }

    pub fn attempt_withdrawal_with_shortfall(&mut self, amount: f64, period: Period) -> f64 {
        let shortfall = amount - f64::min(amount, self.balance[period.get()]);

        self.withdraw_from_period(f64::min(amount, self.balance[period.get()]), period);

        shortfall
    }

    pub fn deposit(&mut self, amount: f64, period: Period) {
        self.balance[period.get()] += amount;
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

        assert_eq!(assets.stocks(Period::new(0)), 1.0);
        assert_eq!(assets.stocks(Period::new(1)), 1.0);
        assert_eq!(assets.stocks(Period::new(2)), 1.0);
        assert_eq!(assets.stocks(Period::new(3)), 1.0);
        assert_eq!(assets.stocks(Period::new(4)), 0.5);
        assert_eq!(assets.stocks(Period::new(5)), 0.75);
        assert_eq!(assets.stocks(Period::new(6)), 0.75);
        assert_eq!(assets.stocks(Period::new(100)), 0.75);

        assert_eq!(assets.bonds(Period::new(0)), 0.0);
        assert_eq!(assets.bonds(Period::new(1)), 0.0);
        assert_eq!(assets.bonds(Period::new(2)), 0.0);
        assert_eq!(assets.bonds(Period::new(3)), 0.0);
        assert_eq!(assets.bonds(Period::new(4)), 0.5);
        assert_eq!(assets.bonds(Period::new(5)), 0.25);
        assert_eq!(assets.bonds(Period::new(6)), 0.25);
        assert_eq!(assets.bonds(Period::new(100)), 0.25);
    }

    #[test]
    fn assetallocation_linearglide() {
        let assets = AssetAllocation::new_linear_glide(4, 1.0, 4, 0.5);

        assert_eq!(assets.stocks(Period::new(0)), 1.0);
        assert_eq!(assets.stocks(Period::new(1)), 1.0);
        assert_eq!(assets.stocks(Period::new(2)), 1.0);
        assert_eq!(assets.stocks(Period::new(3)), 1.0);
        assert_eq!(assets.stocks(Period::new(4)), 0.875);
        assert_eq!(assets.stocks(Period::new(5)), 0.75);
        assert_eq!(assets.stocks(Period::new(6)), 0.625);
        assert_eq!(assets.stocks(Period::new(7)), 0.5);
        assert_eq!(assets.stocks(Period::new(8)), 0.5);
        assert_eq!(assets.stocks(Period::new(100)), 0.5);

        assert_eq!(assets.bonds(Period::new(0)), 0.0);
        assert_eq!(assets.bonds(Period::new(1)), 0.0);
        assert_eq!(assets.bonds(Period::new(2)), 0.0);
        assert_eq!(assets.bonds(Period::new(3)), 0.0);
        assert_eq!(assets.bonds(Period::new(4)), 0.125);
        assert_eq!(assets.bonds(Period::new(5)), 0.25);
        assert_eq!(assets.bonds(Period::new(6)), 0.375);
        assert_eq!(assets.bonds(Period::new(7)), 0.5);
        assert_eq!(assets.bonds(Period::new(8)), 0.5);
        assert_eq!(assets.bonds(Period::new(100)), 0.5);
    }

    #[test]
    fn account_rebalanceandinvest_period0() {
        // Use powers of two to make the floating point math work out roundly
        let allocation = Rc::new(AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25));
        let mut account = Account{ starting_balance: 1024.0, balance: vec![0.0], allocation: allocation, rates: Rc::new(vec![Rate::new(2.0, 0.5, 1.0)]) };
        
        account.rebalance_and_invest_next_period(Period::new(0));
        assert_eq!(account.balance, vec![1664.0]);
    }

    #[test]
    fn account_rebalanceandinvest_period1() {
        // Use powers of two to make the floating point math work out roundly
        let allocation = Rc::new(AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25));
        let mut account = Account{ starting_balance: 1024.0, balance: vec![1664.0, 0.0], allocation: allocation, rates: Rc::new(vec![Rate::new(2.0, 0.5, 1.0), Rate::new(2.0, 0.5, 1.0)]) };
        
        account.rebalance_and_invest_next_period(Period::new(1));
        assert_eq!(account.balance, vec![1664.0, 2704.0]);
    }

    #[test]
    fn account_withdrawall() {
        let allocation = Rc::new(AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25));
        let mut account = Account{ starting_balance: 1024.0, balance: vec![1024.0; 2], allocation: allocation, rates: Default::default() };

        account.withdraw_from_period(1024.0, Period::new(1));
        assert_eq!(account.balance, vec![1024.0, 0.0]);
    }


    #[test]
    fn account_withdrawsome() {
        let allocation = Rc::new(AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25));
        let mut account = Account{ starting_balance: 1024.0, balance: vec![1024.0; 2], allocation: allocation, rates: Default::default() };

        account.withdraw_from_period(512.0, Period::new(1));
        assert_eq!(account.balance, vec![1024.0, 512.0]);
    }


    #[test]
    #[should_panic]
    fn account_withdrawmore() {
        let allocation = Rc::new(AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25));
        let mut account = Account{ starting_balance: 1024.0, balance: vec![1024.0; 2], allocation: allocation, rates: Default::default() };

        account.withdraw_from_period(2048.0, Period::new(1));
    }

    #[test]
    fn account_attemptwithdrawall() {
        let allocation = Rc::new(AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25));
        let mut account = Account{ starting_balance: 1024.0, balance: vec![1024.0; 2], allocation: allocation, rates: Default::default() };

        let shortfall = account.attempt_withdrawal_with_shortfall(1024.0, Period::new(1));
        assert_eq!(account.balance, vec![1024.0, 0.0]);
        assert_eq!(shortfall, 0.0);
    }


    #[test]
    fn account_attemptwithdrawsome() {
        let allocation = Rc::new(AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25));
        let mut account = Account{ starting_balance: 1024.0, balance: vec![1024.0; 2], allocation: allocation, rates: Default::default() };

        let shortfall = account.attempt_withdrawal_with_shortfall(512.0, Period::new(1));
        assert_eq!(account.balance, vec![1024.0, 512.0]);
        assert_eq!(shortfall, 0.0);
    }


    #[test]
    fn account_attemptwithdrawmore() {
        let allocation = Rc::new(AssetAllocation::new_linear_glide(4, 0.75, 2, 0.25));
        let mut account = Account{ starting_balance: 1024.0, balance: vec![1024.0; 2], allocation: allocation, rates: Default::default() };

        let shortfall = account.attempt_withdrawal_with_shortfall(2048.0, Period::new(1));
        assert_eq!(account.balance, vec![1024.0, 0.0]);
        assert_eq!(shortfall, 1024.0);
    }
}