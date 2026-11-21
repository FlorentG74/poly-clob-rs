/// Trait for types that represent API responses
///
/// This trait only deals with the API contract - deserialization and result counting.
/// It does NOT handle storage or persistence - that's the responsibility of separate
/// storage layers in the application.
pub trait ApiResponse {
    /// Returns the number of results in this response
    ///
    /// Used for pagination logic to determine if there are more pages to fetch
    fn nb_results(&self) -> usize;
}

/// Trait for types that represent keyset-paginated API responses.
///
/// Keyset (cursor-based) pagination returns a `next_cursor` token instead of an
/// offset. Pass the cursor back on the next request. When it is `None` there are
/// no more pages. Implementors guarantee that empty-string cursors from the API
/// are already normalised to `None` (via [`deserialize_cursor`] on the field).
pub trait KeysetApiResponse {
    /// Returns the cursor for the next page, or `None` when no more pages exist.
    fn next_cursor(&self) -> Option<&str>;
}

/// Serde helper: deserialise an `Option<String>` field and coerce `""` to `None`.
///
/// Apply with `#[serde(default, deserialize_with = "deserialize_cursor")]` on
/// keyset `next_cursor` fields so callers never observe an empty-string cursor.
///
/// # Errors
///
/// If the cursor field is present but is neither null nor a string.
pub fn deserialize_cursor<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

/// Data API v2 response envelope: every route wraps its rows in `data`, and paginated
/// routes add a `pagination` object whose `next_cursor` is `null` on the last page.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DataApiPage<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: Option<DataApiPagination>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DataApiPagination {
    #[serde(default, deserialize_with = "deserialize_cursor")]
    pub next_cursor: Option<String>,
}

impl<T> KeysetApiResponse for DataApiPage<T> {
    fn next_cursor(&self) -> Option<&str> {
        self.pagination.as_ref().and_then(|p| p.next_cursor.as_deref())
    }
}
