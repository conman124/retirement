use std::fmt::Display;
use std::rc::Rc;
use std::thread::LocalKey;

#[macro_export]
macro_rules! simplifying_assumption {
    ($a: literal) => {

    };
}

#[derive(Debug, Copy, Clone)]
pub struct Ratio<T>
{
    pub num: T,
    pub denum: T
}

impl<T> Ratio<T>
where T: Display {
    pub fn as_ratio(&self) -> String {
        format!("{}/{}", self.num, self.denum)
    }
}

impl<T> Ratio<T> 
where T: Into<f64> + Copy
{
    pub fn as_percent(&self) -> String {
        format!("{:.1}%", self.num.into() / self.denum.into() * 100.0)
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
        assert_eq!(Ratio{ num: 12, denum: 24}.as_ratio(), "12/24");
    }

    #[test]
    pub fn as_percent() {
        assert_eq!(Ratio{ num: 12.0, denum: 24.0}.as_percent(), "50.0%");
    }
}