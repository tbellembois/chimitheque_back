use axum::extract::FromRequestParts;
use chimitheque_types::requestfilter::RequestFilter;
use http::{StatusCode, request::Parts};
use serde::Serialize;

#[derive(Debug, Serialize, Default)]
pub struct AxumRequestFilter(pub RequestFilter);

impl<S> FromRequestParts<S> for AxumRequestFilter
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Build full absolute URL string so RequestFilter::try_from works
        // axum Parts.uri may not contain scheme/host, so we fake it
        let uri = parts.uri.clone();
        let path_and_query = uri
            .path_and_query()
            .map_or("", http::uri::PathAndQuery::as_str);
        let fake_base = "http://localhost"; // arbitrary base

        let full_url = format!("{fake_base}{path_and_query}");

        match RequestFilter::try_from(full_url.as_str()) {
            Ok(filter) => Ok(AxumRequestFilter(filter)),
            Err(err) => {
                // Respond with 400 + error message
                let body = format!("invalid query/filter: {err}");
                let response = (StatusCode::BAD_REQUEST, body);
                Err(response)
            }
        }
    }
}
