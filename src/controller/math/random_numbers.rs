use polars::frame::DataFrame;
use polars::prelude::Column;
use rand::distributions::Standard;
use rand::{rngs::StdRng, Rng, SeedableRng};
pub struct RandomNumbersGenerator {}

impl Default for RandomNumbersGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomNumbersGenerator {
    pub fn new() -> Self {
        RandomNumbersGenerator {}
    }

    pub fn get_rand(&self) -> f32 {
        let rnd: f32 = StdRng::from_entropy().sample(Standard);
        rnd
    }

    pub fn get_multiple_rand(&self, nb_rands: usize) -> Vec<f32> {
        let mut res = Vec::<f32>::new();

        for _ in 0..nb_rands {
            let rnd: f32 = StdRng::from_entropy().sample(Standard);
            res.push(rnd);
        }

        res
    }

    pub fn get_dataframe_rand(&self, rows: usize, cols: usize) -> DataFrame {
        let mut columns = Vec::<Column>::new();

        let total_size = rows * cols;
        let mut rnd_numbers = self.get_multiple_rand(total_size);

        for i in 0..cols {
            if i == cols - 1 {
                let column = Column::new(format!("{}", i).into(), &rnd_numbers);
                columns.push(column);
            } else {
                let row = rnd_numbers.split_off(rnd_numbers.len() - rows);
                let column = Column::new(format!("{}", i).into(), &row);
                columns.push(column);
            }
        }

        let rnd_df: DataFrame = DataFrame::new(columns).unwrap();

        rnd_df
    }
}

#[cfg(test)]
mod random_numbers_tests {

    #[tokio::test]
    async fn rng() {
        let rng = crate::controller::RandomNumbersGenerator::new();
        let rnds = rng.get_multiple_rand(100);

        log::info!("rnd numbers {:?}", rnds.len());

        assert_eq!(rnds.len(), 100);
    }

    #[tokio::test]
    async fn rng_df() {
        let rng = crate::controller::RandomNumbersGenerator::new();
        let rnd_df = rng.get_dataframe_rand(50, 10);

        log::info!("{}", rnd_df);

        assert_eq!(4, 4);
    }
}
