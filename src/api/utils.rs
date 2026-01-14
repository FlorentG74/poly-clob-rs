use serde::{Deserializer, Deserialize};

/// Helper type that can deserialize from either a string or a number
#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Float(f32),
}

/// Deserialize either a string or a number as an Option<f32>.
/// This supports both JSON API responses (where numbers are strings) and
/// MessagePack serialization (where numbers are native types).
pub fn deserialize_string_to_option_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let opt: Option<StringOrNumber> = Option::deserialize(deserializer)?;
    match opt {
        Some(StringOrNumber::String(s)) => s
            .parse::<f32>()
            .map(Some)
            .map_err(|_| D::Error::custom(format!("failed to parse '{}' as f32", s))),
        Some(StringOrNumber::Float(f)) => Ok(Some(f)),
        None => Ok(None),
    }
}