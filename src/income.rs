use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::assets::{AccountSettings, Account};
use crate::montecarlo::{Period, Lifespan};
use crate::rates::Rate;
use crate::simplifying_assumption;
use crate::taxes::{TaxCollector, Money};

pub trait IncomeProvider {
    fn calculate_income_for_period(&mut self, period: Period, tax: &mut impl TaxCollector);
    fn get_net_income(&self) -> &Vec<f64>;
    fn retire(self) -> (f64, Vec<Account>);
    fn account_contributions(&self) -> &[AccountContribution];
}

simplifying_assumption!("There is no cap on social security contributions/benefits. \
    This will particularly impact high earners and will cause the social security \
    deduction and benefit amount to be too high.");
#[derive(Copy,Clone)]
pub enum Fica {
    Participant{ss_rate: f64},
    Exempt
}

#[wasm_bindgen]
pub struct FicaJS {
    fica: Fica
}

#[wasm_bindgen]
impl FicaJS {
    #[wasm_bindgen]
    pub fn new_participant(ss_rate: f64) -> FicaJS {
        FicaJS{ fica: Fica::Participant { ss_rate }}
    }

    #[wasm_bindgen]
    pub fn new_exempt() -> FicaJS {
        FicaJS{ fica: Fica::Exempt }
    }
}

#[derive(Copy,Clone)]
#[wasm_bindgen]
pub struct RaiseSettings {
    pub amount: f64,
    pub adjust_for_inflation: bool
}

#[derive(Copy,Clone,PartialEq,Eq,Debug)]
#[wasm_bindgen]
pub enum AccountContributionSource {
    Employee,
    Employer
}

#[derive(Copy,Clone,PartialEq,Eq)]
#[wasm_bindgen]
pub enum AccountContributionTaxability {
    PreTax,
    PostTax
}

#[wasm_bindgen]
pub struct AccountContributionSettings {
    account: AccountSettings,
    contribution_pct: f64,
    contribution_source: AccountContributionSource,
    tax: AccountContributionTaxability
}

pub struct AccountContribution {
    account: Account,
    contribution_pct: f64,
    contribution_source: AccountContributionSource,
    tax: AccountContributionTaxability
}

#[wasm_bindgen]
pub struct JobSettings {
    // name, 401k, pension
    starting_gross_income: f64,
    fica: Fica,
    raise: RaiseSettings,
    account_contribution_settings: Vec<AccountContributionSettings>
}

pub struct Job {
    starting_gross_income: f64,
    gross_income: Vec<f64>,
    net_income: Vec<f64>,
    fica: Fica,
    raise: RaiseSettings,
    rates: Rc<Vec<Rate>>,
    account_contributions: Vec<AccountContribution>
}

#[wasm_bindgen]
impl AccountContributionSettings {
    pub fn new(account: AccountSettings, contribution_pct: f64, contribution_source: AccountContributionSource, tax: AccountContributionTaxability) -> AccountContributionSettings {
        AccountContributionSettings { account, contribution_pct, contribution_source, tax }
    }
}

impl AccountContributionSettings {
    pub fn create_account_contribution(&self, lifespan: Lifespan, rates: Rc<Vec<Rate>>) -> AccountContribution {
        AccountContribution {
            account: self.account.create_account(lifespan, rates),
            contribution_pct: self.contribution_pct,
            contribution_source: self.contribution_source,
            tax: self.tax
        }
    }
}

#[wasm_bindgen]
pub struct AccountContributionSettingsVec {
    vec: Vec<AccountContributionSettings>
}

#[wasm_bindgen]
impl AccountContributionSettingsVec {
    #[wasm_bindgen]
    pub fn add(&mut self, accoun_contribution_settings: AccountContributionSettings) {
        self.vec.push(accoun_contribution_settings);
    }
}

