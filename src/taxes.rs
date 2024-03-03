use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::montecarlo::Timespan;
use crate::montecarlo::Period;
use crate::rates::Rate;
use crate::simplifying_assumption;

#[cfg(test)]
use mockall::automock;

// TODO Add taxable with basis here
pub enum Money {
    Taxable(f64),
    NonTaxable(f64),
}

pub struct TaxResult {
    taxes: f64,
    leftover: f64,
}

impl TaxResult {
    #[cfg(test)]
    pub fn new(taxes: f64, leftover: f64) -> TaxResult { TaxResult{ taxes, leftover } }

    pub fn taxes(&self) -> f64 { self.taxes } 
    pub fn leftover(&self) -> f64 { self.leftover } 
}

#[derive(Clone,Copy,Debug)]
#[wasm_bindgen]
pub struct TaxBracket {
    pub floor: f64,
    pub rate: f64,
}

simplifying_assumption!("There are no tax credits.  This will lower the pre-retirement net \
    income, and depending on your settings might lower the retirement withdrawal amount.");
// TODO Add support for long term capital gains rates
#[derive(Clone,Debug)]
#[wasm_bindgen]
pub struct TaxSettings {
    brackets: Vec<TaxBracket>,
    adjust_bracket_floors_for_inflation: bool,
    deduction: f64,
    adjust_deduction_for_inflation: bool,
}

impl TaxSettings {
    pub fn new(brackets: Vec<TaxBracket>, adjust_bracket_floors_for_inflation: bool, deduction: f64, adjust_deduction_for_inflation: bool ) -> TaxSettings {
        TaxSettings { brackets, adjust_bracket_floors_for_inflation, deduction, adjust_deduction_for_inflation }
    }
}

#[wasm_bindgen]
impl TaxSettings {
    #[wasm_bindgen(constructor)]
    pub fn new_from_js(bracket_floors: Vec<f64>, bracket_rates: Vec<f64>, adjust_bracket_floors_for_inflation: bool, deduction: f64, adjust_deduction_for_inflation: bool) -> TaxSettings {
        let brackets = bracket_floors.into_iter().zip(bracket_rates)
            .map(|(floor, rate)| { TaxBracket{floor, rate} })
            .collect();

        Self::new(brackets, adjust_bracket_floors_for_inflation, deduction, adjust_deduction_for_inflation)
    }
}

#[cfg_attr(test, automock)]
pub trait TaxCollector {
    fn new(settings: TaxSettings, rates: Rc<Vec<Rate>>, lifespan: Timespan) -> Self;
    fn collect_income_taxes(&mut self, money: Money, period: Period) -> TaxResult;
}

#[derive(Debug)]
#[wasm_bindgen]
pub struct Tax {
    settings: TaxSettings,
    rates: Rc<Vec<Rate>>,
    gross_income: Vec<f64>
}

impl Tax {
    fn calculate_tax_amount(&self, mut money: f64, period: Period) -> f64 {
        assert!(self.settings.brackets.len() > 0);

        let mut taxes = 0.0;

        let mut deduction_inflation = 1.0;
        if self.settings.adjust_deduction_for_inflation {
            let new_year = period.round_down_to_year();
            if new_year.get() > 0 {
                deduction_inflation = self.rates[new_year.get()-12..new_year.get()].iter().map(|r| r.inflation()).product::<f64>();
            }
        }
        money -= self.settings.deduction * deduction_inflation;

        let mut bracket_inflation = 1.0;
        if self.settings.adjust_bracket_floors_for_inflation {
            let new_year = period.round_down_to_year();
            if new_year.get() > 0 {
                bracket_inflation = self.rates[new_year.get()-12..new_year.get()].iter().map(|r| r.inflation()).product::<f64>();
            }
        }
        for (bracket,next) in self.settings.brackets.iter().zip(self.settings.brackets[1..].iter()) {
            if money < bracket.floor * bracket_inflation {
                break;
            }

            let ceil = f64::min(money, next.floor * bracket_inflation);
            let in_bracket = ceil - bracket.floor * bracket_inflation;
            taxes += in_bracket * bracket.rate;
        }

        let last = self.settings.brackets.last().unwrap();
        if money > last.floor * bracket_inflation {
            let in_bracket = money - last.floor * bracket_inflation;
            taxes += in_bracket * last.rate;
        }

        taxes
    }

    pub fn new(settings: TaxSettings, rates: Rc<Vec<Rate>>, lifespan: Timespan) -> Tax {
        assert_eq!(rates.len(), lifespan.periods());

        Tax{ settings, rates, gross_income: vec![0.0; lifespan.periods()] }
    }
}

impl Tax {
    pub fn collect_income_taxes(&mut self, money: Money, period: Period) -> TaxResult {
        match money {
            Money::NonTaxable(amt) => {
                TaxResult{taxes: 0.0, leftover: amt}
            },
            Money::Taxable(amt) => {
                let year_begin = period.round_down_to_year();
                let cumulative_annual_gross_income: f64 = self.gross_income[year_begin.get()..=period.get()].iter().sum();
                let taxes_paid = self.calculate_tax_amount(cumulative_annual_gross_income, period);
                self.gross_income[period.get()] += amt;
                let total_taxes = self.calculate_tax_amount(cumulative_annual_gross_income + amt, period);

                let taxes = total_taxes - taxes_paid;
                let leftover = amt - taxes;
                
                TaxResult{taxes, leftover}
            }
        }
    }
}

impl TaxCollector for Tax {
    fn new(settings: TaxSettings, rates: Rc<Vec<Rate>>, lifespan: Timespan) -> Tax {
        Self::new(settings, rates, lifespan)
    }

