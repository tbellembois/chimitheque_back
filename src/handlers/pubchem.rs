use std::ops::{Deref, DerefMut};

use axum::{
    Json,
    extract::{Query, State},
};
use chimitheque_pubchem::{pubchem::get_product_by_name, pubchem_compound::Autocomplete};
use chimitheque_pubchem::{
    pubchem::{autocomplete, get_compound_by_name},
    pubchem_compound::Record,
};
use chimitheque_types::pubchemproduct::PubchemProduct;
use http::HeaderMap;

use crate::{appstate::AppState, errors::AppError, utils::get_chimitheque_person_id_from_headers};

pub async fn pubchem_autocomplete(
    State(state): State<AppState>,
    Query(name): Query<String>,
) -> Result<Json<Autocomplete>, AppError> {
    let rate_limiter = state.rate_limiter;

    match autocomplete(rate_limiter.deref(), name.as_str()) {
        Ok(autocomplete) => Ok(Json(autocomplete)),
        Err(err) => Err(AppError::Pubchem(err.to_string())),
    }
}

pub async fn pubchem_getcompoundbyname(
    State(state): State<AppState>,
    Query(name): Query<String>,
) -> Result<Json<Record>, AppError> {
    let rate_limiter = state.rate_limiter;

    match get_compound_by_name(rate_limiter.deref(), name.as_str()) {
        Ok(record) => Ok(Json(record)),
        Err(err) => Err(AppError::Pubchem(err.to_string())),
    }
}

pub async fn pubchem_getproductbyname(
    State(state): State<AppState>,
    Query(name): Query<String>,
) -> Result<Json<Option<PubchemProduct>>, AppError> {
    let rate_limiter = state.rate_limiter;

    match get_product_by_name(rate_limiter.deref(), name.as_str()) {
        Ok(maybe_pubchemproduct) => Ok(Json(maybe_pubchemproduct)),
        Err(err) => Err(AppError::Pubchem(err.to_string())),
    }
}

pub async fn pubchem_create_update_product(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(id): Query<Option<u64>>,
    Json(pubchem_product): Json<PubchemProduct>,
) -> Result<Json<u64>, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    let mayerr_product_id = chimitheque_db::pubchemproduct::create_update_product_from_pubchem(
        db_connection.deref_mut(),
        pubchem_product,
        chimitheque_person_id,
        id,
    );

    match mayerr_product_id {
        Ok(product_id) => Ok(Json(product_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
