pub struct PricingController {}

impl PricingController {}

#[cfg(test)]
mod pricing_controller_tests {

    use polars::{frame::DataFrame, io::SerReader, prelude::CsvReadOptions};

    #[tokio::test]
    async fn variance() {
        let df = load_prices_ts_from_csv();

        println!("Dataframe: {:?}", df);

        assert_eq!(4, 4);
    }

    fn load_prices_ts_from_csv() -> DataFrame {
        CsvReadOptions::default()
            .map_parse_options(|parse_options| parse_options.with_try_parse_dates(true))
            .try_into_reader_with_file_path(Some("../misc/price_timeseries.csv".into()))
            .unwrap()
            .finish()
            .unwrap()
    }
}
