pub struct AssetAllocation {
    stocks_glide: Vec<f64>,
}

impl AssetAllocation {
    pub fn new(stocks_glide: Vec<f64>) -> AssetAllocation {
        assert!(stocks_glide.len() >= 1);
        assert!(stocks_glide.iter().min_by(|x,y| x.partial_cmp(y).unwrap()).unwrap() >= &0.0);
        assert!(stocks_glide.iter().max_by(|x,y| x.partial_cmp(y).unwrap()).unwrap() <= &1.0);

        AssetAllocation{ stocks_glide }
    }

    pub fn new_linear_glide(periods_before: usize, start_stocks: f64, periods_glide: usize, end_stocks: f64) -> AssetAllocation {
        assert!(periods_before >= 1);
        assert!(periods_glide >= 1);
        assert!(start_stocks >= 0.0 && start_stocks <= 1.0);
        assert!(end_stocks >= 0.0 && end_stocks <= 1.0);

        let mut stocks_glide = vec![start_stocks; periods_before + periods_glide];
        
        for i in periods_before..periods_before+periods_glide {
            let frac = (i - periods_before + 1) as f64 / periods_glide as f64;
            stocks_glide[i] = frac * (end_stocks - start_stocks) + start_stocks;
        }

        AssetAllocation { stocks_glide }
    }

    pub fn stocks(&self, period: usize) -> f64 {
        if period < self.stocks_glide.len() {
            self.stocks_glide[period]
        } else {
            *self.stocks_glide.last().unwrap()
        }
    }

    pub fn bonds(&self, period: usize) -> f64 {
        1.0 - self.stocks(period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec() {
        let assets = AssetAllocation::new(vec![1.0, 1.0, 1.0, 1.0, 0.5, 0.75]);

        assert_eq!(assets.stocks(0), 1.0);
        assert_eq!(assets.stocks(1), 1.0);
        assert_eq!(assets.stocks(2), 1.0);
        assert_eq!(assets.stocks(3), 1.0);
        assert_eq!(assets.stocks(4), 0.5);
        assert_eq!(assets.stocks(5), 0.75);
        assert_eq!(assets.stocks(6), 0.75);
        assert_eq!(assets.stocks(100), 0.75);

        assert_eq!(assets.bonds(0), 0.0);
        assert_eq!(assets.bonds(1), 0.0);
        assert_eq!(assets.bonds(2), 0.0);
        assert_eq!(assets.bonds(3), 0.0);
        assert_eq!(assets.bonds(4), 0.5);
        assert_eq!(assets.bonds(5), 0.25);
        assert_eq!(assets.bonds(6), 0.25);
        assert_eq!(assets.bonds(100), 0.25);
    }

    #[test]
    fn linearglide() {
        let assets = AssetAllocation::new_linear_glide(4, 1.0, 4, 0.5);

        assert_eq!(assets.stocks(0), 1.0);
        assert_eq!(assets.stocks(1), 1.0);
        assert_eq!(assets.stocks(2), 1.0);
        assert_eq!(assets.stocks(3), 1.0);
        assert_eq!(assets.stocks(4), 0.875);
        assert_eq!(assets.stocks(5), 0.75);
        assert_eq!(assets.stocks(6), 0.625);
        assert_eq!(assets.stocks(7), 0.5);
        assert_eq!(assets.stocks(8), 0.5);
        assert_eq!(assets.stocks(100), 0.5);

        assert_eq!(assets.bonds(0), 0.0);
        assert_eq!(assets.bonds(1), 0.0);
        assert_eq!(assets.bonds(2), 0.0);
        assert_eq!(assets.bonds(3), 0.0);
        assert_eq!(assets.bonds(4), 0.125);
        assert_eq!(assets.bonds(5), 0.25);
        assert_eq!(assets.bonds(6), 0.375);
        assert_eq!(assets.bonds(7), 0.5);
        assert_eq!(assets.bonds(8), 0.5);
        assert_eq!(assets.bonds(100), 0.5);

    }
}