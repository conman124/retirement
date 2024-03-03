use rand::prelude::*;
use wasm_bindgen::prelude::*;
use crate::montecarlo::Timespan;
use crate::util::get_thread_local_rc;
use std::rc::Rc;


#[wasm_bindgen]
pub enum Gender {
    Male,
    Female
}

#[derive(Debug)]
#[wasm_bindgen]
pub struct PersonSettings {
    name: String,
    age_years: usize,
    age_months: usize,
    annual_death_rates: Rc<[f64]>
}

#[derive(Debug)]
pub struct Person<'a> {
    name: &'a str,
    lifespan: Timespan
}

impl PersonSettings {
    pub fn new(name: String, age_years: usize, age_months: usize, annual_death_rates: Rc<[f64]>) -> PersonSettings {
        PersonSettings { name, age_years, age_months, annual_death_rates }
    }

    pub fn create_person<R: Rng>(&self, rng: &mut R) -> Person {

        let lifespan = life_expectancy::calculate_periods(rng, &self.annual_death_rates[self.age_years..], self.age_months);

        Person {
            name: &self.name,
            lifespan: Timespan::new(lifespan)
        }
    }
}

#[wasm_bindgen]
impl PersonSettings {
    #[wasm_bindgen]
    pub fn new_with_default_death_rates(name: String, age_years: usize, age_months: usize, gender: Gender) -> PersonSettings {
        let rates = match gender {
            Gender::Male => &life_expectancy::ANNUAL_DEATH_MALE_BUILTIN,
            Gender::Female => &life_expectancy::ANNUAL_DEATH_FEMALE_BUILTIN,
        };

        let rates = get_thread_local_rc(rates);

        PersonSettings::new(name, age_years, age_months, rates)
    }

    #[wasm_bindgen]
    pub fn new_with_custom_death_rates(name: String, age_years: usize, age_months: usize, annual_death_rates: &[f64]) -> PersonSettings {
        PersonSettings::new(name, age_years, age_months, Rc::from(annual_death_rates))
    }
}

impl Person<'_> {
    pub fn lifespan(&self) -> Timespan {
        self.lifespan
    }
}

mod life_expectancy {
    use std::cmp;    
    use rand::prelude::*;

    include!(concat!(env!("OUT_DIR"), "/death_female.rs"));
    include!(concat!(env!("OUT_DIR"), "/death_male.rs"));

    fn convert_annual_death_to_monthly_life(rates: &[f64], offset: usize) -> Vec<f64> {
        assert!(rates.len() >= 1);
        assert!(offset < 12);

        let mut ret = Vec::with_capacity(rates.len() * 12 - offset);

        for (pos, prob) in rates.iter().enumerate() {
            let count = if pos == 0 { 12 - offset } else { 12 };
            for _i in 0..count {
                ret.push( (1.0 - prob).powf(1.0/12.0) );
            }
        }

        assert!(ret.len() == rates.len() * 12 - offset );

        ret
    }

    pub fn calculate_periods<R: Rng>(rng: &mut R, annual_death: &[f64], offset: usize) -> usize {
        let life_rates = convert_annual_death_to_monthly_life(annual_death, offset);

        let mut i = 0;
        loop {
            let lived = rng.gen_bool(life_rates[cmp::min(i, life_rates.len() - 1)]);
            if !lived {
                return i;
            }
            i += 1;
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::util::tests::assert_vecfloat_absolute;

        #[test]
        fn convertannual_offset0() {
            let ret = convert_annual_death_to_monthly_life(&vec![0.1, 0.2, 0.3], 0);

            assert_vecfloat_absolute(ret, vec![
                0.9912584, 0.9912584, 0.9912584, 0.9912584, 0.9912584, 0.9912584, 0.9912584, 0.9912584, 0.9912584, 0.9912584, 0.9912584, 0.9912584,
                0.9815765, 0.9815765, 0.9815765, 0.9815765, 0.9815765, 0.9815765, 0.9815765, 0.9815765, 0.9815765, 0.9815765, 0.9815765, 0.9815765,
                0.9707145, 0.9707145, 0.9707145, 0.9707145, 0.9707145, 0.9707145, 0.9707145, 0.9707145, 0.9707145, 0.9707145, 0.9707145, 0.9707145
            ]);
        }

        #[test]
        fn convertannual_offset4() {
            let ret = convert_annual_death_to_monthly_life(&vec![0.05, 0.15, 0.25], 4);

            assert_vecfloat_absolute(ret, vec![
                0.9957347, 0.9957347, 0.9957347, 0.9957347, 0.9957347, 0.9957347, 0.9957347, 0.9957347, 
                0.9865481, 0.9865481, 0.9865481, 0.9865481, 0.9865481, 0.9865481, 0.9865481, 0.9865481, 0.9865481, 0.9865481, 0.9865481, 0.9865481, 
                0.9763116, 0.9763116, 0.9763116, 0.9763116, 0.9763116, 0.9763116, 0.9763116, 0.9763116, 0.9763116, 0.9763116, 0.9763116, 0.9763116
            ]);
        }

        // TODO come up with a good way to actually unit test these functions which depend on RNG
        #[test]
        fn calculateperiods_regression1() {
            let mut my_rng = rand_pcg::Pcg64Mcg::new(1337);
            let annual_death = vec![0.1, 0.15, 0.2, 0.25, 0.30, 0.35];
            let ret = calculate_periods(&mut my_rng, &annual_death, 0);

            // doesn't extend past the end of annual_death vec
            assert_eq!(ret, 60);
        }

        #[test]
        fn calculateperiods_regression2() {
            let mut my_rng = rand_pcg::Pcg64Mcg::new(17);
            let annual_death = vec![0.2];
            let ret = calculate_periods(&mut my_rng, &annual_death, 0);

            // extends past the end of annual_death vec
            assert_eq!(ret, 46);
        }
    }
}