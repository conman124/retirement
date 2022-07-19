use crate::montecarlo::{Period, Lifespan};
use crate::rates::Rate;

pub trait IncomeProvider {
    fn calculate_income_for_period(&mut self, period: Period /*, tax */);
    fn get_net_income(&self) -> &Vec<f64>;
}

pub struct JobSettings {
    // name, social security, 401k, pension, raise, adjust for inflation
    starting_monthly_salary: f64
}

pub struct Job {
    starting_monthly_salary: f64,
    income: Vec<f64>,
    rates: Vec<Rate>
}

impl JobSettings {
    pub fn new(starting_monthly_salary: f64) -> JobSettings {
        JobSettings { starting_monthly_salary }
    }

    pub fn create_job(&self, lifespan: Lifespan, rates: Vec<Rate>) -> Job {
        let income = vec![0.0; lifespan.periods()];

        Job { starting_monthly_salary: self.starting_monthly_salary, income, rates }
    }
}

impl IncomeProvider for Job {
    fn calculate_income_for_period(&mut self, period: Period /*, tax */) {
        if period.get() == 0 {
            self.income[0] = self.starting_monthly_salary;
        } else if !period.is_new_year() {
            // TODO tax, social security, 401k deferrals
            self.income[period.get()] = self.income[period.get() - 1];
        } else {
            // You got a raise :)
            // TODO implement raise mechanics!
        }
    }

    fn get_net_income(&self) -> &Vec<f64> {
        todo!() 
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn calculateincome_noraise_notax() {
        let job_settings = JobSettings::new(1000.0);
        let lifespan = Lifespan::new(12);
        let rates = vec![Rate::new(1.0, 1.0, 1.0); 12];
        let mut job = job_settings.create_job(lifespan, rates);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period);
        }

        assert_eq!(job.income, vec![1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0]);
    }
}