#[wasm_bindgen]
impl JobSettings {
    #[wasm_bindgen]
    pub fn new_from_js(starting_gross_income: f64, fica: FicaJS, raise: RaiseSettings, account_contribution_settings: AccountContributionSettingsVec) -> JobSettings {
        Self::new(starting_gross_income, fica.fica, raise, account_contribution_settings.vec)
    }
}

impl JobSettings {
    pub fn new(starting_gross_income: f64, fica: Fica, raise: RaiseSettings, account_contribution_settings: Vec<AccountContributionSettings>) -> JobSettings {
        JobSettings { starting_gross_income, fica, raise, account_contribution_settings }
    }

    pub fn create_job(&self, lifespan: Lifespan, careerspan: Lifespan, rates: Rc<Vec<Rate>>) -> Job {
        assert_eq!(lifespan.periods(), rates.len());
        let gross_income = vec![0.0; careerspan.periods()];
        let net_income = vec![0.0; careerspan.periods()];
        let account_contributions = self.account_contribution_settings.iter().map(|settings| settings.create_account_contribution(lifespan, rates.clone()) ).collect();

        Job { starting_gross_income: self.starting_gross_income, gross_income, net_income, fica: self.fica, raise: self.raise, rates, account_contributions }
    }
}

impl IncomeProvider for Job {
    fn calculate_income_for_period(&mut self, period: Period, tax: &mut impl TaxCollector) {
        assert!(period.get() < self.net_income.len());

        // Rebalance + invest for this period.  This has to be done before we deposit anything
        for account in self.account_contributions.iter_mut() {
            account.account.rebalance_and_invest_next_period(period);
        }

        let gross = if period.get() == 0 {
            self.starting_gross_income
        } else if !period.is_new_year() {
            self.gross_income[period.get() - 1]
        } else {
            simplifying_assumption!("You get a raise every 12 months after the beginning of the \
                simulation.  As long as the raise parameters are accurate, this should approximate \
                an annual raise.");

            let mut inflation_adjustment = 1.0;
            if self.raise.adjust_for_inflation {
                inflation_adjustment = self.rates[period.get()-12..period.get()].iter().map(|r| r.inflation()).product::<f64>();
            }

            self.gross_income[period.get() - 1] * self.raise.amount * inflation_adjustment
        };

        self.gross_income[period.get()] = gross;

        let fica_deduction = match self.fica {
            Fica::Participant { ss_rate } => { gross * ss_rate },
            Fica::Exempt => { 0.0 }
        };

        let mut pretax_contributions = 0.0;
        for account in &mut self.account_contributions {
            if account.tax == AccountContributionTaxability::PreTax {
                if account.contribution_source == AccountContributionSource::Employee {
                    pretax_contributions += gross * account.contribution_pct
                }
                account.account.deposit(gross * account.contribution_pct, period);
            }
        }

        let taxable = gross - pretax_contributions;

        let net = tax.collect_income_taxes(Money::Taxable(taxable), period).leftover();

        // TODO contribute to social security/pension

        let mut posttax_contributions = 0.0;
        for account in &mut self.account_contributions {
            if account.tax == AccountContributionTaxability::PostTax {
                assert_eq!(account.contribution_source, AccountContributionSource::Employee);

                posttax_contributions += gross * account.contribution_pct;
                account.account.deposit(gross * account.contribution_pct, period);
            }
        }

        self.net_income[period.get()] = net - fica_deduction - posttax_contributions;
    }

    fn get_net_income(&self) -> &Vec<f64> {
        &self.net_income
    }

    fn retire(self) -> (f64, Vec<Account>) {
        let months = std::cmp::min(12, self.net_income.len());

        (
            self.net_income[self.net_income.len()-months..].iter().sum::<f64>() / (months as f64),
            self.account_contributions.into_iter().map(|a| {a.account}).collect()
        )
    }

