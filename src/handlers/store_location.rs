use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chimitheque_types::{permission::PermissionItem, storelocation::StoreLocation};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    AppState,
    authorization::{
        refuse_unless_can_write_in_entity, refuse_unless_can_write_in_store_location_entity,
    },
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
    headers: HeaderMap,
    Path(path_params): Path<CreateUpdateStoreLocationPathParameters>,
    Json(store_location): Json<StoreLocation>,
) -> Result<Json<u64>, AppError> {
    info!("create_update_store_location: {}", store_location);

    // Get the chimitheque_person_id.
    let chimitheque_person_id = get_chimitheque_person_id_from_headers(&headers)?;

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

    // The store location is written in its entity, under a parent of an
    // entity the person can write too.
    if let Some(entity_id) = store_location
        .entity
        .as_ref()
        .and_then(|entity| entity.entity_id)
    {
        refuse_unless_can_write_in_entity(
            &db_connection,
            chimitheque_person_id,
            &PermissionItem::Entities,
            entity_id,
        )?;
    }
    if let Some(parent_id) = store_location
        .store_location
        .as_ref()
        .and_then(|parent| parent.store_location_id)
    {
        refuse_unless_can_write_in_store_location_entity(
            &db_connection,
            chimitheque_person_id,
            &PermissionItem::Entities,
            parent_id,
        )?;
    }

    // A store location stays in its entity, with a parent of the same entity.
    let entity_id_of = |store_location_id| {
        chimitheque_db::storelocation::store_location_entity_id(&db_connection, store_location_id)
            .map_err(|err| AppError::Database(err.to_string()))
    };
    let stored_entity_id = match store_location.store_location_id {
        Some(store_location_id) => entity_id_of(store_location_id)?,
        None => None,
    };
    let parent_entity_id = match store_location
        .store_location
        .as_ref()
        .and_then(|parent| parent.store_location_id)
    {
        Some(parent_id) => entity_id_of(parent_id)?,
        None => None,
    };
    check_store_location_entity(
        stored_entity_id,
        store_location
            .entity
            .as_ref()
            .and_then(|entity| entity.entity_id),
        parent_entity_id,
    )
    .map_err(AppError::InputValidation)?;

    // A store location can not be moved inside itself.
    if let Some(store_location_id) = store_location.store_location_id
        && let Some(parent_id) = store_location
            .store_location
            .as_ref()
            .and_then(|parent| parent.store_location_id)
    {
        refuse_if(
            chimitheque_db::storelocation::store_location_is_ancestor_of(
                &db_connection,
                store_location_id,
                parent_id,
            ),
            "the parent store location is inside this store location",
        )?;
    }

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

/// Checks that a store location stays in its entity, and that its parent is in
/// the same entity.
///
/// `stored_entity_id` is the entity of an updated store location, `entity_id`
/// the one of the request (an update without one keeps its entity) and
/// `parent_entity_id` the entity of the parent, if any.
fn check_store_location_entity(
    stored_entity_id: Option<u64>,
    entity_id: Option<u64>,
    parent_entity_id: Option<u64>,
) -> Result<(), String> {
    if let (Some(stored_entity_id), Some(entity_id)) = (stored_entity_id, entity_id)
        && stored_entity_id != entity_id
    {
        return Err("a store location can not be moved to another entity".to_string());
    }

    if let (Some(entity_id), Some(parent_entity_id)) =
        (entity_id.or(stored_entity_id), parent_entity_id)
        && entity_id != parent_entity_id
    {
        return Err("the parent store location must be in the same entity".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_store_location_entity() {
        // A new store location, with or without a parent of its entity.
        assert!(check_store_location_entity(None, Some(1), None).is_ok());
        assert!(check_store_location_entity(None, Some(1), Some(1)).is_ok());
        // An update keeping its entity, given or not.
        assert!(check_store_location_entity(Some(1), Some(1), Some(1)).is_ok());
        assert!(check_store_location_entity(Some(1), None, Some(1)).is_ok());

        // Moved to another entity.
        assert_eq!(
            check_store_location_entity(Some(1), Some(2), None),
            Err("a store location can not be moved to another entity".to_string())
        );
        // A parent of another entity: new, or updated with or without entity.
        for (stored, entity) in [(None, Some(1)), (Some(1), Some(1)), (Some(1), None)] {
            assert_eq!(
                check_store_location_entity(stored, entity, Some(2)),
                Err("the parent store location must be in the same entity".to_string()),
                "{stored:?} {entity:?}"
            );
        }
    }
}
