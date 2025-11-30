use serde::Deserialize;

use super::ApiResponse;

pub type PolymarketTagsResponse = Vec<PolyResponseTag>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolyResponseTag {
    pub id: String,
    pub label: Option<String>,
    pub slug: Option<String>,
    pub published_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(rename = "_sync")]
    pub sync: Option<bool>,
}

// ApiResponse implementations
impl ApiResponse for PolymarketTagsResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}

impl ApiResponse for PolyResponseTag {
    fn nb_results(&self) -> usize {
        0 // Single tag response
    }
}