    fn account_contributions(&self) -> &[AccountContribution] {
        &self.account_contributions
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;

    use super::*;
    use crate::assets::AssetAllocation;
    use crate::util::tests::assert_vecfloat_absolute;
    use crate::taxes::{MockTaxCollector,TaxResult};

    fn get_tax_mock(rate: f64) -> impl TaxCollector {
        let mut mock = MockTaxCollector::default();
        mock.expect_collect_income_taxes().returning(move |money, _period| {
            match money {
                Money::Taxable(amt) => TaxResult::new(rate * amt, (1.0 - rate) * amt),
                Money::NonTaxable(amt) => TaxResult::new(0.0, amt)
            }
        });
        mock
    }
    
    #[test]
    pub fn calculateincome_noraise_notax() {
        let job_settings = JobSettings::new(1000.0, Fica::Exempt, RaiseSettings {amount: 1.0, adjust_for_inflation: false}, vec![] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.0); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.0);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        assert_eq!(job.get_net_income(), &vec![1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0]);
    }

    #[test]
    pub fn calculateincome_raise_notax() {
        let job_settings = JobSettings::new(1024.0, Fica::Exempt, RaiseSettings { amount: 1.0625, adjust_for_inflation: false }, vec![] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.0); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.0);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        assert_eq!(job.get_net_income(), &vec![1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1088.0, 1088.0, 1088.0, 1088.0]);
    }

    #[test]
    pub fn calculateincome_raiseinflation_notax() {
        let job_settings = JobSettings::new(1024.0, Fica::Exempt, RaiseSettings { amount: 1.0625, adjust_for_inflation: true }, vec![] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.002); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.0);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1114.401155525, 1114.401155525, 1114.401155525, 1114.401155525]);
    }

    #[test]
    pub fn calculateincome_fica_raiseinflation_notax() {
        let job_settings = JobSettings::new(1024.0, Fica::Participant { ss_rate: 0.0625 }, RaiseSettings { amount: 1.0625, adjust_for_inflation: true }, vec![] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.002); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.0);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 1044.751083304, 1044.751083304, 1044.751083304, 1044.751083304]);
    }

    #[test]
    pub fn calculateincome_noraise_10tax() {
        let job_settings = JobSettings::new(1000.0, Fica::Exempt, RaiseSettings {amount: 1.0, adjust_for_inflation: false}, vec![] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.0); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.1);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0, 900.0]);
    }

    #[test]
    pub fn calculateincome_fica_raise_10tax() {
        let job_settings = JobSettings::new(1000.0, Fica::Participant { ss_rate: 0.0625 }, RaiseSettings {amount: 1.0625, adjust_for_inflation: true}, vec![] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.002); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.1);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 911.436491945, 911.436491945, 911.436491945, 911.436491945]);
    }

    #[test]
    pub fn calculateincome_fica_raise_10tax_employeepretax401k() {
        let asset_allocation = Rc::new(AssetAllocation::new_linear_glide(1, 0.5, 1, 0.5));
        let account = AccountSettings::new(0.0, asset_allocation);
        let account_contributions = AccountContributionSettings { account, contribution_pct: 0.08, contribution_source: AccountContributionSource::Employee, tax: AccountContributionTaxability::PreTax };
        let job_settings = JobSettings::new(1000.0, Fica::Participant { ss_rate: 0.0625 }, RaiseSettings {amount: 1.0625, adjust_for_inflation: true}, vec![account_contributions] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.006, 1.0, 1.002); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.1);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![765.5, 765.5, 765.5, 765.5, 765.5, 765.5, 765.5, 765.5, 765.5, 765.5, 765.5, 765.5, 833.080160697, 833.080160697, 833.080160697, 833.080160697]);
        assert_vecfloat_absolute(job.account_contributions[0].account.balance().to_vec(), vec![80.0, 160.24, 240.72072, 321.44288216, 402.40721080648, 483.614432438899, 565.065275736216, 646.760471563425, 728.700752978115, 810.886855237049, 893.31951580276, 975.999474350168, 1065.99006304858, 1156.25062351308, 1246.78196565898, 1337.58490183132]);
    }

    #[test]
    pub fn calculateincome_fica_raise_10tax_employerpretax401k() {
        let asset_allocation = Rc::new(AssetAllocation::new_linear_glide(1, 0.5, 1, 0.5));
        let account = AccountSettings::new(0.0, asset_allocation);
        let account_contributions = AccountContributionSettings { account, contribution_pct: 0.08, contribution_source: AccountContributionSource::Employer, tax: AccountContributionTaxability::PreTax };
        let job_settings = JobSettings::new(1000.0, Fica::Participant { ss_rate: 0.0625 }, RaiseSettings {amount: 1.0625, adjust_for_inflation: true}, vec![account_contributions] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.006, 1.0, 1.002); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.1);

        for period in lifespan.iter() {

            job.calculate_income_for_period(period, &mut tax);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 837.50, 911.436491945, 911.436491945, 911.436491945, 911.436491945]);
        assert_vecfloat_absolute(job.account_contributions[0].account.balance().to_vec(), vec![80.0, 160.24, 240.72072, 321.44288216, 402.40721080648, 483.614432438899, 565.065275736216, 646.760471563425, 728.700752978115, 810.886855237049, 893.31951580276, 975.999474350168, 1065.99006304858, 1156.25062351308, 1246.78196565898, 1337.58490183132]);
    }

    #[test]
    pub fn calculateincome_fica_raise_10tax_employeeposttax401k() {
        let asset_allocation = Rc::new(AssetAllocation::new_linear_glide(1, 0.5, 1, 0.5));
        let account = AccountSettings::new(0.0, asset_allocation);
        let account_contributions = AccountContributionSettings { account, contribution_pct: 0.08, contribution_source: AccountContributionSource::Employee, tax: AccountContributionTaxability::PostTax };
        let job_settings = JobSettings::new(1000.0, Fica::Participant { ss_rate: 0.0625 }, RaiseSettings {amount: 1.0625, adjust_for_inflation: true}, vec![account_contributions] );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.006, 1.0, 1.002); 16];
        let mut job = job_settings.create_job(lifespan, lifespan, Rc::new(rates));
        let mut tax = get_tax_mock(0.1);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![757.5, 757.5, 757.5, 757.5, 757.5, 757.5, 757.5, 757.5, 757.5, 757.5, 757.5, 757.5, 824.37390167, 824.37390167, 824.37390167, 824.37390167, ]);
        assert_vecfloat_absolute(job.account_contributions[0].account.balance().to_vec(), vec![80.0, 160.24, 240.72072, 321.44288216, 402.40721080648, 483.614432438899, 565.065275736216, 646.760471563425, 728.700752978115, 810.886855237049, 893.31951580276, 975.999474350168, 1065.99006304858, 1156.25062351308, 1246.78196565898, 1337.58490183132]);
    }

    #[test]
    pub fn retire_fica_raise_10tax_employerpretax401k() {
        let asset_allocation = Rc::new(AssetAllocation::new_linear_glide(1, 0.5, 1, 0.5));
        let account = AccountSettings::new(0.0, asset_allocation);
        let account_contributions = AccountContributionSettings { account, contribution_pct: 0.08, contribution_source: AccountContributionSource::Employer, tax: AccountContributionTaxability::PreTax };
        let job_settings = JobSettings::new(1000.0, Fica::Participant { ss_rate: 0.0625 }, RaiseSettings {amount: 1.0625, adjust_for_inflation: true}, vec![account_contributions] );
        let lifespan = Lifespan::new(20);
        let careerspan = Lifespan::new(16);
        let rates = vec![Rate::new(1.006, 1.0, 1.002); 20];
        let mut job = job_settings.create_job(lifespan, careerspan, Rc::new(rates));
        let mut tax = get_tax_mock(0.1);

        for period in careerspan.iter() {
            job.calculate_income_for_period(period, &mut tax);
        }

        let (monthly_net_salary, accounts) = job.retire();

        assert_float_absolute_eq!(monthly_net_salary, 862.145497315);
        assert_eq!(accounts[0].balance().len(), 20);
    }

}