use serde::{Deserializer, Deserialize};

/// Deserialize a string as an Option<f32>.
pub fn deserialize_string_to_option_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => s
            .parse::<f32>()
            .map(Some)
            .map_err(|_| D::Error::custom(format!("failed to parse '{}' as f32", s))),
        None => Ok(None),
    }
}