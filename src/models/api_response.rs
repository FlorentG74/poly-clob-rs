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
