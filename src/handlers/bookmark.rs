use axum::extract::State;
use axum_extra::extract::Query;
use chimitheque_db::bookmark::toggle_product_bookmark;
use http::HeaderMap;
use std::ops::DerefMut;

use crate::{appstate::AppState, errors::AppError, utils::get_chimitheque_person_id_from_headers};

pub async fn toogle_bookmark(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(id): Query<u64>,
) -> Result<(), AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match toggle_product_bookmark(db_connection.deref_mut(), chimitheque_person_id, id) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