    fn collect_income_taxes(&mut self, money: Money, period: Period) -> TaxResult {
        self.collect_income_taxes(money, period)
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use super::*;

    #[test]
    pub fn calculatetaxamount_belowdeduction() {
        let lifespan = Timespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.0); 12]), lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(500.0, Period::new(0)), 0.0);
    }

    #[test]
    pub fn calculatetaxamount_onebracket() {
        let lifespan = Timespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.0); 12]), lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(11000.0, Period::new(0)), 100.0);
    }

    #[test]
    pub fn calculatetaxamount_middlebracket() {
        let lifespan = Timespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.0); 12]), lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(12000.0, Period::new(0)), 220.0);
    }

    #[test]
    pub fn calculatetaxamount_topbracket() {
        let lifespan = Timespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.0); 12]), lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(14000.0, Period::new(0)), 480.0);
    }

    #[test]
    pub fn calculatetaxamount_inflatededuction() {
        let lifespan = Timespan::new(24);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: true, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.002); 24]), lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(12000.0, Period::new(0)), 220.0);
        assert_float_absolute_eq!(tax.calculate_tax_amount(15000.0, Period::new(0)), 620.0);

        assert_float_absolute_eq!(tax.calculate_tax_amount(12000.0, Period::new(12)), 190.881078);
        assert_float_absolute_eq!(tax.calculate_tax_amount(15000.0, Period::new(12)), 586.027924876);
    }

    #[test]
    pub fn calculatetaxamount_inflatebrackets() {
        let lifespan = Timespan::new(24);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: true };
        let tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.002); 24]), lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(12000.0, Period::new(0)), 220.0);
        assert_float_absolute_eq!(tax.calculate_tax_amount(15000.0, Period::new(0)), 620.0);

        assert_float_absolute_eq!(tax.calculate_tax_amount(12000.0, Period::new(12)), 219.5146846);
        assert_float_absolute_eq!(tax.calculate_tax_amount(15000.0, Period::new(12)), 618.0587386);
    }

    #[test]
    pub fn calculatetaxamount_inflateboth() {
        let lifespan = Timespan::new(24);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: true, brackets, adjust_bracket_floors_for_inflation: true };
        let tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.002); 24]), lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(12000.0, Period::new(0)), 220.0);
        assert_float_absolute_eq!(tax.calculate_tax_amount(15000.0, Period::new(0)), 620.0);

        assert_float_absolute_eq!(tax.calculate_tax_amount(12000.0, Period::new(12)), 190.3957631);
        assert_float_absolute_eq!(tax.calculate_tax_amount(15000.0, Period::new(12)), 584.0866634);
    }

    #[test]
    pub fn collectincometaxes_nontaxable() {
        let lifespan = Timespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let mut tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.0); 12]), lifespan);

        let ret = tax.collect_income_taxes(Money::NonTaxable(1000.0), Period::new(0));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);
    }

    #[test]
    pub fn collectincometaxes_taxablemultiple() {
        let lifespan = Timespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let mut tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.0); 12]), lifespan);

        let ret = tax.collect_income_taxes(Money::Taxable(6000.0), Period::new(0));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 6000.0);

        let ret = tax.collect_income_taxes(Money::Taxable(6000.0), Period::new(0));
        assert_float_absolute_eq!(ret.taxes(), 220.0);
        assert_float_absolute_eq!(ret.leftover(), 5780.0);
    }

    #[test]
    pub fn collectincometaxes_mixedtaxable() {
        let lifespan = Timespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let mut tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.0); 12]), lifespan);

        let ret = tax.collect_income_taxes(Money::NonTaxable(15000.0), Period::new(0));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 15000.0);

        let ret = tax.collect_income_taxes(Money::Taxable(11000.0), Period::new(0));
        assert_float_absolute_eq!(ret.taxes(), 100.0);
        assert_float_absolute_eq!(ret.leftover(), 10900.0);
    }

    #[test]
    pub fn collectincometaxes_multiyear() {
        let lifespan = Timespan::new(24);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let settings = TaxSettings { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let mut tax = Tax::new(settings, Rc::new(vec![Rate::new(1.0, 1.0, 1.0); 24]), lifespan);

        // Year 1, month 1
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(0));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 2
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(1));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 3
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(2));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 4
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(3));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 5
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(4));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 6
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(5));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 7
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(6));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 8
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(7));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 9
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(8));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 10
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(9));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 1, month 11
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(10));

        assert_float_absolute_eq!(ret.taxes(), 100.0);
        assert_float_absolute_eq!(ret.leftover(), 900.0);

        // Year 1, month 12
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(11));

        assert_float_absolute_eq!(ret.taxes(), 120.0);
        assert_float_absolute_eq!(ret.leftover(), 880.0);

        // Begin year 2

        // Year 2, month 1
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(12));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 2
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(13));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 3
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(14));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 4
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(15));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 5
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(16));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 6
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(17));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 7
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(18));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 8
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(19));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 9
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(20));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 10
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(21));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);

        // Year 2, month 11
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(22));

        assert_float_absolute_eq!(ret.taxes(), 100.0);
        assert_float_absolute_eq!(ret.leftover(), 900.0);

        // Year 2, month 12
        let ret = tax.collect_income_taxes(Money::Taxable(1000.0), Period::new(23));

        assert_float_absolute_eq!(ret.taxes(), 120.0);
        assert_float_absolute_eq!(ret.leftover(), 880.0);
    }
}