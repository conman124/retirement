use rand::prelude::*;
use rand::distributions::Uniform;
use crate::rates::{generate_rates,Rate};
use crate::assets::{Account,AccountSettings};
use crate::util::Ratio;
use crate::withdrawal::{WithdrawalStrategyOrig,WithdrawalStrategy};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Lifespan {
    periods: usize
}

#[derive(Debug)]
pub struct LifespanIterator {
    current: usize,
    periods: usize
}

#[derive(Copy, Clone, Debug)]
pub struct Period {
    period: usize
}

impl Lifespan {
    pub fn new(periods: usize) -> Lifespan {
        Lifespan{ periods }
    }

    pub fn iter(&self) -> LifespanIterator {
        LifespanIterator{current: 0, periods: self.periods}
    }

    pub fn periods(&self) -> usize {
        self.periods
    }
}

impl Iterator for LifespanIterator {
    type Item = Period;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.periods {
            self.current += 1;
            Some(Period{ period: self.current-1 })
        } else {
            None
        }
    }
}

impl Period {
    #[cfg(test)]
    pub fn new(period: usize) -> Period {
        Period { period }
    }

    pub fn get(&self) -> usize {
        self.period
    }

    pub fn is_new_year(&self) -> bool {
        // TODO fix this logic
        self.period % 12 == 0
    }
}

impl std::ops::Sub<usize> for Period {
    type Output = Period;

    fn sub(self, rhs: usize) -> Self::Output {
        Period { period: self.period - rhs }
    }
}

impl std::ops::Add<usize> for Period {
    type Output = Period;

    fn add(self, rhs: usize) -> Self::Output {
        Period { period: self.period + rhs }
    }
}

pub struct Run<'a> {
    rates: Vec<Rate>,
    accounts: Vec<Account<'a >>,
    assets_adequate_periods: usize,
    lifespan: Lifespan
}

fn calculate_periods(rng: &mut impl Rng) -> usize {
    rng.sample(Uniform::new(10, 50))
}

impl<'a> Run<'a> {
    pub fn execute<T: SeedableRng + Rng + Clone>(seed: u64, all_rates: &[Rate], sublength: usize, accounts_settings: &Vec<AccountSettings<'a>>, withdrawal: f64) -> Run<'a> {
        let mut rng = T::seed_from_u64(seed);


        let periods = calculate_periods(&mut rng);
        let lifespan = Lifespan::new(periods);
        let rates = generate_rates(T::seed_from_u64(rng.gen()), all_rates, sublength, periods);
        // TODO figure out a way to avoid cloning rates here
        let accounts = accounts_settings.iter().map(|a| a.create_account(lifespan, rates.clone())).collect();

        let mut run = Run {
            rates,
            accounts,
            assets_adequate_periods: 0,
            lifespan
        };

        run.populate(withdrawal);

        run
    }

    fn populate(&mut self, withdrawal: f64) {
        for period in self.lifespan.iter() {
            for account in self.accounts.iter_mut() {
                account.rebalance_and_invest_next_period(period);
            }

            let strategy = WithdrawalStrategyOrig::new();
            match strategy.execute(withdrawal, &mut self.accounts, period) {
                Err(_) => { return; }
                _ => {}
            }

            self.assets_adequate_periods += 1;
        }
    }
}

pub struct Simulation<'a> {
    runs: Vec<Run<'a>>
}

impl<'a> Simulation<'a> {
    pub fn new<T: SeedableRng + Rng + Clone>(seed: u64, count: usize, all_rates: &[Rate], sublength: usize, accounts_settings: &Vec<AccountSettings<'a>>, withdrawal: f64) -> Simulation<'a> {
        let runs: Vec<Run<'a>> = (0..count).map(|seed2| {
            // TODO this seed stuff is kinda awful
            let new_seed = (seed as usize * count) as u64 + (seed2 as u64);
            Run::execute::<T>(new_seed, all_rates, sublength, accounts_settings, withdrawal)
        }).collect();

        Simulation { runs }
    }

    pub fn success_rate(&self) -> Ratio<usize> {
        Ratio {
            num: self.runs.iter().filter(|a| a.assets_adequate_periods >= a.lifespan.periods()).count(),
            denum: self.runs.len()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assets::AssetAllocation;
    use super::*;

    #[test]
    pub fn run_withadequate() {
        let rates = vec![Rate::new(1.25, 1.0, 1.0), Rate::new(1.5, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5)];
        let asset_allocation = AssetAllocation::new_linear_glide(1, 0.75, 2, 0.25);

        let account = AccountSettings::new(1024.0, &asset_allocation).create_account(Lifespan::new(3), rates.clone());
        let mut run = Run { rates, accounts: vec![account], assets_adequate_periods: 0, lifespan: Lifespan::new(3) };
        run.populate(16.0);

        assert_eq!(run.accounts[0].balance(), &vec![1200.0, 1634.0, 1822.25]);
        assert_eq!(run.assets_adequate_periods, 3);
    }

    #[test]
    pub fn run_withinadequate() {
        let rates = vec![Rate::new(1.25, 1.0, 1.0), Rate::new(1.25, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5)];
        let asset_allocation = AssetAllocation::new_linear_glide(1, 0.75, 2, 0.25);

        let account = AccountSettings::new(1024.0, &asset_allocation).create_account(Lifespan::new(3), rates.clone());
        let mut run = Run { rates, accounts: vec![account], assets_adequate_periods: 0, lifespan: Lifespan::new(3) };
        run.populate(512.0);

        assert_eq!(run.accounts[0].balance(), &vec![704.0, 368.0, 0.0]);
        assert_eq!(run.assets_adequate_periods, 2);
    }

    #[test]
    pub fn period_sub() {
        let period = Period::new(1);
        let new_period = period - 1;

        assert_eq!(new_period.get(), 0);
    }

    #[test]
    pub fn period_add() {
        let period = Period::new(1);
        let new_period = period + 1;

        assert_eq!(new_period.get(), 2);
    }

    #[test]
    pub fn lifespan_iter() {
        let lifespan = Lifespan::new(10);
        let mut iter = lifespan.iter();
        for i in 0..10 {
            assert_eq!(iter.next().unwrap().get(), i);
        }
        assert!(iter.next().is_none());
    }

    #[test]
    pub fn lifespan_max() {
        let lifespan1 = Lifespan::new(10);
        let lifespan2 = Lifespan::new(20);

        assert_eq!(std::cmp::max(lifespan1, lifespan2).periods, lifespan2.periods);
    }
}