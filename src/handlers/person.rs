use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chimitheque_types::{person::Person, requestfilter::RequestFilter};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use crate::{
    AppState, appstate::init_casbin_enforcer, errors::AppError,
    utils::get_chimitheque_person_id_from_headers,
};

pub async fn get_connected_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Person>, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let maybe_person = chimitheque_db::person::get_people(
        db_connection.deref(),
        RequestFilter {
            id: Some(chimitheque_person_id),
            ..Default::default()
        },
        chimitheque_person_id,
    );

    match maybe_person {
        Ok((person, _)) => Ok(Json(person.first().unwrap().to_owned())),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_people(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<Json<(Vec<Person>, usize)>, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_people = chimitheque_db::person::get_people(
        db_connection.deref(),
        request_filter,
        chimitheque_person_id,
    );

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
    request_filter: RequestFilter,
) -> Result<Json<Box<dyn erased_serde::Serialize>>, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_people = chimitheque_db::person::get_people(
        db_connection.deref(),
        request_filter.clone(),
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
    Path(path_params): Path<CreateUpdatePersonPathParameters>,
    Json(person): Json<Person>,
) -> Result<Json<u64>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    // Sanitize and validate the person.
    let mut person = person.clone();
    if let Err(err) = person.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    };

    // update?
    if path_params.id > 0 {
        person.person_id = Some(path_params.id);
    }

    let mayerr_person_id =
        chimitheque_db::person::create_update_person(db_connection.deref_mut(), person);

    init_casbin_enforcer(state.casbin_enforcer, state.db_connection_pool).await?;

    match mayerr_person_id {
        Ok(person_id) => Ok(Json(person_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn delete_person(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<(), AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    init_casbin_enforcer(state.casbin_enforcer, state.db_connection_pool).await?;

    match chimitheque_db::person::delete_person(db_connection.deref_mut(), id) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
