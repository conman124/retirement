use std::rc::Rc;
use std::thread::LocalKey;

use wasm_bindgen::prelude::*;

#[macro_export]
macro_rules! simplifying_assumption {
    ($a: literal) => {

    };
}

#[derive(Debug, Copy, Clone)]
#[wasm_bindgen]
pub struct Ratio
{
    pub num: usize,
    pub denom: usize
}

impl Ratio {
    pub fn as_ratio(&self) -> String {
        format!("{}/{}", self.num, self.denom)
    }

    pub fn as_percent(&self) -> String {
        format!("{:.1}%", self.num as f64 / self.denom as f64 * 100.0)
    }
}

pub fn get_thread_local_rc<T: ?Sized>(key: &'static LocalKey<Rc<T>>) -> Rc<T> {
    let mut option = Option::default();

    key.with(|inner| {
        option.replace(Rc::clone(inner));
    });

    option.unwrap()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use assert_float_eq::*;

    pub fn assert_vecfloat_absolute(vec1: Vec<f64>, vec2: Vec<f64>) -> () {
        assert_eq!(vec1.len(), vec2.len());

        for (f1, f2) in vec1.iter().zip(vec2) {
            assert_float_absolute_eq!(f1, f2);
        }
    }


    #[test]
    pub fn as_ratio() {
        assert_eq!(Ratio{ num: 12, denom: 24}.as_ratio(), "12/24");
    }

    #[test]
    pub fn as_percent() {
        assert_eq!(Ratio{ num: 12, denom: 24}.as_percent(), "50.0%");
    }
}