use axum::extract::{Path, State};
use axum_extra::extract::Query;
use chimitheque_db::{borrowing::toggle_storage_borrowing, reference::Reference};
use chimitheque_types::{borrowing::Borrowing, person::Person};
use http::HeaderMap;
use serde::Deserialize;

use crate::{
    appstate::AppState,
    errors::AppError,
    utils::{get_chimitheque_person_id_from_headers, refuse_missing_references},
};

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

    // Sanitize the borrowing: the comment is trimmed, and an empty comment
    // is stored as NULL.
    let mut borrowing = Borrowing {
        borrowing_comment: Some(query_params.borrowing_comment),
        person: chimitheque_person_id,
        storage: path_params.id,
        borrower: Person {
            person_id: Some(query_params.borrower_id),
            ..Default::default()
        },
        ..Default::default()
    };
    if let Err(err) = borrowing.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    }

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    refuse_missing_references(
        &db_connection,
        &[
            Reference::new("storage", "storage", Some(path_params.id)),
            Reference::new("borrower", "person", Some(query_params.borrower_id)),
        ],
    )?;

    match toggle_storage_borrowing(
        &mut db_connection,
        chimitheque_person_id,
        path_params.id,
        query_params.borrower_id,
        borrowing.borrowing_comment,
    ) {
        Ok(()) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
