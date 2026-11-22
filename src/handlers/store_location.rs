use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chimitheque_types::storelocation::StoreLocation;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    AppState,
    axumrequestfilter::AxumRequestFilter,
    errors::AppError,
    utils::{get_chimitheque_person_id_from_headers, refuse_if, refuse_missing_references},
};

pub async fn get_store_locations(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum_request_filter: AxumRequestFilter,
) -> Result<Json<(Vec<StoreLocation>, usize)>, AppError> {
    info!("get_store_locations");

    let request_filter = axum_request_filter.0;

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_store_locations = chimitheque_db::storelocation::get_store_locations(
        &db_connection,
        &request_filter,
        chimitheque_person_id,
    );

    match mayerr_store_locations {
        Ok(store_locations) => Ok(Json(store_locations)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GetStoreLocationsOldResponse {
    rows: Vec<StoreLocation>,
    total: usize,
}

pub async fn get_store_locations_old(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum_request_filter: AxumRequestFilter,
) -> Result<Json<Box<dyn erased_serde::Serialize>>, AppError> {
    info!("get_store_locations_old");

    let request_filter = axum_request_filter.0;

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_store_locations = chimitheque_db::storelocation::get_store_locations(
        &db_connection,
        &request_filter.clone(),
        chimitheque_person_id,
    );

    if request_filter.id.is_none() {
        match mayerr_store_locations {
            Ok(store_locations) => Ok(Json(Box::new(GetStoreLocationsOldResponse {
                rows: store_locations.0,
                total: store_locations.1,
            }))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    } else {
        match mayerr_store_locations {
            Ok(store_locations) => Ok(Json(Box::new(
                store_locations.0.first().unwrap().to_owned(),
            ))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUpdateStoreLocationPathParameters {
    #[serde(default)]
    id: u64,
}

pub async fn create_update_store_location(
    State(state): State<AppState>,
    Path(path_params): Path<CreateUpdateStoreLocationPathParameters>,
    Json(store_location): Json<StoreLocation>,
) -> Result<Json<u64>, AppError> {
    info!("create_update_store_location: {}", store_location);

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    // update? Set before validating: a store location can not be its own parent.
    let mut store_location = store_location.clone();
    if path_params.id > 0 {
        store_location.store_location_id = Some(path_params.id);
    }

    // Sanitize and validate the store location.
    if let Err(err) = store_location.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    }

    refuse_missing_references(
        &db_connection,
        &chimitheque_db::storelocation::store_location_references(&store_location),
    )?;

    let mayerr_store_location_id = chimitheque_db::storelocation::create_update_store_location(
        &mut db_connection,
        store_location,
    );

    match mayerr_store_location_id {
        Ok(store_location_id) => Ok(Json(store_location_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn delete_store_location(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<(), AppError> {
    info!("delete_store_location: {}", id);

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    refuse_if(
        chimitheque_db::storelocation::store_location_has_storages(&db_connection, id),
        "the store location still has storages",
    )?;
    refuse_if(
        chimitheque_db::storelocation::store_location_has_children(&db_connection, id),
        "the store location still has child store locations",
    )?;

    match chimitheque_db::storelocation::delete_store_location(&db_connection, id) {
        Ok(()) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
