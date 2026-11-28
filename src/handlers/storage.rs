use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use axum_extra::extract::Query;
use chimitheque_types::{person::Person, storage::Storage};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    AppState,
    authorization::refuse_unless_can_write_storages_in,
    axumrequestfilter::AxumRequestFilter,
    errors::AppError,
    utils::{
        get_chimitheque_person_id_from_headers, get_unit_types, refuse_if,
        refuse_missing_references,
    },
};

pub async fn get_storages(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum_request_filter: AxumRequestFilter,
) -> Result<Json<(Vec<Storage>, usize)>, AppError> {
    info!("get_storages");

    let request_filter = axum_request_filter.0;

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_storages = chimitheque_db::storage::get_storages(
        &db_connection,
        request_filter,
        chimitheque_person_id,
    );

    match mayerr_storages {
        Ok(storages) => Ok(Json(storages)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GetStoragesOldResponse {
    rows: Vec<Storage>,
    total: usize,
}

pub async fn get_storages_old(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum_request_filter: AxumRequestFilter,
) -> Result<Json<Box<dyn erased_serde::Serialize>>, AppError> {
    info!("get_storages_old");

    let request_filter = axum_request_filter.0;

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_storages = chimitheque_db::storage::get_storages(
        &db_connection,
        request_filter.clone(),
        chimitheque_person_id,
    );

    // hack: request_filter.limit == Some(1) -> QRCode scanner
    if request_filter.id.is_none() || request_filter.limit == Some(1) {
        match mayerr_storages {
            Ok(storages) => Ok(Json(Box::new(GetStoragesOldResponse {
                rows: storages.0,
                total: storages.1,
            }))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    } else {
        match mayerr_storages {
            Ok(storages) => Ok(Json(Box::new(storages.0.first().unwrap().to_owned()))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUpdateStorageQueryParameters {
    #[serde(default = "default_nb_items")]
    nb_items: u64,
    #[serde(default)]
    identical_barecode: bool,
}

fn default_nb_items() -> u64 {
    1
}

#[derive(Deserialize)]
pub struct CreateUpdateStoragePathParameters {
    #[serde(default)]
    id: u64,
}

pub async fn create_update_storage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(mut query_params): Query<CreateUpdateStorageQueryParameters>,
    Path(path_params): Path<CreateUpdateStoragePathParameters>,
    Json(storage): Json<Storage>,
) -> Result<Json<Vec<u64>>, AppError> {
    info!("create_update_storage: {}", storage);

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    // Sanitize and validate the storage.
    let mut storage = storage.clone();
    if let Err(err) = storage.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    }

    // Check the unit types against the database, not the request.
    let unit_types = get_unit_types(&db_connection)?;
    if let Err(err) = storage.validate_unit_types(|unit_id| unit_types.get(&unit_id).copied()) {
        return Err(AppError::InputValidation(err.to_string()));
    }

    // Set storage person.
    storage.person = Person {
        person_id: Some(chimitheque_person_id),
        ..Default::default()
    };

    // update?
    if path_params.id > 0 {
        storage.storage_id = Some(path_params.id);
        refuse_history(&db_connection, path_params.id)?;
    }

    // Ensure nb_items not zero.
    if query_params.nb_items == 0 {
        query_params.nb_items = 1;
    }
    check_nb_items(path_params.id > 0, query_params.nb_items)?;

    refuse_missing_references(
        &db_connection,
        &chimitheque_db::storage::storage_references(&storage),
    )?;

    // The storage is written in the entity of its store location.
    if let Some(store_location_id) = storage.store_location.store_location_id {
        refuse_unless_can_write_storages_in(
            &db_connection,
            chimitheque_person_id,
            store_location_id,
        )?;
    }

    let mayerr_storage_id = chimitheque_db::storage::create_update_storage(
        &mut db_connection,
        storage,
        query_params.nb_items,
        query_params.identical_barecode,
    );

    match mayerr_storage_id {
        Ok(storage_id) => Ok(Json(storage_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn delete_storage(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<(), AppError> {
    info!("delete_storage: {}", id);

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    refuse_history(&db_connection, id)?;

    match chimitheque_db::storage::delete_storage(&mut db_connection, id) {
        Ok(()) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn export_storages(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum_request_filter: AxumRequestFilter,
) -> Result<String, AppError> {
    info!("export_storages");

    let request_filter = axum_request_filter.0;

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_storages = chimitheque_db::storage::export_storages(
        &db_connection,
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
    Path(id): Path<u64>,
) -> Result<(), AppError> {
    info!("archive_storage: {}", id);

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    refuse_history(&db_connection, id)?;

    match chimitheque_db::storage::archive_storage(&mut db_connection, id) {
        Ok(()) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn unarchive_storage(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<(), AppError> {
    info!("unarchive_storage: {}", id);

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    refuse_history(&db_connection, id)?;

    match chimitheque_db::storage::unarchive_storage(&mut db_connection, id) {
        Ok(()) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

/// Refuses a change of a history row: an earlier version of a storage.
fn refuse_history(db_connection: &rusqlite::Connection, storage_id: u64) -> Result<(), AppError> {
    refuse_if(
        chimitheque_db::storage::storage_is_history(db_connection, storage_id),
        "a storage history row can not be changed",
    )
}

/// Refuses several copies of an updated storage: an update writes one storage.
fn check_nb_items(is_update: bool, nb_items: u64) -> Result<(), AppError> {
    if is_update && nb_items > 1 {
        return Err(AppError::InputValidation(
            "nb_items must be 1 to update a storage".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_nb_items() {
        assert!(check_nb_items(false, 5).is_ok());
        assert!(check_nb_items(true, 1).is_ok());
        assert!(matches!(
            check_nb_items(true, 2),
            Err(AppError::InputValidation(_))
        ));
    }
}
