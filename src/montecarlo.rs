use std::cell::Ref;
use std::rc::Rc;

use rand::prelude::*;
use wasm_bindgen::prelude::*;
use js_sys::Float64Array;

use crate::income::{JobSettings, IncomeProvider};
use crate::person::PersonSettings;
use crate::rates::{Rate, RatesSource, RatesSourceHolder};
use crate::assets::{Account};
use crate::taxes::{TaxSettings, TaxCollector, Tax};
use crate::util::Ratio;
use crate::withdrawal::{WithdrawalStrategyOrig,WithdrawalStrategy};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
#[wasm_bindgen]
pub struct Lifespan {
    periods: usize
}

#[derive(Debug)]
pub struct LifespanIterator {
    current: usize,
    periods: usize
}

#[derive(Copy, Clone, Debug)]
#[wasm_bindgen]
pub struct Period {
    period: usize
}

impl Lifespan {
    pub fn iter(&self) -> impl Iterator<Item = Period> {
        LifespanIterator{current: 0, periods: self.periods}
    }
}

#[wasm_bindgen]
impl Lifespan {
    #[wasm_bindgen]
    pub fn new(periods: usize) -> Lifespan {
        Lifespan{ periods }
    }

    #[wasm_bindgen]
    pub fn periods(&self) -> usize {
        self.periods
    }

    #[wasm_bindgen]
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

#[derive(Debug)]
pub struct Run {
    rates: Rc<Vec<Rate>>,
    assets_adequate_periods: usize,
    lifespan: Lifespan,
    careerspan: Lifespan,
    retirement_accounts: Vec<Account>
}

impl Run {
    pub fn execute<T: SeedableRng + Rng + Clone + std::fmt::Debug, U: TaxCollector + std::fmt::Debug>(seed: u64, rates_source: Ref<RatesSource>, sublength: usize, job_settings: &JobSettings, person_settings: &PersonSettings, career_periods: usize, tax_settings: TaxSettings) -> Run {
        let mut rng = T::seed_from_u64(seed);

        let person = person_settings.create_person(&mut rng);
        let lifespan = person.lifespan();
        let careerspan = Lifespan::new(career_periods);
        let rates = Rc::new(rates_source.generate_rates(T::seed_from_u64(rng.gen()), sublength, lifespan.periods()));
        let jobs = job_settings.create_job(lifespan, careerspan, Rc::clone(&rates));
        let tax = U::new(tax_settings, Rc::clone(&rates), lifespan);

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

    fn populate<T: IncomeProvider, U: TaxCollector>(&mut self, mut job: T, mut tax: U) {
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

        let (pre_retirement_monthly_income, mut retirement_accounts) = job.retire();
        // TODO make WithdrawalStrategy smart enough to know about taxes
        let withdrawal_strategy = WithdrawalStrategyOrig::new();

        // TODO change withdrawal amount from pre_retirement_income

        for period in life_iter {
            for account in &mut retirement_accounts {
                account.rebalance_and_invest_next_period(period);
            }

            match withdrawal_strategy.execute(pre_retirement_monthly_income, &mut retirement_accounts, period) {
                Ok(_) => {},
                Err(_) => { break; }
            }

            self.assets_adequate_periods += 1;
        }

        self.retirement_accounts = retirement_accounts;
    }
}

#[wasm_bindgen]
pub struct Simulation {
    runs: Vec<Run>
}

#[wasm_bindgen]
impl Simulation {
    #[wasm_bindgen(constructor)]
    pub fn new_default(seed: u64, count: usize, rates_source: RatesSourceHolder, sublength: usize, job_settings: JobSettings, person_settings: PersonSettings, career_periods: usize, tax_settings: TaxSettings) -> Simulation {
        Self::new::<rand_pcg::Pcg64Mcg, Tax>(seed, count, rates_source, sublength, job_settings, person_settings, career_periods, tax_settings)
    }

    #[wasm_bindgen]
    pub fn success_rate(&self) -> Ratio {
        Ratio {
            num: self.runs.iter().filter(|a| a.assets_adequate_periods >= a.lifespan.periods()).count(),
            denom: self.runs.len()
        }
    }

