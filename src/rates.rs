use rand::prelude::*;
use serde::Deserialize;
use std::cmp::min;

#[derive(Copy, Clone, Debug, PartialEq, Deserialize)]
pub struct Rate {
    stocks: f64,
    bonds: f64,
    inflation: f64,
}

impl Rate {
    pub fn stocks(&self) -> f64 { self.stocks }
    pub fn bonds(&self) -> f64 { self.bonds }
    pub fn inflation(&self) -> f64 { self.inflation }
}

include!(concat!(env!("OUT_DIR"), "/rates.rs"));

pub fn generate_rates(rng: impl Rng, rates_in: &[Rate], sublength: usize, length: usize) -> Vec<Rate> {
    let dist = rand::distributions::Uniform::new(0, rates_in.len() + sublength - 1);
    generate_rates_with_distribution(rng, rates_in, sublength, length, dist)
}

fn generate_rates_with_distribution(mut rng: impl Rng, rates_in: &[Rate], sublength: usize, length: usize, dist: impl Distribution<usize>) -> Vec<Rate> {
    assert!(sublength <= rates_in.len());
    assert!(sublength != 0);
    assert!(rates_in.len() != 0);

    let mut rates = Vec::new();

    loop {
        let num = dist.sample(&mut rng);

        let slice: &[Rate];

        if num < sublength-1 {
            slice = &rates_in[..num+1];
        } else if num >= rates_in.len() {
            slice = &rates_in[num-sublength+1..];
        } else {
            slice = &rates_in[num+1-sublength..num+1];
        }

        let slice = &slice[0..min(slice.len(), length-rates.len())];

        rates.extend_from_slice(slice);

        if rates.len() == length {
            return rates;
        }

        assert!(rates.len() < length);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::mock::StepRng;

    struct MyUniform {
        top_excl: usize
    }

    impl MyUniform {
        fn new(i: usize) -> MyUniform { MyUniform {top_excl: i} }
    }

    impl Distribution<usize> for MyUniform {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> usize {
            rng.gen::<usize>() % self.top_excl
        }
    }

    fn rate_const(i: usize) -> Rate {
        Rate{ stocks: i as f64, bonds: i as f64, inflation: i as f64}
    }

    fn rate_seq(length: usize) -> Vec<Rate> {
        (0..length).map(|i| {
            rate_const(i)
        }).collect()
    }

    #[test]
    fn rate_getters() {
        let rate = Rate { stocks: 1.0, bonds: 2.0, inflation: 3.0 };
        assert_eq!(rate.stocks(), 1.0);
        assert_eq!(rate.bonds(), 2.0);
        assert_eq!(rate.inflation(), 3.0);
    }

    #[test]
    #[should_panic]
    fn rateprovider_sublength0() {
        generate_rates(StepRng::new(0, 1), &rate_seq(10), 0, 1);
    }

    #[test]
    #[should_panic]
    fn rateprovider_sublength_gt_rates() {
        generate_rates(StepRng::new(0, 1), &rate_seq(10), 11, 1);
    }

    #[test]
    fn rateprovider_rate3_sublength1_length6() {
        let rates_in = rate_seq(3);
        let out = generate_rates_with_distribution(StepRng::new(0, 1), &rates_in, 1, 6, MyUniform::new(3));
        let expected: Vec<Rate> = Vec::from([0usize, 1, 2, 0, 1, 2].map(|i| { rate_const(i) }));

        assert_eq!(out, expected);
    }

    #[test]
    fn rateprovider_rate6_sublength3_length18() {
        // 0
        // 0 1
        // 0 1 2
        // 1 2 3
        // 2 3 4
        // 3 4 5    
        // 4 5
        // 5

        let rates_in = rate_seq(6);
        let out = generate_rates_with_distribution(StepRng::new(0, 1), &rates_in, 3, 18, MyUniform::new(8));
        let expected: Vec<Rate> = Vec::from([0usize, 0, 1, 0, 1, 2, 1, 2, 3, 2, 3, 4, 3, 4, 5, 4, 5, 5].map(|i| { rate_const(i) }));

        assert_eq!(out, expected);
    }

    #[test]
    fn rateprovider_rate6_sublength3_length10() {
        // 0
        // 0 1
        // 0 1 2
        // 1 2 3
        // 2

        let rates_in = rate_seq(6);
        let out = generate_rates_with_distribution(StepRng::new(0, 1), &rates_in, 3, 10, MyUniform::new(8));
        let expected: Vec<Rate> = Vec::from([0usize, 0, 1, 0, 1, 2, 1, 2, 3, 2].map(|i| { rate_const(i) }));

        assert_eq!(out, expected);
    }

    #[test]
    fn rateprovider_rate6_sublength3_length20() {
        // 0
        // 0 1
        // 0 1 2
        // 1 2 3
        // 2 3 4
        // 3 4 5    
        // 4 5
        // 5
        // 0
        // 0

        let rates_in = rate_seq(6);
        let out = generate_rates_with_distribution(StepRng::new(0, 1), &rates_in, 3, 20, MyUniform::new(8));
        let expected: Vec<Rate> = Vec::from([0usize, 0, 1, 0, 1, 2, 1, 2, 3, 2, 3, 4, 3, 4, 5, 4, 5, 5, 0, 0].map(|i| { rate_const(i) }));

        assert_eq!(out, expected);
    }

    #[test]
    fn rateprovider_rate6_sublength3_length100_regression() {
        let rates_in = rate_seq(6);
        let out = generate_rates(rand_pcg::Pcg64Mcg::new(1337), &rates_in, 3, 100);
        let expected: Vec<Rate> = Vec::from([0, 1, 0, 0, 1, 1, 2, 3, 0, 1, 1, 2, 3, 2, 3, 4, 1, 2, 3, 5, 3, 4, 5, 5, 2, 3, 4, 1, 2, 3, 0, 1, 0, 1, 2, 3, 4, 0, 1, 2, 2, 3, 4, 0, 1, 2, 0, 3, 4, 5, 3, 4, 5, 3, 4, 5, 2, 3, 4, 3, 4, 5, 1, 2, 3, 0, 1, 3, 4, 5, 1, 2, 3, 0, 1, 2, 0, 1, 2, 3, 4, 5, 0, 1, 2, 1, 2, 3, 1, 2, 3, 4, 5, 2, 3, 4, 5, 4, 5, 1].map(|i| { rate_const(i) }));

        assert_eq!(out, expected);
    }
}