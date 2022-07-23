use crate::montecarlo::{Period, Lifespan};
use crate::rates::Rate;
use crate::simplifying_assumption;

pub trait IncomeProvider {
    fn calculate_income_for_period(&mut self, period: Period /*, tax */);
    fn get_net_income(&self) -> &Vec<f64>;
}

simplifying_assumption!("There is no cap on social security contributions/benefits. \
    This will particularly impact high earners and will cause the social security \
    deduction and benefit amount to be too high.");
#[derive(Copy,Clone)]
pub enum Fica {
    Participant{ss_rate: f64},
    Exempt
}

#[derive(Copy,Clone)]
pub struct RaiseSettings {
    pub amount: f64,
    pub adjust_for_inflation: bool
}

pub struct JobSettings {
    // name, 401k, pension
    starting_monthly_salary: f64,
    fica: Fica,
    raise: RaiseSettings,
}

pub struct Job {
    starting_monthly_salary: f64,
    income: Vec<f64>,
    fica: Fica,
    raise: RaiseSettings,
    rates: Vec<Rate>
}

impl JobSettings {
    pub fn new(starting_monthly_salary: f64, fica: Fica, raise: RaiseSettings) -> JobSettings {
        JobSettings { starting_monthly_salary, fica, raise }
    }

    pub fn create_job(&self, lifespan: Lifespan, rates: Vec<Rate>) -> Job {
        let income = vec![0.0; lifespan.periods()];
        assert_eq!(lifespan.periods(), rates.len());

        Job { starting_monthly_salary: self.starting_monthly_salary, income, fica: self.fica, raise: self.raise, rates }
    }
}

impl IncomeProvider for Job {
    fn calculate_income_for_period(&mut self, period: Period /*, tax */) {
        let salary = if period.get() == 0 {
            self.starting_monthly_salary
        } else if !period.is_new_year() {
            self.income[period.get() - 1]
        } else {
            simplifying_assumption!("You get a raise every 12 months after the beginning of the \
                simulation.  As long as the raise parameters are accurate, this should approximate \
                an annual raise.");

            let mut inflation_adjustment = 1.0;
            if self.raise.adjust_for_inflation {
                inflation_adjustment = self.rates[period.get()-12..period.get()].iter().map(|r| r.inflation()).product::<f64>();
            }

            self.income[period.get() - 1] * self.raise.amount * inflation_adjustment
        };

        let salary = match self.fica {
            Fica::Participant { ss_rate } => { salary * (1.0 - ss_rate) },
            Fica::Exempt => { salary }
        };

        // TODO contribute to social security/pension/401k
        // TODO tax

        self.income[period.get()] = salary;
    }

    fn get_net_income(&self) -> &Vec<f64> {
        &self.income
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::tests::assert_vecfloat_absolute;

    #[test]
    pub fn calculateincome_noraise_notax() {
        let job_settings = JobSettings::new(1000.0, Fica::Exempt, RaiseSettings {amount: 1.0, adjust_for_inflation: false} );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.0); 16];
        let mut job = job_settings.create_job(lifespan, rates);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period);
        }

        assert_eq!(job.get_net_income(), &vec![1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0]);
    }

    #[test]
    pub fn calculateincome_raise_notax() {
        let job_settings = JobSettings::new(1024.0, Fica::Exempt, RaiseSettings { amount: 1.0625, adjust_for_inflation: false } );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.0); 16];
        let mut job = job_settings.create_job(lifespan, rates);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period);
        }

        assert_eq!(job.get_net_income(), &vec![1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1088.0, 1088.0, 1088.0, 1088.0]);
    }

    #[test]
    pub fn calculateincome_raiseinflation_notax() {
        let job_settings = JobSettings::new(1024.0, Fica::Exempt, RaiseSettings { amount: 1.0625, adjust_for_inflation: true } );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.002); 16];
        let mut job = job_settings.create_job(lifespan, rates);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1024.0, 1114.401155525, 1114.401155525, 1114.401155525, 1114.401155525]);
    }
}