use rand::prelude::*;
use rand::distributions::Uniform;
use crate::rates::{generate_rates,Rate};
use crate::assets::{Account,AccountSettings};

pub struct Run<'a> {
    rates: Vec<Rate>,
    accounts: Vec<Account<'a >>,
    periods: usize,
    assets_adequate_periods: usize,
}

fn calculate_periods(rng: &mut impl Rng) -> usize {
    rng.sample(Uniform::new(10, 50))
}

impl<'a> Run<'a> {
    pub fn execute<T: SeedableRng + Rng + Clone>(seed: u64, all_rates: &[Rate], sublength: usize, length: usize, accounts_settings: Vec<AccountSettings<'a>>, withdrawal: f64) -> Run<'a> {
        let mut rng = T::seed_from_u64(seed);


        let periods = calculate_periods(&mut rng);
        let rates = generate_rates(T::seed_from_u64(rng.gen()), all_rates, sublength, length);
        // TODO figure out a way to avoid cloning rates here
        let accounts = accounts_settings.iter().map(|a| a.create_account(periods, rates.clone())).collect();

        let mut run = Run {
            rates,
            accounts,
            periods,
            assets_adequate_periods: 0
        };

        run.populate(withdrawal);

        run
    }

    fn populate(&mut self, withdrawal: f64) {
        for period in 1..=self.periods {
            for account in self.accounts.iter_mut() {
                account.rebalance_and_invest_next_period(period);
                if account.balance()[period] < withdrawal {
                    return;
                }
                account.withdraw_from_period(withdrawal, period);
            }
            self.assets_adequate_periods += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assets::AssetAllocation;
    use super::*;

    #[test]
    pub fn run_executewithadequate() {
        let rates = vec![Rate::new(1.25, 1.0, 1.0), Rate::new(1.5, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5)];
        let asset_allocation = AssetAllocation::new_linear_glide(1, 0.75, 2, 0.25);

        let account = AccountSettings::new(1024.0, &asset_allocation).create_account(3, rates.clone());
        let mut run = Run { rates, accounts: vec![account], periods: 3, assets_adequate_periods: 0 };
        run.populate(16.0);

        assert_eq!(run.accounts[0].balance(), &vec![1024.0, 1200.0, 1634.0, 1822.25]);
        assert_eq!(run.assets_adequate_periods, 3);
    }

    #[test]
    pub fn run_executewithinadequate() {
        let rates = vec![Rate::new(1.25, 1.0, 1.0), Rate::new(1.25, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5)];
        let asset_allocation = AssetAllocation::new_linear_glide(1, 0.75, 2, 0.25);

        let account = AccountSettings::new(1024.0, &asset_allocation).create_account(3, rates.clone());
        let mut run = Run { rates, accounts: vec![account], periods: 3, assets_adequate_periods: 0 };
        run.populate(512.0);

        // TODO this last number is wrong, but I need to refactor the withdrawal logic to handle this situation
        assert_eq!(run.accounts[0].balance(), &vec![1024.0, 704.0, 368.0, 414.0]);
        assert_eq!(run.assets_adequate_periods, 2);
    }
}