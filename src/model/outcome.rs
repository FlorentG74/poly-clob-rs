use crate::model::PolyResponseMarket;

#[derive(Debug)]
pub struct Outcome {
    pub token_id: String,
    pub outcome: String,
    pub price: f64,
}

impl Outcome {
    pub fn new(market: &PolyResponseMarket, token_id: String) -> Self {
        let outcomes_str = market.outcomes.as_ref().unwrap();
        let outcomes: Vec<String> = serde_json::from_str(outcomes_str).unwrap();

        let outcome_prices_str = market.outcome_prices.as_ref().unwrap();
        let outcome_prices: Vec<String> = serde_json::from_str(outcome_prices_str).unwrap();

        let clob_token_ids_str = market.clob_token_ids.as_ref().unwrap();
        let clob_token_ids: Vec<String> = serde_json::from_str(clob_token_ids_str).unwrap();

        let mut outcome: String = "N/A".to_string();
        let mut price: f64 = 0.0;

        for t in 0..(outcomes.len() - 1) {
            if token_id.eq(clob_token_ids.get(t).unwrap()) {
                price = (outcome_prices.get(t).unwrap()).parse().unwrap();
                outcome = (outcomes.get(t).unwrap()).parse().unwrap();
                break;
            }
        }

        Outcome {
            token_id,
            outcome,
            price,
        }
    }
}
