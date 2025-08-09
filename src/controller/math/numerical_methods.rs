use linfa::prelude::*;
use linfa::traits::Fit;
use linfa_linear::LinearRegression;
use ndarray::prelude::*;
use polars::prelude::*;

use log::Level::Debug;

use crate::controller::{plot_lin_reg, YEAR_MILLIS};

pub struct NumericalMethods;

impl NumericalMethods {
    pub fn average_sampling_frequency(df: &DataFrame) -> u32 {
        let timestamps = df.column("timestamp").unwrap();
        let nb_observations = i64::from(timestamps.len() as u32);

        let first_observation = timestamps.get(0).unwrap();
        let first_ts: i64 = first_observation.try_extract().unwrap();

        let last_observation = timestamps.get(timestamps.len() - 1).unwrap();
        let last_ts: i64 = last_observation.try_extract().unwrap();

        let step =
            f64::round(((last_ts - first_ts) as f64) / ((1000000 * nb_observations) as f64)) as u32;

        log::debug!("Avg Step {:?}", step);

        step
    }

    pub fn downsample_time_series(df: &DataFrame, freq: i64) -> DataFrame {
        // resample dataframe at 2* average frequency (e.g. 10s)
        let resampling_freq = format!("{}ms", freq);

        let downsampled_df = df
            .clone()
            .lazy()
            .group_by_dynamic(
                col("timestamp"),
                [],
                DynamicGroupOptions {
                    every: Duration::parse(resampling_freq.as_str()),
                    period: Duration::parse(resampling_freq.as_str()),
                    offset: Duration::parse("0"),
                    ..Default::default()
                },
            )
            .agg([col("bid").mean(), col("ask").mean(), col("mid").mean()])
            .collect()
            .unwrap();

        downsampled_df
    }

    pub fn calc_rolling_averages(df: &DataFrame, every: &str, period: &str) -> DataFrame {
        df.clone()
            .lazy()
            .group_by_dynamic(
                col("timestamp"),
                [],
                DynamicGroupOptions {
                    every: Duration::parse(every),
                    period: Duration::parse(period),
                    offset: Duration::parse("0"),
                    ..Default::default()
                },
            )
            .agg([col("bid").mean(), col("ask").mean(), col("mid").mean()])
            .collect()
            .unwrap()
    }

    pub fn calc_historical_variances(df: &DataFrame, period: &str) -> DataFrame {
        let ddof: u8 = 1;

        let period_duration = Duration::parse(period);

        // Offset by period min. 1s to calculate in arrears
        let offset_duration =
            Duration::parse(format!("{}ns", -period_duration.nanoseconds() + 1000000000).as_str());

        df.clone()
            .lazy()
            .rolling(
                col("timestamp"),
                [],
                RollingGroupOptions {
                    period: period_duration,
                    offset: offset_duration,
                    ..Default::default()
                },
            )
            .agg([
                col("bid").var(ddof),
                col("ask").var(ddof),
                col("mid").var(ddof),
            ])
            .collect()
            .unwrap()
    }

    pub fn calc_historical_annual_vol(
        df: &DataFrame,
        sampling_period_ms: i64,
        nb_periods: u32,
    ) -> f64 {
        let nb_periods_in_year = YEAR_MILLIS / sampling_period_ms;

        let sqrt_t = f64::sqrt(nb_periods_in_year as f64);

        let sample_df = df
            .clone()
            .lazy()
            .tail(nb_periods + 1)
            .select([col("mid")])
            .with_row_index("index", Some(0))
            .collect()
            .unwrap();

        log::debug!("Sample DF in histo Vol calc {:?}", sample_df);

        // Calculate returns
        let vol_df = sample_df
            .clone()
            .lazy()
            .rolling(
                col("index"),
                [],
                RollingGroupOptions {
                    index_column: "index".into(),
                    period: Duration::parse("2i"),
                    offset: Duration::parse("-1i"),
                    ..Default::default()
                },
            )
            .agg([col("mid")])
            .with_column(
                ((col("mid").list().last() / col("mid").list().first()) - lit(1))
                    //.log(f64::exp(1.0))
                    .alias("return"),
            )
            .tail(nb_periods)
            .std(1)
            .select([(col("return") * lit(sqrt_t)).alias("vol")])
            .collect()
            .unwrap();

        return vol_df
            .column("vol")
            .unwrap()
            .get(0)
            .unwrap()
            .try_extract()
            .unwrap();
    }

    pub fn linear_regression(data: DataFrame, x: &str, y: &str) -> (f64, f64) {
        // Normalize time values (Substract min value & divide by 1000)
        let array = data
            .lazy()
            .select([(col(x)).cast(DataType::Int64).alias(x), (col(y)).alias(y)])
            .with_column(((col(x) - col(x).first()) / lit(1000)).alias(x))
            .collect()
            .unwrap()
            .to_ndarray::<Float64Type>(IndexOrder::Fortran)
            .unwrap();

        log::debug!("Normalized values for lin-reg: {:?}", &array);

        // Converting from an array to a Linfa Dataset
        let (data, targets) = (
            (&array).slice(s![.., 0..1]).to_owned(),
            (&array).column(1).to_owned(),
        );

        //Build dataset
        let dataset = Dataset::new(data, targets).with_feature_names(vec!["x", "y"]);

        // Perform regression
        let lin_reg = LinearRegression::new();
        let model = lin_reg.fit(&dataset).unwrap();

        let a = model.params()[0];
        let b = model.intercept();

        if log::log_enabled!(Debug) {
            plot_lin_reg(array.clone(), a, b);
        }

        return (a, b);
    }

    pub fn generate_f64_range(start: f64, end: f64, step: f64) -> Vec<f64> {
        let mut range = Vec::<f64>::new();
        let mut curr = start;

        while curr <= end {
            range.push(curr);
            curr = curr + step;
        }

        return range;
    }
}

#[cfg(test)]
mod market_data_controller_tests {
    use chrono::NaiveDate;
    use polars::prelude::*;

    use crate::controller::{NumericalMethods, DAY_MILLIS};

    #[test]
    fn generate_range() {
        let range = NumericalMethods::generate_f64_range(0.0, 0.1, 0.01);

        println!("Range {:?}", range);
    }

    #[test]
    fn calc_hist_vol() {
        let date_series = Column::new(
            "timestamp".into(),
            &[
                NaiveDate::from_ymd_opt(2017, 5, 23)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 5, 24)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 5, 25)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 5, 26)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 5, 29)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 5, 30)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 5, 31)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 6, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 6, 2)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 6, 5)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                NaiveDate::from_ymd_opt(2017, 6, 6)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            ],
        );

        let value_series = Column::new(
            "mid".into(),
            &[
                147.82, 149.5, 149.78, 149.86, 149.93, 150.89, 152.39, 153.74, 152.79, 151.23,
                151.78,
            ],
        );

        let prices_df: DataFrame = DataFrame::new(vec![date_series, value_series]).unwrap();

        let vol = NumericalMethods::calc_historical_annual_vol(&prices_df, DAY_MILLIS, 10);

        assert!(f64::abs(vol - 0.13296) < 0.01);
    }
}
