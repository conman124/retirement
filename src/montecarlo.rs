use rand::prelude::*;
use rand::distributions::Uniform;
use crate::income::{JobSettings, Job, IncomeProvider};
use crate::rates::{generate_rates,Rate};
use crate::assets::{Account};
use crate::taxes::{TaxSettings, TaxCollector};
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

    pub fn iter(&self) -> impl Iterator<Item = Period> {
        LifespanIterator{current: 0, periods: self.periods}
    }

    pub fn periods(&self) -> usize {
        self.periods
    }

    pub fn contains(&self, period: Period) -> bool {
        period.get() < self.periods
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

    pub fn round_down_to_year(&self) -> Period {
        Period { period: self.period - (self.period % 12) }
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
    assets_adequate_periods: usize,
    lifespan: Lifespan,
    careerspan: Lifespan,
    retirement_accounts: Vec<Account<'a>>
}

fn calculate_periods(rng: &mut impl Rng) -> usize {
    rng.sample(Uniform::new(10, 50))
}

impl<'a> Run<'a> {
    pub fn execute<T: SeedableRng + Rng + Clone, U: TaxCollector>(seed: u64, all_rates: &[Rate], sublength: usize, job_settings: &JobSettings<'a>, career_periods: usize, tax_settings: TaxSettings) -> Run<'a> {
        let mut rng = T::seed_from_u64(seed);

        let periods = calculate_periods(&mut rng);
        let lifespan = Lifespan::new(periods);
        let careerspan = Lifespan::new(career_periods);
        let rates = generate_rates(T::seed_from_u64(rng.gen()), all_rates, sublength, periods);
        // TODO figure out a way to avoid cloning rates here
        let jobs = job_settings.create_job(lifespan, rates.clone());
        let tax = U::new(tax_settings, rates.clone(), lifespan);

        let mut run = Run {
            rates,
            assets_adequate_periods: 0,
            lifespan,
            careerspan,
            retirement_accounts: Vec::with_capacity(jobs.account_contributions().len())
        };

        run.populate(jobs, tax);

        run
    }

    fn populate<U: TaxCollector>(&mut self, mut job: Job<'a>, mut tax: U) {
        let mut life_iter = self.lifespan.iter();

        // Run until either we hit retirement or we die
        while let Some(period) = life_iter.next() {
            job.calculate_income_for_period(period, &mut tax);

            self.assets_adequate_periods += 1;
            
            // Check if we've hit retirement
            if period.get() == self.careerspan.periods() - 1 {
                break;
            }
        }

        let (pre_retirement_income, mut retirement_accounts) = job.retire();
        // TODO make WithdrawalStrategy smart enough to know about taxes
        let withdrawal_strategy = WithdrawalStrategyOrig::new();

        // TODO change withdrawal amount from pre_retirement_income

        for period in life_iter {
            match withdrawal_strategy.execute(pre_retirement_income, &mut retirement_accounts, period) {
                Ok(_) => {},
                Err(_) => { break; }
            }

            self.assets_adequate_periods += 1;
        }

        self.retirement_accounts = retirement_accounts;
    }
}

pub struct Simulation<'a> {
    runs: Vec<Run<'a>>
}

impl<'a> Simulation<'a> {
    pub fn new<T: SeedableRng + Rng + Clone, U: TaxCollector>(seed: u64, count: usize, all_rates: &[Rate], sublength: usize, job_settings: JobSettings<'a>, career_periods: usize, tax_settings: TaxSettings) -> Simulation<'a> {
        let runs: Vec<Run> = (0..count).map(|seed2| {
            // TODO this seed stuff is kinda awful
            let new_seed = (seed as usize * count) as u64 + (seed2 as u64);
            // TODO figure out a way to avoid cloning tax_settings here
            Run::execute::<T, U>(new_seed, all_rates, sublength, &job_settings, career_periods, tax_settings.clone())
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

/*    #[test]
    pub fn run_withadequate() {
        let rates = vec![Rate::new(1.25, 1.0, 1.0), Rate::new(1.5, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5)];
        let asset_allocation = AssetAllocation::new_linear_glide(1, 0.75, 2, 0.25);

        let account = AccountSettings::new(1024.0, &asset_allocation).create_account(Lifespan::new(3), rates.clone());
        let mut run = Run { rates, accounts: vec![account], assets_adequate_periods: 0, lifespan: Lifespan::new(3), careerspan: Lifespan::new(3) };
        run.populate(16.0);

        assert_eq!(run.accounts[0].balance(), &vec![1200.0, 1634.0, 1822.25]);
        assert_eq!(run.assets_adequate_periods, 3);
    }

    #[test]
    pub fn run_withinadequate() {
        let rates = vec![Rate::new(1.25, 1.0, 1.0), Rate::new(1.25, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5)];
        let asset_allocation = AssetAllocation::new_linear_glide(1, 0.75, 2, 0.25);

        let account = AccountSettings::new(1024.0, &asset_allocation).create_account(Lifespan::new(3), rates.clone());
        let mut run = Run { rates, accounts: vec![account], assets_adequate_periods: 0, lifespan: Lifespan::new(3), careerspan: Lifespan::new(3) };
        run.populate(512.0);

        assert_eq!(run.accounts[0].balance(), &vec![704.0, 368.0, 0.0]);
        assert_eq!(run.assets_adequate_periods, 2);
    }
*/
    

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