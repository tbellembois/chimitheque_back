use axum::extract::{Path, State};
use axum_extra::extract::Query;
use chimitheque_db::borrowing::toggle_storage_borrowing;
use http::HeaderMap;
use serde::Deserialize;
use std::ops::DerefMut;

use crate::{appstate::AppState, errors::AppError, utils::get_chimitheque_person_id_from_headers};

#[derive(Deserialize)]
pub struct ToogleBorrowingQueryParameters {
    #[serde(default)]
    borrower_id: u64,
    #[serde(default)]
    borrowing_comment: String,
}

#[derive(Deserialize)]
pub struct ToogleBorrowingPathParameters {
    #[serde(default)]
    id: u64,
}

pub async fn toogle_borrowing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query_params): Query<ToogleBorrowingQueryParameters>,
    Path(path_params): Path<ToogleBorrowingPathParameters>,
) -> Result<(), AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match toggle_storage_borrowing(
        db_connection.deref_mut(),
        chimitheque_person_id,
        path_params.id,
        query_params.borrower_id,
        Some(query_params.borrowing_comment),
    ) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
