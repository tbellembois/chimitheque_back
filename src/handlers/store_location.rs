use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use chimitheque_types::{requestfilter::RequestFilter, storelocation::StoreLocation};
use std::ops::{Deref, DerefMut};

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

    let mayerr_store_locations = chimitheque_db::storelocation::get_store_locations(
        db_connection.deref(),
        request_filter,
        chimitheque_person_id,
    );

    match mayerr_store_locations {
        Ok(store_locations) => Ok(Json(store_locations)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn create_update_store_location(
    State(state): State<AppState>,
    Json(store_location): Json<StoreLocation>,
) -> Result<Json<u64>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    let mayerr_store_location_id = chimitheque_db::storelocation::create_update_store_location(
        db_connection.deref_mut(),
        store_location,
    );

    match mayerr_store_location_id {
        Ok(store_location_id) => Ok(Json(store_location_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn delete_store_location(
    State(state): State<AppState>,
    Query(id): Query<u64>,
) -> Result<(), AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::storelocation::delete_store_location(db_connection.deref_mut(), id) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
