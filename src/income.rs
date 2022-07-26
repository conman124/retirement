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
    starting_gross_income: f64,
    fica: Fica,
    raise: RaiseSettings,
}

pub struct Job {
    starting_gross_income: f64,
    gross_income: Vec<f64>,
    net_income: Vec<f64>,
    fica: Fica,
    raise: RaiseSettings,
    rates: Vec<Rate>
}

impl JobSettings {
    pub fn new(starting_gross_income: f64, fica: Fica, raise: RaiseSettings) -> JobSettings {
        JobSettings { starting_gross_income, fica, raise }
    }

    pub fn create_job(&self, lifespan: Lifespan, rates: Vec<Rate>) -> Job {
        let gross_income = vec![0.0; lifespan.periods()];
        let net_income = vec![0.0; lifespan.periods()];
        assert_eq!(lifespan.periods(), rates.len());

        Job { starting_gross_income: self.starting_gross_income, gross_income, net_income, fica: self.fica, raise: self.raise, rates }
    }
}

impl IncomeProvider for Job {
    fn calculate_income_for_period(&mut self, period: Period /*, tax */) {
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

        let net = match self.fica {
            Fica::Participant { ss_rate } => { gross * (1.0 - ss_rate) },
            Fica::Exempt => { gross }
        };

        // TODO contribute to social security/pension/401k
        // TODO tax

        self.net_income[period.get()] = net;
    }

    fn get_net_income(&self) -> &Vec<f64> {
        &self.net_income
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

    #[test]
    pub fn calculateincome_fica_raiseinflation_notax() {
        let job_settings = JobSettings::new(1024.0, Fica::Participant { ss_rate: 0.0625 }, RaiseSettings { amount: 1.0625, adjust_for_inflation: true } );
        let lifespan = Lifespan::new(16);
        let rates = vec![Rate::new(1.0, 1.0, 1.002); 16];
        let mut job = job_settings.create_job(lifespan, rates);

        for period in lifespan.iter() {
            job.calculate_income_for_period(period);
        }

        assert_vecfloat_absolute(job.get_net_income().clone(), vec![960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 960.0, 1044.751083304, 1044.751083304, 1044.751083304, 1044.751083304]);
    }
}