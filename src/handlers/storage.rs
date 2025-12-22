use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use chimitheque_types::{requestfilter::RequestFilter, storage::Storage};
use std::ops::{Deref, DerefMut};

use crate::{AppState, errors::AppError, utils::get_chimitheque_person_id_from_headers};

pub async fn get_storages(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<Json<(Vec<Storage>, usize)>, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_storages = chimitheque_db::storage::get_storages(
        db_connection.deref(),
        request_filter,
        chimitheque_person_id,
    );

    match mayerr_storages {
        Ok(storages) => Ok(Json(storages)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn create_update_storage(
    State(state): State<AppState>,
    Query(nb_items): Query<u64>,
    Query(identical_barecode): Query<bool>,
    Json(storage): Json<Storage>,
) -> Result<Json<Vec<u64>>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    // Sanitize and validate the storage.
    let mut storage = storage.clone();
    if let Err(err) = storage.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    };

    let mayerr_storage_id = chimitheque_db::storage::create_update_storage(
        db_connection.deref_mut(),
        storage,
        nb_items,
        identical_barecode,
    );

    match mayerr_storage_id {
        Ok(storage_id) => Ok(Json(storage_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn delete_storage(
    State(state): State<AppState>,
    Query(id): Query<u64>,
) -> Result<(), AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::storage::delete_storage(db_connection.deref_mut(), id) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn export_storages(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<String, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_storages = chimitheque_db::storage::export_storages(
        db_connection.deref(),
        request_filter,
        chimitheque_person_id,
    );

    match mayerr_storages {
        Ok(storages) => Ok(storages),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn archive_storage(
    State(state): State<AppState>,
    Query(id): Query<u64>,
) -> Result<(), AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::storage::archive_storage(db_connection.deref_mut(), id) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn unarchive_storage(
    State(state): State<AppState>,
    Query(id): Query<u64>,
) -> Result<(), AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::storage::unarchive_storage(db_connection.deref_mut(), id) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
