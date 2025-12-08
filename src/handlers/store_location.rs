use axum::{Json, extract::State, http::HeaderMap};
use chimitheque_types::{requestfilter::RequestFilter, storelocation::StoreLocation};
use std::ops::Deref;

use crate::{AppState, errors::AppError, utils::get_chimitheque_person_id_from_headers};

pub async fn get_store_locations(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<Json<(Vec<StoreLocation>, usize)>, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let store_locations = chimitheque_db::storelocation::get_store_locations(
        db_connection.deref(),
        request_filter,
        chimitheque_person_id,
    );

    match store_locations {
        Ok(store_locations) => Ok(Json(store_locations)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
