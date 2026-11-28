use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chimitheque_types::{person::Person, requestfilter::RequestFilter};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    AppState,
    authorization::{
        Requester, StoredPerson, check_person_save, drop_permissions_outside_entities,
    },
    axumrequestfilter::AxumRequestFilter,
    errors::AppError,
    utils::{get_chimitheque_person_id_from_headers, refuse_if, refuse_missing_references},
};

pub async fn get_connected_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Person>, AppError> {
    info!("get_connected_user");

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    info!("chimitheque_person_id: {}", chimitheque_person_id);

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let maybe_people = chimitheque_db::person::get_people(
        &db_connection,
        &RequestFilter {
            id: Some(chimitheque_person_id),
            ..Default::default()
        },
        chimitheque_person_id,
    );

    match maybe_people {
        Ok((people, _)) => Ok(Json(people.first().unwrap().to_owned())),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_people(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum_request_filter: AxumRequestFilter,
) -> Result<Json<(Vec<Person>, usize)>, AppError> {
    info!("get_people");

    let request_filter = axum_request_filter.0;

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_people =
        chimitheque_db::person::get_people(&db_connection, &request_filter, chimitheque_person_id);

    match mayerr_people {
        Ok(people) => Ok(Json(people)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GetPeopleOldResponse {
    rows: Vec<Person>,
    total: usize,
}

pub async fn get_people_old(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum_request_filter: AxumRequestFilter,
) -> Result<Json<Box<dyn erased_serde::Serialize>>, AppError> {
    info!("get_people_old");

    let request_filter = axum_request_filter.0;

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_people = chimitheque_db::person::get_people(
        &db_connection,
        &request_filter.clone(),
        chimitheque_person_id,
    );

    if request_filter.id.is_none() {
        match mayerr_people {
            Ok(people) => Ok(Json(Box::new(GetPeopleOldResponse {
                rows: people.0,
                total: people.1,
            }))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    } else {
        match mayerr_people {
            Ok(people) => Ok(Json(Box::new(people.0.first().unwrap().to_owned()))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUpdatePersonPathParameters {
    #[serde(default)]
    id: u64,
}

pub async fn create_update_person(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path_params): Path<CreateUpdatePersonPathParameters>,
    Json(person): Json<Person>,
) -> Result<Json<u64>, AppError> {
    info!("create_update_person: {}", person);

    // Get the chimitheque_person_id.
    let chimitheque_person_id = get_chimitheque_person_id_from_headers(&headers)?;

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    // Sanitize and validate the person.
    let mut person = person.clone();
    if let Err(err) = person.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    }

    // update?
    if path_params.id > 0 {
        person.person_id = Some(path_params.id);
    }

    refuse_if(
        chimitheque_db::person::person_email_exists(
            &db_connection,
            &person.person_email,
            person.person_id,
        ),
        "a person with this email already exists",
    )?;
    refuse_missing_references(
        &db_connection,
        &chimitheque_db::person::person_references(&person),
    )?;

    // A person removed from an entity loses its permissions.
    drop_permissions_outside_entities(&mut person);

    // Managers, permissions and memberships the requester may not change.
    let requester = get_requester(&db_connection, chimitheque_person_id)?;
    let stored = match person.person_id {
        Some(person_id) => StoredPerson {
            permissions: chimitheque_db::person::person_permissions(&db_connection, person_id)
                .map_err(|err| AppError::Database(err.to_string()))?,
            managed_entity_ids: chimitheque_db::person::managed_entity_ids(
                &db_connection,
                person_id,
            )
            .map_err(|err| AppError::Database(err.to_string()))?,
            member_entity_ids: chimitheque_db::person::member_entity_ids(&db_connection, person_id)
                .map_err(|err| AppError::Database(err.to_string()))?,
            is_admin: chimitheque_db::person::is_person_admin(&db_connection, person_id)
                .map_err(|err| AppError::Database(err.to_string()))?,
        },
        None => StoredPerson::default(),
    };
    check_person_save(&requester, &stored, &person).map_err(AppError::InputValidation)?;

    let mayerr_person_id = chimitheque_db::person::create_update_person(&mut db_connection, person);

    match mayerr_person_id {
        Ok(person_id) => Ok(Json(person_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn delete_person(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<(), AppError> {
    info!("delete_person: {}", id);

    // Products and storages of a deleted person are given to the admin
    // (person 1), so the admin can not be deleted.
    if id == 1 {
        return Err(AppError::InputValidation(
            "the Chimithèque admin can not be deleted".to_string(),
        ));
    }

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::person::delete_person(&mut db_connection, id) {
        Ok(()) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

/// Returns whether the person is an admin, and the entities they manage.
fn get_requester(db_connection: &Connection, person_id: u64) -> Result<Requester, AppError> {
    Ok(Requester {
        is_admin: chimitheque_db::person::is_person_admin(db_connection, person_id)
            .map_err(|err| AppError::Database(err.to_string()))?,
        managed_entity_ids: chimitheque_db::person::managed_entity_ids(db_connection, person_id)
            .map_err(|err| AppError::Database(err.to_string()))?,
    })
}
