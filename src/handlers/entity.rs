use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chimitheque_types::{entity::Entity, requestfilter::RequestFilter, stock::Stock};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use crate::{AppState, errors::AppError, utils::get_chimitheque_person_id_from_headers};

pub async fn get_entities(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<Json<(Vec<Entity>, usize)>, AppError> {
    // Get the chimitheque_entity_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_entities = chimitheque_db::entity::get_entities(
        db_connection.deref(),
        request_filter,
        chimitheque_person_id,
    );

    match mayerr_entities {
        Ok(entities) => Ok(Json(entities)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GetEntitiesOldResponse {
    rows: Vec<Entity>,
    total: usize,
}

pub async fn get_entities_old(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<Json<Box<dyn erased_serde::Serialize>>, AppError> {
    // Get the chimitheque_entity_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_entities = chimitheque_db::entity::get_entities(
        db_connection.deref(),
        request_filter.clone(),
        chimitheque_person_id,
    );

    if request_filter.id.is_none() {
        match mayerr_entities {
            Ok(entities) => Ok(Json(Box::new(GetEntitiesOldResponse {
                rows: entities.0,
                total: entities.1,
            }))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    } else {
        match mayerr_entities {
            Ok(entities) => Ok(Json(Box::new(entities.0.first().unwrap().to_owned()))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUpdateEntityPathParameters {
    #[serde(default)]
    id: u64,
}

pub async fn create_update_entity(
    State(state): State<AppState>,
    Path(path_params): Path<CreateUpdateEntityPathParameters>,
    Json(entity): Json<Entity>,
) -> Result<Json<u64>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    // Sanitize and validate the entity.
    let mut entity = entity.clone();
    if let Err(err) = entity.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    };

    // update?
    if path_params.id > 0 {
        entity.entity_id = Some(path_params.id);
    }

    let mayerr_entity_id =
        chimitheque_db::entity::create_update_entity(db_connection.deref_mut(), entity);

    match mayerr_entity_id {
        Ok(entity_id) => Ok(Json(entity_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn delete_entity(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<(), AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::entity::delete_entity(db_connection.deref_mut(), id) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_entity_stock(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<Vec<Stock>>, AppError> {
    // Get the chimitheque_entity_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_stock =
        chimitheque_db::stock::compute_stock(db_connection.deref(), id, chimitheque_person_id);

    match mayerr_stock {
        Ok(stock) => Ok(Json(stock)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