    #[wasm_bindgen]
    pub fn assets_adequate_periods_for_run(&self, run: usize) -> usize {
       self.runs[run].assets_adequate_periods
    }

    #[wasm_bindgen]
    pub fn lifespan_for_run(&self, run: usize) -> Lifespan {
       self.runs[run].lifespan
    }

    #[wasm_bindgen]
    pub fn get_account_balance_for_run(&self, run: usize, acct: usize) -> Float64Array {
       unsafe {
           Float64Array::view(&self.runs[run].retirement_accounts[acct].balance())
       }
    }
}

impl Simulation {
    pub fn new<T: SeedableRng + Rng + Clone + std::fmt::Debug, U: TaxCollector + std::fmt::Debug>(seed: u64, count: usize, rates_source: RatesSourceHolder, sublength: usize, job_settings: JobSettings, person_settings: PersonSettings, career_periods: usize, tax_settings: TaxSettings) -> Simulation {
        let runs: Vec<Run> = (0..count).map(|seed2| {
            // TODO this seed stuff is kinda awful
            let new_seed = (seed as usize * count) as u64 + (seed2 as u64);
            // TODO figure out a way to avoid cloning tax_settings here
            Run::execute::<T, U>(new_seed, rates_source.get_rates_source(), sublength, &job_settings, &person_settings, career_periods, tax_settings.clone())
        }).collect();

        Simulation { runs }
    }

}

#[cfg(test)]
mod tests {
    use crate::assets::{AssetAllocation,AccountSettings};
    use crate::income::{Fica,RaiseSettings,AccountContributionSettings,AccountContributionSource,AccountContributionTaxability};
    use crate::rates::RatesSourceHolder;
    use crate::taxes::{MockTaxCollector,TaxResult,Money, TaxBracket};
    use crate::util::get_thread_local_rc;
    use super::*;

    include!(concat!(env!("OUT_DIR"), "/test_rates.rs"));
    include!(concat!(env!("OUT_DIR"), "/test_death.rs"));

    fn get_null_tax() -> impl TaxCollector {
        let mut null_tax = MockTaxCollector::default();
        null_tax.expect_collect_income_taxes().returning(move |money, _period| {
            match money {
                Money::Taxable(amt) => TaxResult::new(0.0, amt),
                Money::NonTaxable(amt) => TaxResult::new(0.0, amt)
            }
        });
        null_tax
    }

    #[test]
    pub fn run_withadequate() {
        let rates = Rc::new(vec![Rate::new(1.25, 1.0, 1.0), Rate::new(1.5, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5), Rate::new(1.25, 1.0, 1.0), Rate::new(1.5, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5)]);
        let asset_allocation = Rc::new(AssetAllocation::new_linear_glide(1, 0.75, 2, 0.25));

        let account = AccountContributionSettings::new(AccountSettings::new(2048.0, asset_allocation), 0.25, AccountContributionSource::Employee, AccountContributionTaxability::PreTax);
        let mut run = Run { rates: Rc::clone(&rates), assets_adequate_periods: 0, lifespan: Lifespan::new(6), careerspan: Lifespan::new(3), retirement_accounts: vec![] };
        let job = JobSettings::new(2048.0, Fica::Exempt, RaiseSettings {amount: 1.0, adjust_for_inflation: false}, vec![account] ).create_job(Lifespan::new(6), Lifespan::new(3), rates);
        let null_tax = get_null_tax();
        
        run.populate(job, null_tax);

        assert_eq!(run.retirement_accounts[0].balance(), &vec![2944.0, 4560.0, 5642.0, 4458.625, 4315.9453125, 3319.4384765625]);
        assert_eq!(run.assets_adequate_periods, 6);
    }

