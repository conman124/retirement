use crate::montecarlo::Lifespan;
use crate::montecarlo::Period;
use crate::simplifying_assumption;

todo!(); // Add taxable with basis here
pub enum Money {
    Taxable(f64),
    NonTaxable(f64),
}

pub struct TaxResult {
    taxes: f64,
    leftover: f64,
}

impl TaxResult {
    pub fn taxes(&self) -> f64 { self.taxes } 
    pub fn leftover(&self) -> f64 { self.leftover } 
}

pub struct TaxBracket {
    pub floor: f64,
    pub rate: f64,
}

simplifying_assumption!("There are no tax credits.  This will lower the pre-retirement net \
    income, and depending on your settings might lower the retirement withdrawal amount.");
todo!(); // Add support for long term capital gains rates
pub struct TaxRates {
    pub brackets: Vec<TaxBracket>,
    pub adjust_bracket_floors_for_inflation: bool,
    pub deduction: f64,
    pub adjust_deduction_for_inflation: bool,
}

pub trait TaxCollector {
    fn collect_income_taxes(&mut self, money: Money, period: Period) -> TaxResult;
}

pub struct Tax {
    rates: TaxRates,
    gross_income: Vec<f64>
}

impl Tax {
    pub fn new(rates: TaxRates, lifespan: Lifespan) -> Tax {
        Tax{ rates, gross_income: vec![0.0; lifespan.periods()] }
    }

    fn calculate_tax_amount(&self, mut money: f64) -> f64 {
        let mut taxes = 0.0;

        // TODO inflate deduction
        money -= self.rates.deduction;

        // TODO inflate brackets
        for (bracket,next) in self.rates.brackets.iter().zip(self.rates.brackets[1..].iter()) {
            if money < bracket.floor {
                break;
            }
        }

        0.0
    }
}


impl TaxCollector for Tax {
    fn collect_income_taxes(&mut self, money: Money, period: Period) -> TaxResult {
        match money {
            Money::NonTaxable(amt) => {
                TaxResult{taxes: 0.0, leftover: amt}
            },
            Money::Taxable(amt) => {
                let year_begin = period.round_down_to_year();
                let cumulative_annual_gross_income: f64 = self.gross_income[year_begin.get()..=period.get()].iter().sum(); 



                todo!()
            }
        }
    }
}