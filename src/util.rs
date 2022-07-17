use std::fmt::Display;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn as_ratio() {
        assert_eq!(Ratio{ num: 12, denum: 24}.as_ratio(), "12/24");
    }

    #[test]
    pub fn as_percent() {
        assert_eq!(Ratio{ num: 12.0, denum: 24.0}.as_percent(), "50.0%");
    }
}