    #[test]
    pub fn run_withinadequate() {
        let rates = Rc::new(vec![Rate::new(1.25, 1.0, 1.0), Rate::new(1.5, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5), Rate::new(1.25, 1.0, 1.0), Rate::new(1.5, 1.25, 1.0), Rate::new(0.75, 1.25, 1.5)]);
        let asset_allocation = Rc::new(AssetAllocation::new_linear_glide(1, 0.75, 2, 0.25));

        let account = AccountContributionSettings::new(AccountSettings::new(1024.0, asset_allocation), 0.125, AccountContributionSource::Employee, AccountContributionTaxability::PreTax);
        let mut run = Run { rates: Rc::clone(&rates), assets_adequate_periods: 0, lifespan: Lifespan::new(6), careerspan: Lifespan::new(3), retirement_accounts: vec![] };
        let job = JobSettings::new(2048.0, Fica::Exempt, RaiseSettings {amount: 1.0, adjust_for_inflation: false}, vec![account] ).create_job(Lifespan::new(6), Lifespan::new(3), rates);
        let null_tax = get_null_tax();
        
        run.populate(job, null_tax);

        assert_eq!(run.retirement_accounts[0].balance(), &vec![1472.0, 2280.0, 2821.0, 1205.3125, 0.0, 0.0]);
        assert_eq!(run.assets_adequate_periods, 4);
    }
    
    #[test]
    pub fn simulation_regression1() {
        let asset_allocation = Rc::new(AssetAllocation::new_linear_glide(1, 0.83, (110 - 27) * 12, 0.0));
        let account_settings = AccountSettings::new(50000.0, asset_allocation);
        let account_contribution_settings = AccountContributionSettings::new(account_settings, 0.15, AccountContributionSource::Employee, AccountContributionTaxability::PostTax);
        let job_settings = JobSettings::new(129000.0 / 12.0, Fica::Exempt, RaiseSettings { amount: 1.05, adjust_for_inflation: true }, vec![account_contribution_settings]);
        let death_rates = get_thread_local_rc(&TEST_DEATH_BUILTIN).clone();

        // When I first created this regression, there was a bug that was skipping the first death rate in the csv as a "header".
        // To keep the original results, slice of that number
        let death_rates = Rc::from(&death_rates[1..]);

        let person_settings = PersonSettings::new(27, 0, death_rates);
        let brackets = vec![(0.0, 0.1), (10275.0, 0.12), (41775.0, 0.22), (89075.0, 0.24), (170050.0, 0.32), (215950.0, 0.35), (539900.0, 0.37)].iter().map(|b| { TaxBracket { floor: b.0, rate: b.1 } }).collect();
        let tax_settings = TaxSettings::new(brackets, true, 12950.0, true );
        let simulation = Simulation::new::<rand_pcg::Pcg64Mcg, Tax>(1337, 100, RatesSourceHolder::new_from_custom(Vec::from(TEST_RATES_BUILTIN)), 12, job_settings, person_settings, (65 - 27) * 12, tax_settings);

        assert_eq!(simulation.success_rate().num, 48);
        assert_eq!(simulation.success_rate().denom, 100);

        assert_eq!(simulation.runs[0].lifespan.periods(), 767);
        assert_eq!(simulation.runs[0].assets_adequate_periods, 622);
        assert_eq!(simulation.runs[0].retirement_accounts[0].balance()[..12], [51248.7292286, 52380.39457286909, 56871.42575448158, 59525.492082032, 61196.13885752394, 61785.05465636826, 65607.00783072409, 67606.33964011342, 67969.00130185773, 71380.60268634508, 73701.0843924699, 75908.8568924566]);
        assert_eq!(simulation.runs[0].retirement_accounts[0].balance()[(simulation.runs[0].lifespan.periods()-12)..], [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

        assert_eq!(simulation.runs[1].lifespan.periods(), 691);
        assert_eq!(simulation.runs[1].assets_adequate_periods, 691);
        assert_eq!(simulation.runs[1].retirement_accounts[0].balance()[..12], [54073.2778065, 56965.67060992199, 59760.360763633245, 63107.109164191774, 68958.71226715308, 67842.21923729929, 74852.24766690681, 72512.28377092176, 74959.52661903139, 76316.76162827399, 77291.47993597148, 80256.03843738187]);
        assert_eq!(simulation.runs[1].retirement_accounts[0].balance()[(simulation.runs[1].lifespan.periods()-12)..], [744821.5730118523, 703018.223741604, 663064.1007979073, 611859.271316495, 562289.5586005333, 518130.04116344935, 466121.4553477689, 417050.95723389054, 367524.3422774736, 321111.5655106036, 271257.2857022287, 219811.49559669665]);

        
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