#[allow(async_fn_in_trait)]
pub trait MarketDataConnector {
    fn new() -> Self;

    async fn retrieve_and_cache_prices(&mut self, instrument_ids: &[String]);
    async fn subscribe_to_prices_stream(&self, instrument_ids: &Vec<String>);
}
