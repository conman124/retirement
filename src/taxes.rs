use crate::montecarlo::Lifespan;
use crate::montecarlo::Period;
use crate::simplifying_assumption;

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
    pub fn taxes(&self) -> f64 { self.taxes } 
    pub fn leftover(&self) -> f64 { self.leftover } 
}

pub struct TaxBracket {
    pub floor: f64,
    pub rate: f64,
}

simplifying_assumption!("There are no tax credits.  This will lower the pre-retirement net \
    income, and depending on your settings might lower the retirement withdrawal amount.");
// TODO Add support for long term capital gains rates
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
        assert!(self.rates.brackets.len() > 0);

        let mut taxes = 0.0;

        // TODO inflate deduction
        money -= self.rates.deduction;

        // TODO inflate brackets
        for (bracket,next) in self.rates.brackets.iter().zip(self.rates.brackets[1..].iter()) {
            if money < bracket.floor {
                break;
            }

            let ceil = f64::min(money, next.floor);
            let in_bracket = ceil - bracket.floor;
            taxes += in_bracket * bracket.rate;
        }

        let last = self.rates.brackets.last().unwrap();
        if money > last.floor {
            let in_bracket = money - last.floor;
            taxes += in_bracket * last.rate;
        }

        taxes
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
                let taxes_paid = self.calculate_tax_amount(cumulative_annual_gross_income);
                self.gross_income[period.get()] += amt;
                let total_taxes = self.calculate_tax_amount(cumulative_annual_gross_income + amt);

                let taxes = total_taxes - taxes_paid;
                let leftover = amt - taxes;
                
                TaxResult{taxes, leftover}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use super::*;

    #[test]
    pub fn calculatetaxamount_belowdeduction() {
        let lifespan = Lifespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 } ];
        let rates = TaxRates { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(rates, lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(500.0), 0.0);
    }

    #[test]
    pub fn calculatetaxamount_onebracket() {
        let lifespan = Lifespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 } ];
        let rates = TaxRates { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(rates, lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(11000.0), 100.0);
    }

    #[test]
    pub fn calculatetaxamount_middlebracket() {
        let lifespan = Lifespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let rates = TaxRates { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(rates, lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(12000.0), 220.0);
    }

    #[test]
    pub fn calculatetaxamount_topbracket() {
        let lifespan = Lifespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let rates = TaxRates { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let tax = Tax::new(rates, lifespan);

        assert_float_absolute_eq!(tax.calculate_tax_amount(14000.0), 480.0);
    }

    #[test]
    pub fn collectincometaxes_nontaxable() {
        let lifespan = Lifespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let rates = TaxRates { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let mut tax = Tax::new(rates, lifespan);

        let ret = tax.collect_income_taxes(Money::NonTaxable(1000.0), Period::new(0));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 1000.0);
    }

    #[test]
    pub fn collectincometaxes_taxablemultiple() {
        let lifespan = Lifespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let rates = TaxRates { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let mut tax = Tax::new(rates, lifespan);

        let ret = tax.collect_income_taxes(Money::Taxable(6000.0), Period::new(0));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 6000.0);

        let ret = tax.collect_income_taxes(Money::Taxable(6000.0), Period::new(0));
        assert_float_absolute_eq!(ret.taxes(), 220.0);
        assert_float_absolute_eq!(ret.leftover(), 5780.0);
    }

    #[test]
    pub fn collectincometaxes_mixedtaxable() {
        let lifespan = Lifespan::new(12);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let rates = TaxRates { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let mut tax = Tax::new(rates, lifespan);

        let ret = tax.collect_income_taxes(Money::NonTaxable(15000.0), Period::new(0));

        assert_float_absolute_eq!(ret.taxes(), 0.0);
        assert_float_absolute_eq!(ret.leftover(), 15000.0);

        let ret = tax.collect_income_taxes(Money::Taxable(11000.0), Period::new(0));
        assert_float_absolute_eq!(ret.taxes(), 100.0);
        assert_float_absolute_eq!(ret.leftover(), 10900.0);
    }

    #[test]
    pub fn collectincometaxes_multiyear() {
        let lifespan = Lifespan::new(24);
        let brackets = vec![TaxBracket { floor: 0.0, rate: 0.1 }, TaxBracket { floor: 1000.0, rate: 0.12 }, TaxBracket { floor: 3000.0, rate: 0.14 } ];
        let rates = TaxRates { deduction: 10000.0, adjust_deduction_for_inflation: false, brackets, adjust_bracket_floors_for_inflation: false };
        let mut tax = Tax::new(rates, lifespan);

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